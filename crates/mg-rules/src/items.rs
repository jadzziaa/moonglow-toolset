//! Items: an item's value (its `Cost`), computed from baseitems.2da, the
//! item property tables (itempropdef.2da, iprp_costtable.2da and the cost
//! and subtype tables they name), iprp_spells.2da and iprp_chargecost.2da,
//! and for armor parts_chest.2da and armor.2da.
//!
//! The computation is the engine's, checked against `nwserver` for every
//! base-game item (`engine_item_cost.rs`), down to its single-precision
//! arithmetic and the way it picks the "two most expensive" spells.

use std::sync::Arc;

use crate::GameData;

/// Item property type of Cast Spell (itempropdef.2da row).
pub const CAST_SPELL: u16 = 15;

/// Item property types of Use Limitation (itempropdef.2da rows): by
/// alignment group (iprp_aligngrp.2da), class (classes.2da), race
/// (racialtypes.2da) and specific alignment (iprp_alignment.2da).
pub const USE_LIMITATION_ALIGNMENT_GROUP: u16 = 62;
pub const USE_LIMITATION_CLASS: u16 = 63;
pub const USE_LIMITATION_RACE: u16 = 64;
pub const USE_LIMITATION_SPECIFIC_ALIGNMENT: u16 = 65;

/// Why a creature may not wear an item ([`GameData::wear_problem`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WearProblem {
    /// It has none of these feats, one of which the item needs.
    Feats(Vec<u16>),
    /// Its classes are none of these (classes.2da rows) the item is
    /// limited to.
    Class(Vec<u16>),
    /// Its race is none of these (racialtypes.2da rows).
    Race(Vec<u16>),
    /// Its alignment is in none of these groups (iprp_aligngrp.2da rows)
    /// and none of these alignments (iprp_alignment.2da rows).
    Alignment { groups: Vec<u16>, specific: Vec<u16> },
}

/// A creature's alignment, on its two axes (0 to 100): good at 70 and
/// above, evil at 30 and below; lawful and chaotic likewise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alignment {
    pub good_evil: i64,
    pub lawful_chaotic: i64,
}

impl Alignment {
    /// A creature's (`GoodEvil`, `LawfulChaotic`; 50 where left out).
    pub fn of(creature: &mg_gff::Struct) -> Alignment {
        Alignment {
            good_evil: creature.integer("GoodEvil").unwrap_or(50),
            lawful_chaotic: creature.integer("LawfulChaotic").unwrap_or(50),
        }
    }

    /// On an axis: 0 the high end (good, lawful), 1 neutral, 2 the low
    /// (evil, chaotic).
    fn step(v: i64) -> u16 {
        match v {
            70.. => 0,
            31..=69 => 1,
            _ => 2,
        }
    }

    /// Its iprp_alignment.2da row: LG, LN, LE, NG, TN, NE, CG, CN, CE.
    pub fn specific(self) -> u16 {
        Self::step(self.lawful_chaotic) * 3 + Self::step(self.good_evil)
    }

    /// Whether it is in iprp_aligngrp.2da's group `g`: 0 all, 1 neutral
    /// (on either axis), 2 lawful, 3 chaotic, 4 good, 5 evil.
    pub fn in_group(self, g: u16) -> bool {
        let (law, good) = (Self::step(self.lawful_chaotic), Self::step(self.good_evil));
        match g {
            0 => true,
            1 => law == 1 || good == 1,
            2 => law == 0,
            3 => law == 2,
            4 => good == 0,
            5 => good == 2,
            _ => false,
        }
    }
}

/// One entry of an item's `PropertiesList`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemProperty {
    /// itempropdef.2da row (`PropertyName`).
    pub property: u16,
    /// Row of the property's subtype table (`Subtype`).
    pub subtype: u16,
    /// iprp_costtable.2da row (`CostTable`).
    pub cost_table: u8,
    /// Row of the cost table (`CostValue`).
    pub cost_value: u16,
    /// iprp_paramtable.2da row, 255 for none (`Param1`).
    pub param1: u8,
    /// Row of the parameter table (`Param1Value`).
    pub param1_value: u8,
    /// `ChanceAppear`.
    pub chance: u8,
}

/// What an item's value depends on.
#[derive(Debug, Clone, Default)]
pub struct ItemValue {
    /// baseitems.2da row (`BaseItem`).
    pub base_item: u32,
    pub stack_size: u32,
    /// Charges left (`Charges`): spells used by charges are worth that share
    /// of a full 50.
    pub charges: u32,
    /// `AddCost`, added to each item of the stack.
    pub add_cost: u32,
    /// parts_chest.2da row (`ArmorPart_Torso`): an armor's base cost is its
    /// armor class's.
    pub torso: u32,
    pub properties: Vec<ItemProperty>,
}

impl ItemProperty {
    /// A `PropertiesList` entry.
    ///
    /// A number its field cannot hold (a `PropertyName` of 70000 in a file
    /// some tool wrote) is read as the field's largest, a row no table
    /// has: not cut to the row its low bits spell, which is another
    /// property's.
    pub fn from_gff(s: &mg_gff::Struct) -> ItemProperty {
        let word = |label: &str| crate::word(s.integer(label).unwrap_or(0));
        let byte = |label: &str, or: i64| crate::byte(s.integer(label).unwrap_or(or));
        ItemProperty {
            property: word("PropertyName"),
            subtype: word("Subtype"),
            cost_table: byte("CostTable", 0),
            cost_value: word("CostValue"),
            param1: byte("Param1", 255),
            param1_value: byte("Param1Value", 0),
            chance: byte("ChanceAppear", 100),
        }
    }

    /// A `PropertiesList` entry, its fields in the order Aurora writes
    /// them.
    pub fn to_gff(&self) -> mg_gff::Struct {
        use mg_gff::Value;
        let mut s = mg_gff::Struct::new(0);
        s.set("PropertyName", Value::Word(self.property));
        s.set("Subtype", Value::Word(self.subtype));
        s.set("CostTable", Value::Byte(self.cost_table));
        s.set("CostValue", Value::Word(self.cost_value));
        s.set("Param1", Value::Byte(self.param1));
        s.set("Param1Value", Value::Byte(self.param1_value));
        s.set("ChanceAppear", Value::Byte(self.chance));
        s
    }
}

