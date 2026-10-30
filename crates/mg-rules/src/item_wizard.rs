//! The Item Wizard's magical items: Aurora's Item Wizard, with Magical
//! ticked, gives the new item random properties for an item level
//! (1-5, 6-10, 11-15, 16-20) and quality (Low, Medium, High, Godly).
//!
//! They come from the toolset's own tables, `itmwiz*.2da` (one per kind
//! of base item, by its baseitems.2da `PropColumn`): rows of a property,
//! its subtype, value and parameter, and `Value`, a tier. As Aurora's
//! made items show (`docs/findings.md`, "Item Wizard's random
//! properties"):
//!
//! - level band `L` and quality `Q` make a step `3L + Q` (0 to 12); the
//!   first property is of the tier that step reaches in the table,
//!   `round(step × top tier / 12)`;
//! - the next ones are one or two tiers lower (Low: two), each kept only
//!   if the item's value stays within the most an item of that level is
//!   worth (itemvalue.2da `MAXSINGLEITEMVALUE`): Low at most two
//!   properties, the others as many as fit;
//! - potions, scrolls, wands and trap kits get one; magic staves three
//!   spells, of the tier and one below; rods up to three;
//! - staves, rods and wands have 50 charges; scrolls and wands may be
//!   used by bards, wizards and sorcerers only, staves by wizards and
//!   sorcerers (Use Limitation: Class);
//! - a magical item is written unidentified.
//!
//! Unlike Aurora, a property the base item cannot have (itemprops.2da)
//! or with a subtype, value or parameter its tables don't hold is never
//! chosen: Aurora's ranged table gives Vampiric Regeneration, which
//! itemprops.2da doesn't allow on ranged weapons.

use std::collections::HashMap;

use crate::GameData;
use crate::items::{ItemProperty, ItemValue};

/// The item level bands, as the wizard names them.
pub const LEVELS: [&str; 4] = ["1-5", "6-10", "11-15", "16-20"];

/// The qualities, as the wizard names them.
pub const QUALITIES: [&str; 4] = ["Low", "Medium", "High", "Godly"];

/// Use Limitation: Class (itempropdef.2da row).
const USE_LIMITATION_CLASS: u16 = 63;

/// The charges of staves, rods and wands.
pub const CHARGES: u8 = 50;

/// How a table's properties are given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// One property (potions, scrolls, wands, trap kits).
    Single,
    /// Three spells, the second of the first one's tier (magic staves).
    Staff,
    /// Up to three spells (magic rods).
    Rod,
    /// As many as fit, Low two at most.
    Other,
}

/// The wizard's table for a baseitems.2da `PropColumn`, and how it is
/// used; `None` where Aurora offers no magic (or offers it and gives
/// nothing: gems and gauntlets).
fn table(column: i32) -> Option<(&'static str, Kind)> {
    Some(match column {
        0 => ("itmwizmelee", Kind::Other),
        1 => ("itmwizranged", Kind::Other),
        2 => ("itmwizthrow", Kind::Other),
        3 => ("itmwizstaves", Kind::Staff),
        4 => ("itmwizrods", Kind::Rod),
        5 => ("itmwizammo", Kind::Other),
        // Armor and shields; amulets, belts, boots, bracers, cloaks and
        // rings too (the properties Aurora gives them are this table's).
        6 | 16 => ("itmwizarmor", Kind::Other),
        7 => ("itmwizhelmet", Kind::Other),
        8 => ("itmwizpotion", Kind::Single),
        9 => ("itmwizscroll", Kind::Single),
        10 => ("itmwizwands", Kind::Single),
        12 => ("itmwiztrap", Kind::Single),
        _ => return None,
    })
}

/// The `PropColumn`s Aurora offers Magical for and gives no properties:
/// gems (17) and gauntlets (21), made unidentified.
const MAGICAL_WITHOUT_PROPERTIES: [i32; 2] = [17, 21];

/// Whether the wizard offers to make a base item magical: it has a table
/// of properties, or is a gem or gauntlets (unidentified, with none), as
/// Aurora offers it.
pub fn offered(game: &GameData, base_item: u32) -> bool {
    let column =
        game.table("baseitems").ok().and_then(|t| t.get_int(base_item as usize, "PropColumn"));
    column.is_some_and(|c| table(c).is_some() || MAGICAL_WITHOUT_PROPERTIES.contains(&c))
}

