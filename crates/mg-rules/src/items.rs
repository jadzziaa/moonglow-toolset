//! Items: an item's value (its `Cost`), computed from baseitems.2da, the
//! item property tables (itempropdef.2da, iprp_costtable.2da and the cost
//! and subtype tables they name), iprp_spells.2da and iprp_chargecost.2da,
//! and for armor parts_chest.2da and armor.2da.
//!
//! The computation is the engine's, checked against `nwserver` for every
//! base-game item (`engine_item_cost.rs`), down to its single-precision
//! arithmetic and the way it picks the "two most expensive" spells.

use crate::GameData;

/// Item property type of Cast Spell (itempropdef.2da row).
pub const CAST_SPELL: u16 = 15;

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
    pub fn from_gff(s: &mg_gff::Struct) -> ItemProperty {
        let int = |label: &str| s.integer(label).unwrap_or(0);
        ItemProperty {
            property: int("PropertyName") as u16,
            subtype: int("Subtype") as u16,
            cost_table: int("CostTable") as u8,
            cost_value: int("CostValue") as u16,
            param1: s.integer("Param1").unwrap_or(255) as u8,
            param1_value: int("Param1Value") as u8,
            chance: s.integer("ChanceAppear").unwrap_or(100) as u8,
        }
    }
}

impl ItemValue {
    /// The fields of an item (a UTI, or an item in an inventory).
    pub fn from_gff(item: &mg_gff::Struct) -> ItemValue {
        let int = |label: &str| item.integer(label).unwrap_or(0).max(0) as u32;
        ItemValue {
            base_item: int("BaseItem"),
            stack_size: item.integer("StackSize").unwrap_or(1).max(1) as u32,
            charges: int("Charges"),
            add_cost: int("AddCost"),
            torso: int("ArmorPart_Torso"),
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
    /// rounded.
    pub fn armor_class(&self, torso: u32) -> Option<u32> {
        let ac = self.float("parts_chest", torso as usize, "ACBONUS")?;
        Some(ac.round().max(0.0) as u32)
    }

    /// Whether a base item is armor (its parts make its model).
    pub fn is_armor(&self, base_item: u32) -> bool {
        self.table("baseitems").ok().and_then(|t| t.get_int(base_item as usize, "ModelType"))
            == Some(3)
    }

    /// The feats an item needs to be equipped, any one of them: its base
    /// item's `ReqFeat0`–`5`, or for armor the proficiency its armor class
    /// takes (hardcoded: 1–3 Light, 4–5 Medium, 6 and up Heavy Armor
    /// Proficiency, feats 3, 4 and 2). Aurora checks them as an item is
    /// equipped (`docs/research/notes_creature_spells.md`).
    pub fn required_feats(&self, item: &mg_gff::Struct) -> Vec<u16> {
        let base = item.integer("BaseItem").unwrap_or(-1);
        let Ok(row) = usize::try_from(base) else { return Vec::new() };
        let table = self.table("baseitems").ok();
        let listed: Vec<u16> = (0..6)
            .filter_map(|i| table.as_ref()?.get_int(row, &format!("ReqFeat{i}")))
            .filter_map(|f| u16::try_from(f).ok())
            .collect();
        if !listed.is_empty() || !self.is_armor(row as u32) {
            return listed;
        }
        let torso = item.integer("ArmorPart_Torso").unwrap_or(0).max(0) as u32;
        match self.armor_class(torso).unwrap_or(0) {
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
            self.armor_class(item.torso)
                .and_then(|ac| self.float("armor", ac as usize, "COST"))
                .unwrap_or(0.0)
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
    /// baseitems.2da `PropColumn` names), by name.
    pub fn available_properties(&self, base_item: u32) -> Vec<PropertyType> {
        let column = self
            .table("baseitems")
            .ok()
            .and_then(|t| t.get_int(base_item as usize, "PropColumn"))
            .filter(|&c| c >= 0);
        let Some(column) = column else { return Vec::new() };
        let Ok(props) = self.table("itemprops") else { return Vec::new() };
        let mut out: Vec<PropertyType> = (0..props.len())
            .filter(|&row| props.cell(row, column as usize) == Some("1"))
            .filter_map(|row| self.property_type(row as u16))
            .collect();
        out.sort_by_key(|t| t.name.to_lowercase());
        out
    }

    /// Rows of a table with a name (its `Name` string, else its `Label`),
    /// skipping unnamed ones such as the "Random" rows.
    fn named_rows(&self, table: &str, keep: impl Fn(usize) -> bool) -> Vec<crate::Choice> {
        let Ok(t) = self.table(table) else { return Vec::new() };
        (0..t.len())
            .filter(|&row| keep(row))
            .filter_map(|row| {
                let name = t.get_int(row, "Name")?;
                let text = self
                    .string(mg_core::StrRef(name as u32))
                    .filter(|s| !s.is_empty())
                    .or_else(|| t.get(row, "Label").map(str::to_string))?;
                Some(crate::Choice { row, text })
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
    /// usable on it).
    pub fn property_subtypes(&self, t: &PropertyType, base_item: u32) -> Vec<crate::Choice> {
        let Some(table) = &t.subtypes else { return Vec::new() };
        let spells = self.table(table).ok().filter(|_| table == "iprp_spells");
        let column = self.use_column(base_item, "PotionUse", "WandUse").unwrap_or("GeneralUse");
        self.named_rows(table, |row| {
            spells.as_ref().is_none_or(|s| s.get_int(row, column).unwrap_or(1) != 0)
        })
    }

    /// The values of a cost table (Cast Spell's charges: those that apply
    /// to the base item).
    pub fn property_costs(&self, cost_table: u8, base_item: u32) -> Vec<crate::Choice> {
        if cost_table == 0 {
            return Vec::new();
        }
        let Some(table) = self.text("iprp_costtable", cost_table as usize, "Name") else {
            return Vec::new();
        };
        let table = table.to_lowercase();
        let charges = self.table(&table).ok().filter(|_| table == "iprp_chargecost");
        let column = self.use_column(base_item, "PotionCost", "WandCost");
        self.named_rows(&table, |row| match (&charges, column) {
            (Some(c), Some(col)) => c.get_int(row, col).unwrap_or(0) != 0,
            _ => true,
        })
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
        match self.text("iprp_paramtable", param_table as usize, "TableResRef") {
            Some(t) => self.named_rows(&t.to_lowercase(), |_| true),
            None => Vec::new(),
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
        let name_of = |table: Option<String>, row: usize| {
            let t = self.table(&table?.to_lowercase()).ok()?;
            string(t.get_int(row, "Name")).or_else(|| t.get(row, "Label").map(str::to_string))
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
            &b"2DA V2.0\n\nACBONUS\n0 0.00\n1 2.40\n2 6.00\n3 4.00\n"[..],
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
        assert_eq!(gd.armor_class(1), Some(2));
        assert_eq!(gd.item_cost(&armor), 10);
    }
}