impl ItemValue {
    /// The fields of an item (a UTI, or an item in an inventory).
    pub fn from_gff(item: &mg_gff::Struct) -> ItemValue {
        let int = |label: &str| crate::row(item.integer(label).unwrap_or(0));
        ItemValue {
            base_item: int("BaseItem"),
            stack_size: crate::row(item.integer("StackSize").unwrap_or(1)).max(1),
            charges: int("Charges"),
            add_cost: int("AddCost"),
            torso: torso(item),
            properties: item
                .list("PropertiesList")
                .unwrap_or(&[])
                .iter()
                .map(ItemProperty::from_gff)
                .collect(),
        }
    }
}

/// iprp_chargecost.2da rows whose spells use charges (1 to 5 per use).
const PER_CHARGE: std::ops::RangeInclusive<u16> = 2..=6;

impl GameData {
    fn float(&self, table: &str, row: usize, column: &str) -> Option<f32> {
        self.table(table).ok()?.get_float(row, column)
    }

    fn text(&self, table: &str, row: usize, column: &str) -> Option<String> {
        self.table(table).ok()?.get(row, column).map(str::to_string)
    }

    /// An armor's base armor class: its torso's parts_chest.2da `ACBONUS`,
    /// rounded. A torso with none (a blank cell, as most tables' row 0, or
    /// a row the table lacks) is 0, clothing, as the engine has it: its
    /// weight and cost are armor.2da row 0's (`engine_armor_torso.rs`).
    pub fn armor_class(&self, torso: u32) -> u32 {
        let ac = self.float("parts_chest", torso as usize, "ACBONUS").unwrap_or(0.0);
        ac.round().max(0.0) as u32
    }

    /// Whether a base item is armor (its parts make its model).
    pub fn is_armor(&self, base_item: u32) -> bool {
        self.table("baseitems").is_ok_and(|t| is_armor_row(&t, base_item as usize))
    }

    /// The feats an item needs to be equipped, any one of them: its base
    /// item's `ReqFeat0`–`4` (the columns baseitems.2da has), or for armor the proficiency its armor class
    /// takes (hardcoded: 1–3 Light, 4–5 Medium, 6 and up Heavy Armor
    /// Proficiency, feats 3, 4 and 2). Aurora checks them as an item is
    /// equipped (`docs/research/notes_creature_spells.md`).
    pub fn required_feats(&self, item: &mg_gff::Struct) -> Vec<u16> {
        let base = item.integer("BaseItem").unwrap_or(-1);
        let Ok(row) = usize::try_from(base) else { return Vec::new() };
        let table = self.table("baseitems").ok();
        let listed: Vec<u16> = (0..5)
            .filter_map(|i| table.as_ref()?.get_int(row, &format!("ReqFeat{i}")))
            .filter_map(|f| u16::try_from(f).ok())
            .collect();
        if !listed.is_empty() || !self.is_armor(row as u32) {
            return listed;
        }
        match self.armor_class(torso(item)) {
            0 => Vec::new(),
            1..=3 => vec![3],
            4..=5 => vec![4],
            _ => vec![2],
        }
    }

    /// The feats to offer a creature (its `feats`) equipping an item: the
    /// item's [`required_feats`](Self::required_feats) when it has none of
    /// them, else nothing.
    pub fn missing_feats(&self, item: &mg_gff::Struct, feats: &[u16]) -> Vec<u16> {
        let need = self.required_feats(item);
        if need.iter().any(|f| feats.contains(f)) { Vec::new() } else { need }
    }

    /// Why `creature` (a creature's fields: `FeatList`, `ClassList`,
    /// `Race`, `GoodEvil`, `LawfulChaotic`) may not wear `item`, if it may
    /// not: the game takes such an item off, into the backpack, as it
    /// loads the creature (placed or made from its blueprint), and
    /// `ActionEquipItem` does not put it on (`engine_equipped_items.rs`).
    /// It needs one of the item's [`required_feats`](Self::required_feats)
    /// (from its own `FeatList`), and of each kind of the item's Use
    /// Limitation properties, one that it fits: one of its classes, its
    /// race, its alignment. The item's worth and the creature's level are
    /// no matter.
    pub fn wear_problem(
        &self,
        creature: &mg_gff::Struct,
        item: &mg_gff::Struct,
    ) -> Option<WearProblem> {
        let feats: Vec<u16> = creature
            .list("FeatList")
            .unwrap_or(&[])
            .iter()
            .filter_map(|f| u16::try_from(f.integer("Feat")?).ok())
            .collect();
        let missing = self.missing_feats(item, &feats);
        if !missing.is_empty() {
            return Some(WearProblem::Feats(missing));
        }
        let properties: Vec<ItemProperty> =
            item.list("PropertiesList").unwrap_or(&[]).iter().map(ItemProperty::from_gff).collect();
        let of = |kind: u16| -> Vec<u16> {
            properties.iter().filter(|p| p.property == kind).map(|p| p.subtype).collect()
        };
        let classes: Vec<u16> = creature
            .list("ClassList")
            .unwrap_or(&[])
            .iter()
            .filter_map(|c| u16::try_from(c.integer("Class")?).ok())
            .collect();
        let limit = of(USE_LIMITATION_CLASS);
        if !limit.is_empty() && !limit.iter().any(|c| classes.contains(c)) {
            return Some(WearProblem::Class(limit));
        }
        let race = creature.integer("Race").and_then(|r| u16::try_from(r).ok());
        let limit = of(USE_LIMITATION_RACE);
        if !limit.is_empty() && !race.is_some_and(|r| limit.contains(&r)) {
            return Some(WearProblem::Race(limit));
        }
        // (A group and a specific alignment are one kind: either fits.)
        let alignment = Alignment::of(creature);
        let (groups, specific) =
            (of(USE_LIMITATION_ALIGNMENT_GROUP), of(USE_LIMITATION_SPECIFIC_ALIGNMENT));
        let fits = groups.iter().any(|&g| alignment.in_group(g))
            || specific.contains(&alignment.specific());
        if (!groups.is_empty() || !specific.is_empty()) && !fits {
            return Some(WearProblem::Alignment { groups, specific });
        }
        None
    }