/// The wizard's Magical choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Magical {
    /// The item level band: 0 (1-5) to 3 (16-20).
    pub level: u8,
    /// 0 (Low) to 3 (Godly).
    pub quality: u8,
    /// What the random properties are chosen with.
    pub seed: u64,
}

/// What the wizard gives an item.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Magic {
    /// In the order they were chosen (the first, the highest, first).
    pub properties: Vec<ItemProperty>,
    /// `Charges` (50 for staves, rods and wands).
    pub charges: u8,
}

/// The rows of a base item's table it can have, with their tiers.
#[derive(Debug, Clone)]
pub struct ItemMagic {
    kind: Kind,
    /// The itmwiz table.
    table: &'static str,
    /// Each row's property and tier.
    rows: Vec<(ItemProperty, i32)>,
    top: i32,
    /// The classes that may use it, as Use Limitation: Class rows.
    classes: Vec<u16>,
    charges: u8,
}

impl ItemMagic {
    /// A base item's random properties, if the wizard makes it magical.
    pub fn new(game: &GameData, base_item: u32) -> Option<ItemMagic> {
        let column = game.table("baseitems").ok()?.get_int(base_item as usize, "PropColumn")?;
        let (name, kind) = table(column)?;
        let t = game.table(name).ok()?;
        let valid = Validity::new(game, base_item);
        let mut rows = Vec::new();
        let mut top = None;
        for r in 0..t.len() {
            let (Some(property), Some(tier)) = (t.get_int(r, "Prop"), t.get_int(r, "Value")) else {
                continue;
            };
            top = top.max(Some(tier));
            let p = ItemProperty {
                property: property.clamp(0, 65535) as u16,
                subtype: t.get_int(r, "SubProp").map_or(65535, |v| v.clamp(0, 65535) as u16),
                cost_table: t.get_int(r, "CostTable").unwrap_or(0).clamp(0, 255) as u8,
                cost_value: t.get_int(r, "CostVal").unwrap_or(0).clamp(0, 65535) as u16,
                param1: t.get_int(r, "ParamTable").map_or(255, |v| v.clamp(0, 255) as u8),
                param1_value: t.get_int(r, "Param").unwrap_or(0).clamp(0, 255) as u8,
                chance: 100,
            };
            if tier >= 0 && valid.allows(&p) {
                rows.push((p, tier));
            }
        }
        let top = top?;
        if rows.is_empty() {
            return None;
        }
        let classes: &[u16] = match column {
            9 | 10 => &[1, 10, 9],
            3 => &[10, 9],
            _ => &[],
        };
        let classes = classes.iter().copied().filter(|&c| valid.allows(&class_limit(c))).collect();
        let charges = if matches!(column, 3 | 4 | 10) { CHARGES } else { 0 };
        Some(ItemMagic { kind, table: name, rows, top, classes, charges })
    }

    /// The tier the first property is of: the step `3 × level + quality`
    /// of 12, as far up the table's tiers.
    pub fn tier(&self, level: u8, quality: u8) -> i32 {
        let step = 3 * i32::from(level.min(3)) + i32::from(quality.min(3));
        (step * self.top * 2 + 12) / 24
    }

    /// The tiers the properties after the first are of: one or two
    /// below it (Low: two); a staff's spells its own and one below.
    pub fn later_tiers(&self, level: u8, quality: u8) -> Vec<i32> {
        let first = self.tier(level, quality);
        let below: &[i32] = match (self.kind, quality) {
            (Kind::Single, _) => &[],
            (Kind::Staff, _) => &[0, 1],
            (Kind::Other, 0) => &[2],
            _ => &[1, 2],
        };
        // (A staff's and a rod's spells are of the lowest tier at least.)
        let clamp = matches!(self.kind, Kind::Staff | Kind::Rod);
        below
            .iter()
            .map(|d| if clamp { (first - d).max(0) } else { first - d })
            .filter(|t| *t >= 0)
            .collect()
    }

    /// The most properties it gets, the classes it is limited to aside.
    pub fn most(&self, quality: u8) -> usize {
        match (self.kind, quality) {
            (Kind::Single, _) => 1,
            (Kind::Staff | Kind::Rod, _) => 3,
            (Kind::Other, 0) => 2,
            (Kind::Other, _) => 4,
        }
    }