    /// What a [`WearProblem`] says, for a builder: "it has none of the
    /// feats: Martial Weapon Proficiency", "only Wizard may wear it"….
    pub fn wear_problem_text(&self, p: &WearProblem) -> String {
        use crate::rows::RowTable;
        let names = |table, name, label, rows: &[u16]| {
            let t = RowTable { table, name: Some(name), label: Some(label) };
            let named: Vec<String> = rows
                .iter()
                .map(|&r| self.row_name(&t, i64::from(r)).unwrap_or_else(|| format!("({r})")))
                .collect();
            named.join(", ")
        };
        match p {
            WearProblem::Feats(f) => {
                format!("it has none of the feats it needs: {}", names("feat", "FEAT", "LABEL", f))
            }
            WearProblem::Class(c) => {
                format!("it is limited to {}", names("classes", "Name", "Label", c))
            }
            WearProblem::Race(r) => {
                format!("it is limited to {}", names("racialtypes", "Name", "Label", r))
            }
            WearProblem::Alignment { groups, specific } => {
                let mut all = Vec::new();
                if !groups.is_empty() {
                    all.push(names("iprp_aligngrp", "Name", "Label", groups));
                }
                if !specific.is_empty() {
                    all.push(names("iprp_alignment", "Name", "Label", specific));
                }
                format!("it is limited to {}", all.join(", "))
            }
        }
    }

    /// A property's cost multiplier: its type's `Cost` (else its subtype's)
    /// times its cost table value's `Cost`.
    pub fn property_cost(&self, p: &ItemProperty) -> f32 {
        let row = p.property as usize;
        let base = self.float("itempropdef", row, "Cost").or_else(|| {
            let sub = self.text("itempropdef", row, "SubTypeResRef")?;
            self.float(&sub.to_lowercase(), p.subtype as usize, "Cost")
        });
        let table = self
            .table("itempropdef")
            .ok()
            .and_then(|t| t.get_int(row, "CostTableResRef"))
            .filter(|&t| t > 0)
            .and_then(|t| self.text("iprp_costtable", t as usize, "Name"));
        let value = match table {
            Some(t) => self.float(&t.to_lowercase(), p.cost_value as usize, "Cost").unwrap_or(0.0),
            None => 1.0,
        };
        base.unwrap_or(0.0) * value
    }

    /// A Cast Spell property's cost: the spell's times its charge cost's,
    /// and for spells used by charges, the share of 50 charges left.
    fn spell_cost(&self, p: &ItemProperty, charges: u32) -> f32 {
        let spell = self.float("iprp_spells", p.subtype as usize, "Cost").unwrap_or(0.0);
        let per = self.float("iprp_chargecost", p.cost_value as usize, "Cost").unwrap_or(0.0);
        let cost = spell * per;
        if charges > 0 && PER_CHARGE.contains(&p.cost_value) {
            cost * charges as f32 / 50.0
        } else {
            cost
        }
    }

    /// The item's value, as the engine computes it (and stores in `Cost`):
    ///
    /// - base cost: baseitems.2da `BaseCost` (armor: armor.2da `COST` of
    ///   its armor class);
    /// - passive properties: their costs summed, squared, times 1000;
    /// - spells: half of each, plus half of the most expensive and a quarter
    ///   of the second (as the engine tracks them: a new most expensive
    ///   does not make the old one second);
    /// - (base + spells, and the passive part, each truncated) times
    ///   `ItemMultiplier`, truncated, plus `AddCost`, times the stack.
    ///
    /// The engine's gold value then shows plot and creature items as 0 and
    /// other items as at least 1; this is the value before that.
    pub fn item_cost(&self, item: &ItemValue) -> u32 {
        let Ok(base_items) = self.table("baseitems") else { return 0 };
        let row = item.base_item as usize;
        let base = if self.is_armor(item.base_item) {
            self.float("armor", self.armor_class(item.torso) as usize, "COST").unwrap_or(0.0)
        } else {
            base_items.get_float(row, "BaseCost").unwrap_or(0.0)
        };
        let multiplier = base_items.get_float(row, "ItemMultiplier").unwrap_or(1.0);

        let (mut positive, mut negative) = (0.0f32, 0.0f32);
        let (mut half, mut first, mut second) = (0.0f32, 0.0f32, 0.0f32);
        for p in &item.properties {
            if p.property == CAST_SPELL {
                let cost = self.spell_cost(p, item.charges);
                half += cost / 2.0;
                if cost > first {
                    first = cost;
                } else if cost > second {
                    second = cost;
                }
            } else {
                let cost = self.property_cost(p);
                if cost >= 0.0 {
                    positive += cost;
                } else {
                    negative -= cost;
                }
            }
        }
        let spells = half + first / 2.0 + second / 4.0;
        let plus = (base + spells) as u32 as i64 + (positive * positive * 1000.0) as u32 as i64;
        let minus = (negative * negative * 1000.0) as u32 as i64;
        let unit = ((plus - minus).max(0) as f32 * multiplier) as u32;
        unit.saturating_add(item.add_cost).saturating_mul(item.stack_size.max(1))
    }
}

/// The EE twin of a model or body part field: `x` and the label, cut to
/// 16 characters (`xArmorPart_LBice`), a WORD for part numbers past 255.
pub fn wide_label(label: &str) -> String {
    format!("x{label}").chars().take(16).collect()
}

/// A model or body part number: the EE twin's where there is one, else the
/// BYTE field's.
pub fn part_number(s: &mg_gff::Struct, label: &str) -> Option<i64> {
    s.integer(&wide_label(label)).or_else(|| s.integer(label))
}

/// Whether row `row` of baseitems.2da is armor: its `ModelType` is 3, as
/// the game reads the cell. The one reading of it
/// ([`GameData::is_armor`], an appearance's parts, an item's icon).
pub fn is_armor_row(baseitems: &mg_2da::TwoDa, row: usize) -> bool {
    baseitems.get_int(row, "ModelType") == Some(3)
}

/// An armor's torso, its parts_chest.2da row: [`part_number`]'s, so one
/// past 255 (the BYTE field holds 255, its EE twin the number) is the
/// row the game draws and prices, not row 255.
pub fn torso(item: &mg_gff::Struct) -> u32 {
    crate::row(part_number(item, "ArmorPart_Torso").unwrap_or(0))
}

/// Armor icon parts, bottom first: (field, icon part).
pub const ARMOR_ICON: [(&str, &str); 6] = [
    ("ArmorPart_Pelvis", "pelvis"),
    ("ArmorPart_Torso", "chest"),
    ("ArmorPart_Belt", "belt"),
    ("ArmorPart_LShoul", "shol"),
    ("ArmorPart_RShoul", "shor"),
    ("ArmorPart_Robe", "robe"),
];