    /// The classes it is limited to (classes.2da rows).
    pub fn classes(&self) -> &[u16] {
        &self.classes
    }

    /// The table its properties are rows of (`itmwizmelee`...).
    pub fn table(&self) -> &'static str {
        self.table
    }

    /// Its charges.
    pub fn charges(&self) -> u8 {
        self.charges
    }

    /// Properties for an item (`item`: its base item, armor's torso and
    /// the rest its value depends on) of a level band (0: 1-5 ... 3:
    /// 16-20) and quality (0: Low ... 3: Godly), chosen with `seed`.
    pub fn generate(
        &self,
        game: &GameData,
        item: &ItemValue,
        level: u8,
        quality: u8,
        seed: u64,
    ) -> Magic {
        let mut rng = fastrand::Rng::with_seed(seed);
        let (level, quality) = (level.min(3), quality.min(3));
        let cap = most_worth(game, level, quality);
        let charges = self.charges;
        let mut v =
            ItemValue { charges: u32::from(charges), properties: Vec::new(), ..item.clone() };
        let fits = |v: &mut ItemValue, p: ItemProperty| {
            v.properties.push(p);
            let ok = cap.is_none_or(|cap| game.item_cost(v) <= cap);
            v.properties.pop();
            ok
        };
        // The first, whatever it is worth (Aurora's Low scrolls of 6-10
        // are worth more than a Low item of the band may be); of the
        // nearest tier that has rows, lower ones first.
        let first = self.tier(level, quality);
        let at = |tier: i32| -> Vec<ItemProperty> {
            self.rows.iter().filter(|r| r.1 == tier).map(|r| r.0).collect()
        };
        let Some(rows) =
            (0..=first).rev().chain(first + 1..=self.top).map(at).find(|r| !r.is_empty())
        else {
            return Magic::default();
        };
        v.properties.push(rows[rng.usize(..rows.len())]);
        let (most, tiers) = (self.most(quality), self.later_tiers(level, quality));
        while v.properties.len() < most && !tiers.is_empty() {
            let mut rows: Vec<ItemProperty> = self
                .rows
                .iter()
                .filter(|(p, t)| tiers.contains(t) && !v.properties.iter().any(|q| same(q, p)))
                .map(|r| r.0)
                .collect();
            rng.shuffle(&mut rows);
            match rows.into_iter().find(|p| fits(&mut v, *p)) {
                Some(p) => v.properties.push(p),
                None => break,
            }
        }
        v.properties.extend(self.classes.iter().map(|&c| class_limit(c)));
        Magic { properties: v.properties, charges }
    }
}

/// Whether two properties are of one type and subtype (an item has one).
fn same(a: &ItemProperty, b: &ItemProperty) -> bool {
    (a.property, a.subtype) == (b.property, b.subtype)
}

/// Use Limitation: Class of a class (classes.2da row).
fn class_limit(class: u16) -> ItemProperty {
    ItemProperty {
        property: USE_LIMITATION_CLASS,
        subtype: class,
        cost_table: 0,
        cost_value: 0,
        param1: 255,
        param1_value: 0,
        chance: 100,
    }
}

/// The most an item of a level band and quality may be worth: the
/// itemvalue.2da `MAXSINGLEITEMVALUE` of a character level, Medium to
/// Godly the third to fifth of the band, Low lower (levels 1, 5, 8 and
/// 13, as Aurora's Low items stay within).
pub fn most_worth(game: &GameData, level: u8, quality: u8) -> Option<u32> {
    let (level, quality) = (usize::from(level.min(3)), usize::from(quality.min(3)));
    let character = match quality {
        0 => [1, 5, 8, 13][level],
        q => 5 * level + q + 2,
    };
    let t = game.table("itemvalue").ok()?;
    u32::try_from(t.get_int(character - 1, "MAXSINGLEITEMVALUE")?).ok()
}

/// What a base item can have: its properties (itemprops.2da), and each
/// type's subtypes, values and parameters.
struct Validity<'a> {
    game: &'a GameData,
    base_item: u32,
    types: HashMap<u16, crate::items::PropertyType>,
    /// By property type: its subtypes, values; by parameter table: its
    /// values.
    subtypes: std::cell::RefCell<HashMap<u16, Vec<usize>>>,
    costs: std::cell::RefCell<HashMap<u8, Vec<usize>>>,
    params: std::cell::RefCell<HashMap<u8, Vec<usize>>>,
}