/// The names of an item's inventory icon layers, bottom first (nwn.wiki,
/// baseitems.2da): `i<ItemClass>_<model>` for simple and layered items
/// (`_m_` between for gender-specific ones), a composite weapon's bottom,
/// middle and top (`_b_`, `_m_`, `_t_`), an armor's pelvis, chest, belt,
/// shoulders and robe (`ipm_<part><number>`). Where `female`, the icons a
/// woman's inventory shows (`ipf_`, `_f_`), for the items that have both.
/// What the window draws an icon with, and what a hak's usage counts.
pub fn icon_names(game: &GameData, item: &mg_gff::Struct, female: bool) -> Vec<String> {
    let part = |label: &str| part_number(item, label).unwrap_or(0);
    let g = if female { 'f' } else { 'm' };
    let Ok(t) = game.table("baseitems") else { return Vec::new() };
    let Ok(row) = usize::try_from(item.integer("BaseItem").unwrap_or(-1)) else {
        return Vec::new();
    };
    let class = t.get(row, "ItemClass").unwrap_or_default().to_ascii_lowercase();
    match t.get_int(row, "ModelType").unwrap_or(0) {
        2 => [("ModelPart1", "b"), ("ModelPart2", "m"), ("ModelPart3", "t")]
            .iter()
            .map(|(l, p)| format!("i{class}_{p}_{:03}", part(l)))
            .collect(),
        3 => ARMOR_ICON
            .iter()
            .map(|(l, p)| (part(l), p))
            .filter(|(n, _)| *n > 0)
            .map(|(n, p)| format!("ip{g}_{p}{n:03}"))
            .collect(),
        _ => {
            let gender = if t.get_int(row, "GenderSpecific") == Some(1) {
                format!("{g}_")
            } else {
                String::new()
            };
            vec![format!("i{class}_{gender}{:03}", part("ModelPart1"))]
        }
    }
}

/// An armor's parts, in the game's order (`ITEM_APPR_ARMOR_MODEL_*`): the
/// field with each one's part number, and its name.
pub const ARMOR_PARTS: [(&str, &str); 19] = [
    ("ArmorPart_RFoot", "Right Foot"),
    ("ArmorPart_LFoot", "Left Foot"),
    ("ArmorPart_RShin", "Right Shin"),
    ("ArmorPart_LShin", "Left Shin"),
    ("ArmorPart_LThigh", "Left Thigh"),
    ("ArmorPart_RThigh", "Right Thigh"),
    ("ArmorPart_Pelvis", "Pelvis"),
    ("ArmorPart_Torso", "Torso"),
    ("ArmorPart_Belt", "Belt"),
    ("ArmorPart_Neck", "Neck"),
    ("ArmorPart_RFArm", "Right Forearm"),
    ("ArmorPart_LFArm", "Left Forearm"),
    ("ArmorPart_RBicep", "Right Bicep"),
    ("ArmorPart_LBicep", "Left Bicep"),
    ("ArmorPart_RShoul", "Right Shoulder"),
    ("ArmorPart_LShoul", "Left Shoulder"),
    ("ArmorPart_RHand", "Right Hand"),
    ("ArmorPart_LHand", "Left Hand"),
    ("ArmorPart_Robe", "Robe"),
];

/// An armor's color channels, numbered as the game numbers them
/// (`ITEM_APPR_ARMOR_COLOR_*`): not the order of a PLT's layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArmorChannel {
    Leather1 = 0,
    Leather2 = 1,
    Cloth1 = 2,
    Cloth2 = 3,
    Metal1 = 4,
    Metal2 = 5,
}

impl ArmorChannel {
    pub const ALL: [ArmorChannel; 6] = [
        ArmorChannel::Cloth1,
        ArmorChannel::Cloth2,
        ArmorChannel::Leather1,
        ArmorChannel::Leather2,
        ArmorChannel::Metal1,
        ArmorChannel::Metal2,
    ];

    /// The item's field with this channel's color for the whole armor.
    pub fn field(self) -> &'static str {
        match self {
            ArmorChannel::Leather1 => "Leather1Color",
            ArmorChannel::Leather2 => "Leather2Color",
            ArmorChannel::Cloth1 => "Cloth1Color",
            ArmorChannel::Cloth2 => "Cloth2Color",
            ArmorChannel::Metal1 => "Metal1Color",
            ArmorChannel::Metal2 => "Metal2Color",
        }
    }

    /// The layer of a PLT texture this channel colors (0 and 1 are skin
    /// and hair, 8 and 9 the tattoos).
    pub fn plt_layer(self) -> usize {
        match self {
            ArmorChannel::Metal1 => 2,
            ArmorChannel::Metal2 => 3,
            ArmorChannel::Cloth1 => 4,
            ArmorChannel::Cloth2 => 5,
            ArmorChannel::Leather1 => 6,
            ArmorChannel::Leather2 => 7,
        }
    }

    /// The palette (a texture's name) the channel's colors are chosen
    /// from.
    pub fn palette(self) -> &'static str {
        match self {
            ArmorChannel::Cloth1 | ArmorChannel::Cloth2 => "pal_cloth01",
            ArmorChannel::Leather1 | ArmorChannel::Leather2 => "pal_leath01",
            // (Both metals: the game never reads `pal_armor02`,
            // `mg_image::plt::PALETTES`.)
            ArmorChannel::Metal1 | ArmorChannel::Metal2 => "pal_armor01",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ArmorChannel::Leather1 => "Leather 1",
            ArmorChannel::Leather2 => "Leather 2",
            ArmorChannel::Cloth1 => "Cloth 1",
            ArmorChannel::Cloth2 => "Cloth 2",
            ArmorChannel::Metal1 => "Metal 1",
            ArmorChannel::Metal2 => "Metal 2",
        }
    }
}

/// The field with armor part `part`'s own color of a channel (EE; a BYTE,
/// as the engine writes it: `engine_armor_colors.rs`).
pub fn armor_part_color_label(part: usize, channel: ArmorChannel) -> String {
    format!("APart_{part}_Col_{}", channel as u8)
}

/// Armor part `part`'s own color of a channel (`part`: an index of
/// [`ARMOR_PARTS`]); `None` where it takes the armor's (no field, or 255).
pub fn armor_part_color(item: &mg_gff::Struct, part: usize, channel: ArmorChannel) -> Option<u8> {
    let v = item.integer(&armor_part_color_label(part, channel))?;
    u8::try_from(v).ok().filter(|&v| v != 255)
}

/// Gives armor part `part` its own color of a channel, or with `None`
/// takes it away (the field goes: the engine writes none for such a part).
pub fn set_armor_part_color(
    item: &mut mg_gff::Struct,
    part: usize,
    channel: ArmorChannel,
    color: Option<u8>,
) {
    let label = armor_part_color_label(part, channel);
    match color.filter(|&c| c != 255) {
        Some(c) => item.set(&label, mg_gff::Value::Byte(c)),
        None => {
            item.remove(&label);
        }
    }
}

/// An item property type (an itempropdef.2da row) and the tables that
/// qualify it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyType {
    pub row: u16,
    /// The toolset's name (`Name`).
    pub name: String,
    /// The table of subtypes (`SubTypeResRef`), lower case.
    pub subtypes: Option<String>,
    /// The iprp_costtable.2da row of its values (`CostTableResRef`; 0 is
    /// the empty table).
    pub cost_table: u8,
    /// The iprp_paramtable.2da row of its parameter (`Param1ResRef`); a
    /// subtype table may give one per subtype instead.
    pub param1: Option<u8>,
}

/// Which of a table's named rows a list holds, and in what order: what a
/// list kept is kept by, with its table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct NamedRows {
    /// Only the rows this column has another number than 0 in, a blank
    /// cell read as the number beside it.
    only: Option<(&'static str, i32)>,
    /// By name ([`crate::by_name`]), not in the table's order.
    by_name: bool,
}

/// A list kept of a table's named rows: which of them, and the list.
pub(crate) type NamedList = (NamedRows, Arc<Vec<crate::Choice>>);

impl GameData {
    /// An item property type.
    pub fn property_type(&self, row: u16) -> Option<PropertyType> {
        let t = self.table("itempropdef").ok()?;
        let r = row as usize;
        let name = t
            .get_int(r, "Name")
            .and_then(|s| self.string(mg_core::StrRef(s as u32)))
            .filter(|s| !s.is_empty())
            .or_else(|| t.get(r, "Label").map(str::to_string))?;
        Some(PropertyType {
            row,
            name,
            subtypes: t.get(r, "SubTypeResRef").map(str::to_lowercase),
            cost_table: t.get_int(r, "CostTableResRef").map_or(0, |v| v.clamp(0, 255) as u8),
            param1: t.get_int(r, "Param1ResRef").map(|v| v.clamp(0, 255) as u8),
        })
    }

    /// The property types a base item can have (itemprops.2da, in the column
    /// baseitems.2da `PropColumn` names), by name. Made once for a column,
    /// until the tables change ([`invalidate`](Self::invalidate)).
    pub fn available_properties(&self, base_item: u32) -> Vec<PropertyType> {
        let column = self
            .table("baseitems")
            .ok()
            .and_then(|t| t.get_int(base_item as usize, "PropColumn"))
            .filter(|&c| c >= 0);
        let Some(column) = column else { return Vec::new() };
        let cache = &self.property_types;
        if let Some(made) = cache.read().expect("property type cache poisoned").get(&column) {
            return made.to_vec();
        }
        let Ok(props) = self.table("itemprops") else { return Vec::new() };
        let mut out: Vec<PropertyType> = (0..props.len())
            .filter(|&row| props.cell(row, column as usize) == Some("1"))
            .filter_map(|row| self.property_type(row as u16))
            .collect();
        out.sort_by_key(|t| t.name.to_lowercase());
        let made = Arc::new(out);
        cache.write().expect("property type cache poisoned").insert(column, made.clone());
        made.to_vec()
    }

    /// Rows of a table with a name (its `Name` string, else its `Label`),
    /// skipping unnamed ones such as the "Random" rows: those `rows`
    /// keeps, in the table's order or by name. Made once, until the tables
    /// or the talk table change ([`invalidate`](Self::invalidate),
    /// [`set_custom_tlk`](Self::set_custom_tlk)): the Properties page asks
    /// each frame, for every kind of property, and a hak's table of
    /// twenty thousand feats was named and sorted each time.
    fn named_rows(&self, table: &str, rows: NamedRows) -> Arc<Vec<crate::Choice>> {
        let cache = &self.named;
        let kept = |lists: &Vec<NamedList>| {
            lists.iter().find(|(r, _)| *r == rows).map(|(_, list)| list.clone())
        };
        if let Some(made) = cache.read().expect("named cache poisoned").get(table).and_then(kept) {
            return made;
        }
        let made = Arc::new(self.name_rows(table, rows));
        let mut cache = cache.write().expect("named cache poisoned");
        cache.entry(table.to_string()).or_default().push((rows, made.clone()));
        made
    }

    fn name_rows(&self, table: &str, rows: NamedRows) -> Vec<crate::Choice> {
        // (By name: the list in the table's order, kept too, sorted.)
        if rows.by_name {
            let listed = self.named_rows(table, NamedRows { by_name: false, ..rows });
            return crate::by_name(listed.to_vec());
        }
        let Ok(t) = self.table(table) else { return Vec::new() };
        let keep = |row: usize| match rows.only {
            Some((column, unset)) => t.get_int(row, column).unwrap_or(unset) != 0,
            None => true,
        };
        let (name, label) = (t.column("Name"), t.column("Label"));
        (0..t.len())
            .filter(|&row| keep(row))
            .filter_map(|row| {
                // (A row without a `Name` number is no choice: the
                // "Random" rows. The others by the one naming of a row.)
                t.get_int(row, "Name")?;
                Some(crate::Choice { row, text: self.named_row(&t, row, name, None, label)? })
            })
            .collect()
    }