impl<'a> Validity<'a> {
    fn new(game: &'a GameData, base_item: u32) -> Validity<'a> {
        let types = game.available_properties(base_item).into_iter().map(|t| (t.row, t)).collect();
        Validity {
            game,
            base_item,
            types,
            subtypes: Default::default(),
            costs: Default::default(),
            params: Default::default(),
        }
    }

    /// Whether the base item can have a property, with its parts.
    fn allows(&self, p: &ItemProperty) -> bool {
        let Some(t) = self.types.get(&p.property) else { return false };
        let rows = |c: Vec<crate::Choice>| c.into_iter().map(|c| c.row).collect::<Vec<_>>();
        if t.subtypes.is_some() {
            let mut cache = self.subtypes.borrow_mut();
            let subtypes = cache
                .entry(p.property)
                .or_insert_with(|| rows(self.game.property_subtypes(t, self.base_item)));
            if !subtypes.contains(&usize::from(p.subtype)) {
                return false;
            }
        }
        if p.cost_table != t.cost_table {
            return false;
        }
        if t.cost_table != 0 {
            let mut cache = self.costs.borrow_mut();
            let costs = cache
                .entry(t.cost_table)
                .or_insert_with(|| rows(self.game.property_costs(t.cost_table, self.base_item)));
            if !costs.contains(&usize::from(p.cost_value)) {
                return false;
            }
        }
        let param =
            self.game.property_param_table(t, if t.subtypes.is_some() { p.subtype } else { 0 });
        match param {
            Some(table) => {
                let mut cache = self.params.borrow_mut();
                let values =
                    cache.entry(table).or_insert_with(|| rows(self.game.property_params(table)));
                p.param1 == table && values.contains(&usize::from(p.param1_value))
            }
            None => p.param1 == 255,
        }
    }
}

#[cfg(test)]
mod tests {
    use mg_core::{Language, ResType};
    use mg_resman::{LayerClass, MemContainer, ResKey, ResMan, priority};
    use mg_tlk::Tlk;

    use super::*;

    /// A sword (PropColumn 0), a potion (8) and a torch (20); an
    /// enhancement bonus of +1 to +5 and Light, which swords can have,
    /// and Haste, which they can't.
    fn data() -> GameData {
        let tables: [(&str, &str); 8] = [
            (
                "baseitems",
                "label BaseCost ItemMultiplier ModelType PropColumn\n\
                 0 sword 10 1 2 0\n1 potion 50 1 0 8\n2 torch 1 1 0 20\n",
            ),
            (
                "itemprops",
                "0_Melee 1_Ranged 2_Thrown 3_Staves 4_Rods 5_Ammo 6_Arm_Shld 7_Helm \
                 8_Potions 9_Scrolls 10_Wands 11_Thieves 12_TrapKits 13_Hide 14_Claw \
                 15_Misc_Uneq 16_Misc 17_No_Props 18_Containers 19_HealerKit 20_Torch 21_Glove \
                 StringRef Label\n\
                 0 1 **** **** **** **** **** **** **** **** **** **** **** **** **** **** **** \
                 **** **** **** **** **** **** 1 Enhancement\n\
                 1 1 **** **** **** **** **** **** **** **** **** **** **** **** **** **** **** \
                 **** **** **** **** **** **** 2 Light\n\
                 2 **** **** **** **** **** **** **** **** 1 **** **** **** **** **** **** **** \
                 **** **** **** **** **** **** 3 Haste\n",
            ),
            (
                "itempropdef",
                "Name Label SubTypeResRef Cost CostTableResRef Param1ResRef\n\
                 0 1 Enhancement **** 0 1 ****\n1 2 Light **** 0 0 ****\n\
                 2 3 Haste **** 2 0 ****\n",
            ),
            ("iprp_costtable", "Name Label\n0 **** Base\n1 iprp_bonuscost Bonus\n"),
            (
                "iprp_bonuscost",
                "Name Label Cost\n0 **** zero 0\n1 1 one 1\n2 1 two 2\n3 1 three 3\n4 1 four 4\n\
                 5 1 five 5\n",
            ),
            (
                "itmwizmelee",
                "Prop SubProp CostTable CostVal ParamTable Param Value\n\
                 0 1 **** 0 **** **** **** 0\n1 0 **** 1 1 **** **** 0\n\
                 2 0 **** 1 2 **** **** 1\n3 2 **** 0 **** **** **** 1\n\
                 4 0 **** 1 3 **** **** 2\n5 0 **** 1 5 **** **** 3\n\
                 6 1 **** 0 **** **** **** 1\n",
            ),
            (
                "itmwizpotion",
                "Prop SubProp CostTable CostVal ParamTable Param Value\n0 2 **** 0 **** **** **** 0\n",
            ),
            (
                "itemvalue",
                "LABEL MAXSINGLEITEMVALUE\n0 1 1000\n1 2 1500\n2 3 2500\n3 4 3500\n4 5 5000\n",
            ),
        ];
        let mut mem = MemContainer::new();
        for (name, body) in tables {
            let text = format!("2DA V2.0\n\n{body}");
            mem.insert(ResKey::parse(name, ResType::TWODA).unwrap(), text.as_bytes());
        }
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "mem", LayerClass::Key, mem);
        let mut tlk = Tlk::new(Language::ENGLISH);
        for s in ["Bad Strref", "Enhancement Bonus", "Light", "Haste", "+1"] {
            tlk.entries.push(mg_tlk::TlkEntry::text(s));
        }
        GameData::new(rm, tlk)
    }