    /// The use column (iprp_spells.2da) or cost column (iprp_chargecost.2da)
    /// that applies to a base item: potions and wands have their own.
    fn use_column(
        &self,
        base_item: u32,
        potion: &'static str,
        wand: &'static str,
    ) -> Option<&'static str> {
        let column = self.table("baseitems").ok()?.get_int(base_item as usize, "PropColumn")?;
        match column {
            8 => Some(potion),
            10 => Some(wand),
            _ => None,
        }
    }

    /// A property type's subtypes for a base item (Cast Spell: the spells
    /// usable on it), in their table's order.
    pub fn property_subtypes(&self, t: &PropertyType, base_item: u32) -> Vec<crate::Choice> {
        self.property_subtype_list(t, base_item).to_vec()
    }

    /// [`property_subtypes`](Self::property_subtypes), as the list kept:
    /// for what asks each frame (no copy of it is made).
    pub fn property_subtype_list(
        &self,
        t: &PropertyType,
        base_item: u32,
    ) -> Arc<Vec<crate::Choice>> {
        self.subtype_rows(t, base_item, false)
    }

    /// [`property_subtypes`](Self::property_subtypes) by name
    /// ([`crate::by_name`]), sorted once and kept.
    pub fn property_subtypes_by_name(
        &self,
        t: &PropertyType,
        base_item: u32,
    ) -> Arc<Vec<crate::Choice>> {
        self.subtype_rows(t, base_item, true)
    }

    fn subtype_rows(
        &self,
        t: &PropertyType,
        base_item: u32,
        by_name: bool,
    ) -> Arc<Vec<crate::Choice>> {
        let Some(table) = &t.subtypes else { return Arc::default() };
        // (A spell is usable unless its column says 0.)
        let only = (table == "iprp_spells").then(|| {
            let column = self.use_column(base_item, "PotionUse", "WandUse").unwrap_or("GeneralUse");
            (column, 1)
        });
        self.named_rows(table, NamedRows { only, by_name })
    }

    /// The values of a cost table (Cast Spell's charges: those that apply
    /// to the base item).
    pub fn property_costs(&self, cost_table: u8, base_item: u32) -> Vec<crate::Choice> {
        self.property_cost_list(cost_table, base_item).to_vec()
    }

    /// [`property_costs`](Self::property_costs), as the list kept.
    pub fn property_cost_list(&self, cost_table: u8, base_item: u32) -> Arc<Vec<crate::Choice>> {
        if cost_table == 0 {
            return Arc::default();
        }
        let Some(table) = self.text("iprp_costtable", cost_table as usize, "Name") else {
            return Arc::default();
        };
        let table = table.to_lowercase();
        // (A charge cost applies where a potion's or a wand's column has one.)
        let column = self.use_column(base_item, "PotionCost", "WandCost");
        let only = column.filter(|_| table == "iprp_chargecost").map(|c| (c, 0));
        self.named_rows(&table, NamedRows { only, by_name: false })
    }

    /// The parameter table (iprp_paramtable.2da row) of a property with a
    /// subtype: the type's, else the subtype's `Param1ResRef`.
    pub fn property_param_table(&self, t: &PropertyType, subtype: u16) -> Option<u8> {
        t.param1.or_else(|| {
            let table = t.subtypes.as_ref()?;
            let v = self.table(table).ok()?.get_int(subtype as usize, "Param1ResRef")?;
            u8::try_from(v).ok()
        })
    }

    /// A parameter table's values.
    pub fn property_params(&self, param_table: u8) -> Vec<crate::Choice> {
        self.property_param_list(param_table).to_vec()
    }

    /// [`property_params`](Self::property_params), as the list kept.
    pub fn property_param_list(&self, param_table: u8) -> Arc<Vec<crate::Choice>> {
        match self.text("iprp_paramtable", param_table as usize, "TableResRef") {
            Some(t) => self.named_rows(&t.to_lowercase(), NamedRows::default()),
            None => Arc::default(),
        }
    }

    /// A new property of a type, for a base item: its first subtype, value
    /// and parameter.
    pub fn new_property(&self, t: &PropertyType, base_item: u32) -> ItemProperty {
        let subtype = self.property_subtypes(t, base_item).first().map_or(0, |c| c.row as u16);
        let cost_value =
            self.property_costs(t.cost_table, base_item).first().map_or(0, |c| c.row as u16);
        let param1 = self.property_param_table(t, subtype);
        let param1_value =
            param1.and_then(|p| self.property_params(p).first().map(|c| c.row as u8)).unwrap_or(0);
        ItemProperty {
            property: t.row,
            subtype,
            cost_table: t.cost_table,
            cost_value,
            param1: param1.unwrap_or(255),
            param1_value,
            chance: 100,
        }
    }

    /// A property as the game describes it: its game name (`GameStrRef`, as
    /// "Enhancement Bonus:"), then its subtype, value and parameter.
    pub fn property_text(&self, p: &ItemProperty) -> String {
        let Ok(defs) = self.table("itempropdef") else { return format!("({})", p.property) };
        let r = p.property as usize;
        let string = |s: Option<i32>| {
            s.and_then(|s| self.string(mg_core::StrRef(s as u32))).filter(|s| !s.is_empty())
        };
        let mut parts = vec![
            string(defs.get_int(r, "GameStrRef"))
                .or_else(|| string(defs.get_int(r, "Name")))
                .unwrap_or_else(|| format!("({})", p.property)),
        ];
        // (A row's name as the lists of them have it: `named_row`.)
        let name_of = |table: Option<String>, row: usize| {
            let t = self.table(&table?.to_lowercase()).ok()?;
            self.named_row(&t, row, t.column("Name"), None, t.column("Label"))
        };
        let Some(t) = self.property_type(p.property) else { return parts.remove(0) };
        if t.subtypes.is_some() {
            parts.extend(name_of(t.subtypes, p.subtype as usize));
        }
        if p.cost_table != 0 {
            let table = self.text("iprp_costtable", p.cost_table as usize, "Name");
            parts.extend(name_of(table, p.cost_value as usize));
        }
        if p.param1 != 255 {
            let table = self.text("iprp_paramtable", p.param1 as usize, "TableResRef");
            parts.extend(name_of(table, p.param1_value as usize));
        }
        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use mg_core::{Language, ResType};
    use mg_resman::{LayerClass, MemContainer, ResKey, ResMan, priority};
    use mg_tlk::Tlk;

    use super::*;

    /// Metal 2 is chosen from Metal 1's palette, as the game draws it and
    /// Aurora shows it (Troubletcat's report: the picker showed
    /// `pal_armor02`, which the game never reads).
    #[test]
    fn both_metals_are_in_one_palette() {
        assert_eq!(ArmorChannel::Metal2.palette(), ArmorChannel::Metal1.palette());
        assert_eq!(ArmorChannel::Metal2.palette(), "pal_armor01");
    }

    fn data() -> GameData {
        let tables: [(&str, &str); 8] = [
            (
                "baseitems",
                "label BaseCost ItemMultiplier ModelType ReqFeat0 ReqFeat1\n\
                 0 sword 10 2 2 45 256\n1 misc 0 1 0 **** ****\n\
                 2 arrow 1 0.01 0 **** ****\n3 armor **** 1 3 **** ****\n",
            ),
            (
                "itempropdef",
                "Label SubTypeResRef Cost CostTableResRef\n\
                 0 Ability iprp_abilities 1.2 1\n1 Feat iprp_feats **** 0\n\
                 2 Bad **** -0.5 0\n",
            ),
            ("iprp_abilities", "Label\n0 Str\n"),
            ("iprp_feats", "Label Cost\n0 Alertness 0.75\n"),
            ("iprp_costtable", "Name Label\n0 **** Base\n1 iprp_bonuscost Bonus\n"),
            ("iprp_bonuscost", "Label Cost\n0 zero 0\n1 one 1\n2 two 2\n3 three 3\n"),
            ("iprp_spells", "Label Cost\n0 Small 1000\n1 Big 4000\n2 Mid 2000\n"),
            ("iprp_chargecost", "Label Cost\n0 Random 0\n1 Single 0.5\n2 5_Charges 0.25\n"),
        ];
        let mut mem = MemContainer::new();
        for (name, body) in tables {
            let text = format!("2DA V2.0\n\n{body}");
            mem.insert(ResKey::parse(name, ResType::TWODA).unwrap(), text.as_bytes());
        }
        mem.insert(
            ResKey::parse("parts_chest", ResType::TWODA).unwrap(),
            &b"2DA V2.0\n\nACBONUS\n0 0.00\n1 2.40\n2 6.00\n3 4.00\n4 ****\n"[..],
        );
        mem.insert(
            ResKey::parse("armor", ResType::TWODA).unwrap(),
            &b"2DA V2.0\n\nCOST\n0 1\n1 5\n2 10\n"[..],
        );
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "mem", LayerClass::Key, mem);
        GameData::new(rm, Tlk::new(Language::ENGLISH))
    }

    #[test]
    fn equipping_needs_a_listed_feat_or_armor_proficiency() {
        let gd = data();
        let item = |base: i32, torso: u8| {
            let mut s = mg_gff::Struct::new(0);
            s.set("BaseItem", mg_gff::Value::Int(base));
            s.set("ArmorPart_Torso", mg_gff::Value::Byte(torso));
            s
        };
        // Any one of the base item's feats will do.
        assert_eq!(gd.missing_feats(&item(0, 0), &[]), [45, 256]);
        assert_eq!(gd.missing_feats(&item(0, 0), &[256]), [0u16; 0]);
        assert_eq!(gd.required_feats(&item(1, 0)), [0u16; 0]);
        // Armor by its armor class: clothing, light, heavy, medium.
        let armor: Vec<Vec<u16>> = (0..4).map(|t| gd.required_feats(&item(3, t))).collect();
        assert_eq!(armor, [vec![], vec![3], vec![2], vec![4]]);
        assert_eq!(gd.missing_feats(&item(3, 2), &[3, 4]), [2]);
        assert!(gd.required_feats(&mg_gff::Struct::new(0)).is_empty());
    }

    /// The property lists are made once and kept: the same list each time
    /// it is asked for (no rows named, nothing sorted again), the same
    /// rows as before, and made anew once the tables may have changed. A
    /// table of 25,000 subtypes, asked for by name a thousand times (a
    /// thousand frames of the Properties page), is sorted once.
    #[test]
    fn property_lists_are_made_once_and_kept() {
        let mut feats = String::from("2DA V2.0\n\nName Label\n");
        for row in 0..25_000 {
            feats += &format!("{row} {} Feat{:05}\n", 16_777_216 + row, (row * 7919) % 25_000);
        }
        let tables: [(&str, &str); 8] = [
            ("baseitems", "2DA V2.0\n\nlabel PropColumn\n0 sword 0\n1 potion 8\n"),
            (
                "itemprops",
                "2DA V2.0\n\nc0 c1 c2 c3 c4 c5 c6 c7 c8\n0 1 0 0 0 0 0 0 0 0\n\
                 1 1 0 0 0 0 0 0 0 1\n2 1 0 0 0 0 0 0 0 1\n",
            ),
            (
                "itempropdef",
                "2DA V2.0\n\nName Label SubTypeResRef CostTableResRef Param1ResRef\n\
                 0 **** Zeta iprp_feats 1 0\n1 **** Cast iprp_spells 2 ****\n2 **** Alpha **** 0 ****\n",
            ),
            ("iprp_feats", &feats),
            (
                "iprp_spells",
                "2DA V2.0\n\nName Label GeneralUse PotionUse\n0 900 Zap 1 0\n1 901 Heal 1 1\n\
                 2 902 Bless **** 1\n3 903 Doom 0 0\n",
            ),
            ("iprp_costtable", "2DA V2.0\n\nName\n0 ****\n1 iprp_bonuscost\n2 iprp_chargecost\n"),
            (
                "iprp_chargecost",
                "2DA V2.0\n\nName Label PotionCost\n0 910 Single 1\n1 911 Five 0\n2 912 One ****\n",
            ),
            ("iprp_paramtable", "2DA V2.0\n\nTableResRef\n0 iprp_spells\n"),
        ];
        let mut mem = MemContainer::new();
        for (name, body) in tables {
            mem.insert(ResKey::parse(name, ResType::TWODA).unwrap(), body.as_bytes());
        }
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "mem", LayerClass::Key, mem);
        let gd = GameData::new(rm, Tlk::new(Language::ENGLISH));
        // (No talk table line: each row by its label, as the lists fall
        // back to.) The kinds by name; a potion's are its column's.
        let names = |base| -> Vec<String> {
            gd.available_properties(base).into_iter().map(|t| t.name).collect()
        };
        assert_eq!(names(0), ["Alpha", "Cast", "Zeta"]);
        assert_eq!(names(1), ["Alpha", "Cast"]);
        let kinds = gd.available_properties(0);
        let (cast, zeta) = (&kinds[1], &kinds[2]);
        let texts = |list: &[crate::Choice]| -> Vec<String> {
            list.iter().map(|c| c.text.clone()).collect()
        };
        // A spell is one unless its column says 0; a potion has its own.
        assert_eq!(texts(&gd.property_subtypes(cast, 0)), ["Zap", "Heal", "Bless"]);
        assert_eq!(texts(&gd.property_subtype_list(cast, 1)), ["Heal", "Bless"]);
        assert_eq!(texts(&gd.property_subtypes_by_name(cast, 0)), ["Bless", "Heal", "Zap"]);
        // A charge cost applies to a potion where its column has one.
        assert_eq!(texts(&gd.property_costs(2, 0)), ["Single", "Five", "One"]);
        assert_eq!(texts(&gd.property_cost_list(2, 1)), ["Single"]);
        assert_eq!(texts(&gd.property_params(0)), ["Zap", "Heal", "Bless", "Doom"]);
        assert!(gd.property_cost_list(0, 0).is_empty() && gd.property_param_list(9).is_empty());
        // The long list: by name, the same list every time, at no cost.
        let sorted = gd.property_subtypes_by_name(zeta, 0);
        assert_eq!(sorted.len(), 25_000);
        assert_eq!(
            (sorted[0].text.as_str(), sorted[24_999].text.as_str()),
            ("Feat00000", "Feat24999")
        );
        assert_eq!(*sorted, crate::by_name(gd.property_subtypes(zeta, 0)));
        let started = std::time::Instant::now();
        for _ in 0..1000 {
            assert!(Arc::ptr_eq(&gd.property_subtypes_by_name(zeta, 0), &sorted));
            assert!(Arc::ptr_eq(
                &gd.property_subtype_list(cast, 1),
                &gd.property_subtype_list(cast, 1)
            ));
            assert_eq!(gd.available_properties(0).len(), 3);
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(2), "{:?}", started.elapsed());
        // The tables may have changed: made anew, the same rows.
        gd.invalidate();
        let again = gd.property_subtypes_by_name(zeta, 0);
        assert!(!Arc::ptr_eq(&again, &sorted) && again == sorted);
    }

    fn prop(property: u16, subtype: u16, cost_value: u16) -> ItemProperty {
        ItemProperty {
            property,
            subtype,
            cost_value,
            param1: 255,
            chance: 100,
            ..Default::default()
        }
    }

    /// A property's numbers that their fields cannot hold are not cut to
    /// other rows: `PropertyName` 70000 is not property 4464, and an
    /// item's base item past a `u32` is not row 0.
    #[test]
    fn numbers_a_field_cannot_hold_are_not_other_rows() {
        use mg_gff::Value;
        let mut p = mg_gff::Struct::new(0);
        p.set("PropertyName", Value::Int(70000));
        p.set("Subtype", Value::Int(-1));
        p.set("CostTable", Value::Int(257));
        p.set("CostValue", Value::Int64(1 << 40));
        p.set("Param1Value", Value::Int(300));
        let read = ItemProperty::from_gff(&p);
        assert_eq!((read.property, read.subtype), (u16::MAX, u16::MAX));
        assert_eq!((read.cost_table, read.cost_value, read.param1_value), (255, u16::MAX, 255));
        assert_eq!((read.param1, read.chance), (255, 100), "left out: none, and always");
        // (What fits is as it is.)
        let fits = ItemProperty { property: 15, subtype: 300, cost_value: 7, ..read };
        assert_eq!(ItemProperty::from_gff(&fits.to_gff()), fits);
        let mut item = mg_gff::Struct::new(0);
        item.set("BaseItem", Value::Int64(1 << 32));
        item.set("StackSize", Value::Int64(-5));
        item.set("Charges", Value::Int64((1 << 32) + 3));
        let read = ItemValue::from_gff(&item);
        assert_eq!((read.base_item, read.stack_size, read.charges), (u32::MAX, 1, u32::MAX));
    }

    #[test]
    fn costs_follow_the_engine() {
        let gd = data();
        let sword = |properties| ItemValue {
            base_item: 0,
            stack_size: 1,
            properties,
            ..Default::default()
        };
        // Base cost times the multiplier.
        assert_eq!(gd.item_cost(&sword(vec![])), 20);
        // Str +3: 1.2 × 3 = 3.6; 3.6² × 1000 = 12960; (10 + 12960) × 2.
        assert_eq!(gd.item_cost(&sword(vec![prop(0, 0, 3)])), 25940);
        // A feat's cost comes from its subtype table.
        assert_eq!(gd.item_cost(&sword(vec![prop(1, 0, 0)])), 2 * (10 + 562));
        // Negative costs subtract.
        assert_eq!(gd.item_cost(&sword(vec![prop(0, 0, 1), prop(2, 0, 0)])), 2 * (10 + 1440 - 250));
        // Spells: half of each, half of the "most expensive" and a quarter
        // of the "second", as the engine tracks them: Mid (1000), Big (2000),
        // Small (500) leaves Big first and Small second (Mid, once first, is
        // dropped).
        let misc = |properties, charges| ItemValue {
            base_item: 1,
            stack_size: 1,
            charges,
            properties,
            ..Default::default()
        };
        let spells = vec![prop(CAST_SPELL, 2, 1), prop(CAST_SPELL, 1, 1), prop(CAST_SPELL, 0, 1)];
        assert_eq!(gd.item_cost(&misc(spells, 0)), 1750 + 1000 + 125);
        // Spells used by charges are worth the share of 50 charges left;
        // single-use ones are not.
        assert_eq!(gd.item_cost(&misc(vec![prop(CAST_SPELL, 1, 2)], 10)), 200);
        assert_eq!(gd.item_cost(&misc(vec![prop(CAST_SPELL, 1, 1)], 10)), 2000);
        // Stacks: each item's value truncated, plus the additional cost.
        let arrows = ItemValue { base_item: 2, stack_size: 99, add_cost: 2, ..Default::default() };
        assert_eq!(gd.item_cost(&arrows), 2 * 99);
        // Armor: the cost of its armor class (2.4 rounds to 2).
        let armor = ItemValue { base_item: 3, stack_size: 1, torso: 1, ..Default::default() };
        assert_eq!(gd.armor_class(1), 2);
        assert_eq!(gd.item_cost(&armor), 10);
        // A torso with no ACBONUS (blank, or past the table) is clothing,
        // armor class 0, as the engine prices it.
        assert_eq!(gd.armor_class(4), 0);
        assert_eq!(gd.armor_class(250), 0);
        // A torso past 255 is in the EE twin, the BYTE field holding 255:
        // the twin's row is the armor's (row 255's is another's), for its
        // cost and for the proficiency it takes.
        let mut wide = mg_gff::Struct::new(0);
        wide.set("BaseItem", mg_gff::Value::Int(3));
        wide.set("ArmorPart_Torso", mg_gff::Value::Byte(255));
        assert_eq!(ItemValue::from_gff(&wide).torso, 255);
        assert_eq!(gd.item_cost(&ItemValue::from_gff(&wide)), 1);
        assert!(gd.required_feats(&wide).is_empty());
        wide.set("xArmorPart_Torso", mg_gff::Value::Word(1));
        assert_eq!(torso(&wide), 1);
        assert_eq!(ItemValue::from_gff(&wide).torso, 1);
        assert_eq!(gd.item_cost(&ItemValue::from_gff(&wide)), 10);
        assert_eq!(gd.required_feats(&wide), [3], "light armor");
        let clothing = ItemValue { torso: 4, ..armor.clone() };
        assert_eq!(gd.item_cost(&clothing), 1);
    }
}