    #[test]
    fn properties_by_level_and_quality_from_the_base_item_s_table() {
        let game = data();
        let sword = ItemMagic::new(&game, 0).expect("swords are made magical");
        // Haste, which swords can't have, is never among them.
        assert!(sword.rows.iter().all(|(p, _)| p.property != 2));
        assert_eq!(sword.top, 3);
        // The first property's tier: the step 3L + Q of 12, scaled.
        assert_eq!(sword.tier(0, 0), 0);
        assert_eq!(sword.tier(0, 3), 1, "3 of 12 of 3 tiers: 0.75");
        assert_eq!(sword.tier(1, 1), 1);
        assert_eq!(sword.tier(3, 3), 3);
        let item = ItemValue { base_item: 0, stack_size: 1, ..Default::default() };
        // 1-5, Godly: an item worth at most 5000, the first property of
        // tier 1 (+2 or Light), then what is one or two tiers lower and
        // fits, never the same property twice.
        for seed in 0..20 {
            let m = sword.generate(&game, &item, 0, 3, seed);
            let first = (m.properties[0].property, m.properties[0].cost_value);
            assert!(matches!(first, (0, 2) | (1, 0)), "{m:?}");
            assert_eq!(m.properties.len(), 2, "{m:?}");
            assert_ne!(m.properties[0].property, m.properties[1].property);
            let v = ItemValue { properties: m.properties.clone(), ..item.clone() };
            assert!(game.item_cost(&v) <= 5000, "{m:?}");
            assert_eq!(m.charges, 0);
        }
        // The same seed, the same properties; others differ.
        let one = sword.generate(&game, &item, 0, 3, 7);
        assert_eq!(one, sword.generate(&game, &item, 0, 3, 7));
        let all: std::collections::HashSet<Vec<u16>> = (0..20)
            .map(|s| {
                let m = sword.generate(&game, &item, 0, 3, s);
                m.properties.iter().map(|p| p.property * 10 + p.cost_value).collect()
            })
            .collect();
        assert!(all.len() > 1, "{all:?}");
        // Low: one property where nothing is two tiers lower.
        assert_eq!(sword.generate(&game, &item, 0, 0, 1).properties.len(), 1);
        // A potion gets one; a torch none.
        let potion = ItemMagic::new(&game, 1).unwrap();
        let p = potion.generate(&game, &ItemValue { base_item: 1, ..item.clone() }, 3, 3, 1);
        assert_eq!(p.properties.len(), 1);
        assert_eq!(p.properties[0].property, 2);
        assert!(ItemMagic::new(&game, 2).is_none());
        // The most an item is worth, by level band and quality.
        assert_eq!(most_worth(&game, 0, 0), Some(1000));
        assert_eq!(most_worth(&game, 0, 1), Some(2500));
        assert_eq!(most_worth(&game, 0, 3), Some(5000));
        assert_eq!(most_worth(&game, 3, 3), None, "a short table");
    }
}
