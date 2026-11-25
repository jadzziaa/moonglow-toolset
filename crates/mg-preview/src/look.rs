//! A creature's looks, and its worn and held items', as plain values: what
//! a preview reads of a creature blueprint (UTC) and its equipped items
//! (UTI), so that a caller with no blueprint (a game client, which is told
//! a creature's looks by the server) gives the same and gets the same
//! picture. A blueprint is read into these values ([`DressedLook::from_utc`])
//! and the preview drawn from them alone ([`crate::dressed_look`]): the
//! fields a preview reads are read here, and nowhere else.

use glam::Mat4;
use mg_core::ResRef;
use mg_gff::{Gff, Struct, Value};
use mg_rules::GameData;
use mg_rules::items::{ARMOR_PARTS, ArmorChannel, armor_part_color, part_number, wide_label};
use mg_rules::models::BODY_PARTS;

/// A visual transform's goals (`VisTransformList`'s entry for a scope:
/// each axis's `ValueTo`): drawn as [`crate::visual_matrix`] has it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisualTransform {
    /// `ScaleX`, the model scaled so every way (the client reads no other).
    pub scale: f32,
    /// `RotateX`, `RotateY`, `RotateZ`, degrees.
    pub rotate: [f32; 3],
    /// `TranslateX`, `TranslateY`, `TranslateZ`, metres.
    pub translate: [f32; 3],
}

impl Default for VisualTransform {
    fn default() -> VisualTransform {
        VisualTransform { scale: 1.0, rotate: [0.0; 3], translate: [0.0; 3] }
    }
}

impl VisualTransform {
    /// The entry for scope `scope` of `object`'s `VisTransformList` (an
    /// axis it lacks at rest); `None` without one.
    pub fn of(object: &Struct, scope: i64) -> Option<VisualTransform> {
        let list = object.list("VisTransformList")?;
        let entry = list.iter().find(|e| e.integer("Scope").unwrap_or(0) == scope)?;
        let axis = |label: &str, rest: f32| {
            entry.child(label).and_then(|a| a.float("ValueTo")).unwrap_or(rest)
        };
        let three =
            |prefix: &str, rest: f32| ["X", "Y", "Z"].map(|a| axis(&format!("{prefix}{a}"), rest));
        Some(VisualTransform {
            scale: axis("ScaleX", 1.0),
            rotate: three("Rotate", 0.0),
            translate: three("Translate", 0.0),
        })
    }

    /// As a matrix ([`crate::visual_matrix`]); `None` where it changes
    /// nothing.
    pub fn matrix(&self) -> Option<Mat4> {
        (*self != VisualTransform::default())
            .then(|| crate::visual_matrix(self.scale, self.rotate, self.translate))
    }

    /// The `VisTransformList` entry for `scope` that [`VisualTransform::of`]
    /// reads back as this (each axis's `ValueTo`).
    pub fn to_entry(&self, scope: i64) -> Struct {
        let mut entry = Struct::new(6);
        entry.set("Scope", Value::Int(scope as i32));
        let mut axis = |label: String, v: f32| {
            let mut a = Struct::new(0);
            a.set("ValueTo", Value::Float(v));
            entry.set(&label, Value::Struct(a));
        };
        axis("ScaleX".into(), self.scale);
        for (i, a) in ["X", "Y", "Z"].into_iter().enumerate() {
            axis(format!("Rotate{a}"), self.rotate[i]);
            axis(format!("Translate{a}"), self.translate[i]);
        }
        entry
    }
}

/// `VisTransformList` holding `entries` (scope, transform) on `object`,
/// where there are any.
fn set_transforms(object: &mut Struct, entries: &[(i64, Option<VisualTransform>)]) {
    let list: Vec<Struct> =
        entries.iter().filter_map(|(scope, t)| t.map(|t| t.to_entry(*scope))).collect();
    if !list.is_empty() {
        object.set("VisTransformList", Value::List(list));
    }
}

/// A part or model number: [`part_number`] (the EE twin's where there is
/// one), 0 where there is none or it is below 0 (which is none to the
/// preview too).
fn part(s: &Struct, label: &str) -> u16 {
    part_number(s, label).unwrap_or(0).clamp(0, i64::from(u16::MAX)) as u16
}

/// A part or model number written as the game writes it: the BYTE field,
/// and past 255 its EE twin, a WORD, with the BYTE at 255.
fn set_part(s: &mut Struct, label: &str, n: u16) {
    s.set(label, Value::Byte(n.min(255) as u8));
    if n > 255 {
        s.set(&wide_label(label), Value::Word(n));
    }
}

/// A table row from a field: `None`-like rows (below 0, or past what the
/// type holds) as the type's largest, which no table has, so that the
/// preview still finds no row there.
fn row<T: TryFrom<i64> + Copy>(v: Option<i64>, max: T) -> T {
    T::try_from(v.unwrap_or(0)).unwrap_or(max)
}

/// The scopes of a composite item's parts a preview transforms: the whole
/// item (255), its middle (254) and top (253), as [`ItemLook::scopes`].
pub const ITEM_SCOPES: [i64; 3] = [255, 254, 253];

/// An item as worn or held, by what a preview reads of it: its base item
/// and model parts, its colors, an armor's parts and each part's own
/// colors, and its parts' visual transforms.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ItemLook {
    /// baseitems.2da row (`BaseItem`): its kind of model, whether it is a
    /// shield, its default model.
    pub base_item: u32,
    /// `ModelPart1`, `ModelPart2`, `ModelPart3` (the EE twins' where there
    /// are): a simple or layered item's model (a helmet's, a cloak's
    /// cloakmodel.2da row), a composite one's bottom, middle and top.
    pub model_parts: [u16; 3],
    /// Its colors by channel, in the game's order of them
    /// ([`ArmorChannel`]: leather 1 and 2, cloth 1 and 2, metal 1 and 2);
    /// `None` takes the wearer's (no field).
    pub colors: [Option<u8>; 6],
    /// An armor's parts, in [`ARMOR_PARTS`]' order (the robe last); 0 is
    /// none.
    pub armor_parts: [u16; 19],
    /// Each armor part's own colors (EE), by part as `armor_parts` and by
    /// channel as `colors`; `None` takes the armor's.
    pub part_colors: [[Option<u8>; 6]; 19],
    /// The visual transforms of its parts' scopes ([`ITEM_SCOPES`]: the
    /// whole item, its middle, its top); a composite item's alone are drawn.
    pub scopes: [Option<VisualTransform>; 3],
    /// Its Light property (the first, where it has more), which lights
    /// what is around it while it is held ([`crate::item_light`]).
    pub light: Option<LightProperty>,
}

/// An item's Light property (itempropdef.2da row 44,
/// `ITEM_PROPERTY_LIGHT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightProperty {
    /// iprp_lightcost.2da row (`CostValue`): 1 dim (5 m) to 4 bright
    /// (20 m).
    pub brightness: u16,
    /// iprp_color.2da row (`Param1Value`): 0 blue, 1 yellow, 2 purple,
    /// 3 red, 4 green, 5 orange, 6 white.
    pub color: u8,
}

impl LightProperty {
    /// itempropdef.2da's row of the Light property.
    pub const PROPERTY: u16 = 44;

    /// The property as an item's `PropertiesList` entry has it, as Aurora
    /// writes one (its cost table iprp_lightcost, 18; its parameter table
    /// iprp_color, 9).
    fn to_gff(self) -> Struct {
        mg_rules::items::ItemProperty {
            property: LightProperty::PROPERTY,
            subtype: 0,
            cost_table: 18,
            cost_value: self.brightness,
            param1: 9,
            param1_value: self.color,
            chance: 100,
        }
        .to_gff()
    }
}

/// The first Light property of an item's `PropertiesList`.
fn light_property(uti: &Struct) -> Option<LightProperty> {
    let entries = uti.list("PropertiesList")?.iter();
    let p = entries
        .map(mg_rules::items::ItemProperty::from_gff)
        .find(|p| p.property == LightProperty::PROPERTY)?;
    Some(LightProperty { brightness: p.cost_value, color: p.param1_value })
}

impl ItemLook {
    /// What a preview reads of an item's fields (UTI, or an item struct of
    /// a creature's `Equip_ItemList`).
    pub fn from_uti(uti: &Struct) -> ItemLook {
        let mut look = ItemLook {
            base_item: row(uti.integer("BaseItem"), u32::MAX),
            model_parts: ["ModelPart1", "ModelPart2", "ModelPart3"].map(|f| part(uti, f)),
            ..ItemLook::default()
        };
        for channel in ArmorChannel::ALL {
            let c = channel as usize;
            look.colors[c] = uti.integer(channel.field()).map(|v| v.clamp(0, 255) as u8);
            for (i, _) in ARMOR_PARTS.iter().enumerate() {
                look.part_colors[i][c] = armor_part_color(uti, i, channel);
            }
        }
        for (i, (field, _)) in ARMOR_PARTS.iter().enumerate() {
            look.armor_parts[i] = part(uti, field);
        }
        look.scopes = ITEM_SCOPES.map(|s| VisualTransform::of(uti, s));
        look.light = light_property(uti);
        look
    }

    /// The item's fields that [`ItemLook::from_uti`] reads back as this, in
    /// a struct of id `id` (its slot's, in a creature's `Equip_ItemList`).
    pub fn to_uti(&self, id: u32) -> Struct {
        let mut uti = Struct::new(id);
        uti.set("BaseItem", Value::Int(i32::try_from(self.base_item).unwrap_or(i32::MAX)));
        for (field, n) in
            ["ModelPart1", "ModelPart2", "ModelPart3"].into_iter().zip(self.model_parts)
        {
            if n > 0 {
                set_part(&mut uti, field, n);
            }
        }
        for channel in ArmorChannel::ALL {
            let c = channel as usize;
            if let Some(v) = self.colors[c] {
                uti.set(channel.field(), Value::Byte(v));
            }
            for (i, _) in ARMOR_PARTS.iter().enumerate() {
                mg_rules::items::set_armor_part_color(&mut uti, i, channel, self.part_colors[i][c]);
            }
        }
        for ((field, _), n) in ARMOR_PARTS.iter().zip(self.armor_parts) {
            if n > 0 {
                set_part(&mut uti, field, n);
            }
        }
        let entries: Vec<_> = ITEM_SCOPES.into_iter().zip(self.scopes).collect();
        set_transforms(&mut uti, &entries);
        if let Some(light) = self.light {
            uti.set("PropertiesList", Value::List(vec![light.to_gff()]));
        }
        uti
    }

    /// Armor part `field`'s number ([`ARMOR_PARTS`]' field name); 0 for a
    /// name that is not one.
    pub fn armor_part(&self, field: &str) -> u16 {
        ARMOR_PARTS.iter().position(|(f, _)| *f == field).map_or(0, |i| self.armor_parts[i])
    }

    /// Its colors (PLT layers) over `colors`, the wearer's.
    pub(crate) fn colors_over(&self, mut colors: [u8; 10]) -> [u8; 10] {
        for channel in ArmorChannel::ALL {
            if let Some(v) = self.colors[channel as usize] {
                colors[channel.plt_layer()] = v;
            }
        }
        colors
    }

    /// Armor part `part`'s colors (an index of [`ARMOR_PARTS`]) over
    /// `colors`, the armor's.
    pub(crate) fn part_colors_over(&self, part: usize, mut colors: [u8; 10]) -> [u8; 10] {
        for channel in ArmorChannel::ALL {
            if let Some(c) = self.part_colors[part][channel as usize] {
                colors[channel.plt_layer()] = c;
            }
        }
        colors
    }
}

/// The equipment slots a preview draws (`Equip_ItemList` struct ids, the
/// game's `INVENTORY_SLOT_*` bits).
pub mod slot {
    pub const HEAD: u32 = 0x1;
    pub const CHEST: u32 = 0x2;
    pub const RIGHT_HAND: u32 = 0x10;
    pub const LEFT_HAND: u32 = 0x20;
    pub const CLOAK: u32 = 0x40;
    /// The left ring's slot (not drawn): where [`super::DressedLook::to_utc`]
    /// writes a light of the undrawn slots, as a ring (baseitems.2da row 52).
    pub const LEFT_RING: u32 = 0x80;
    pub(crate) const RING_BASE_ITEM: i32 = 52;
    /// All of them, in the order [`super::DressedLook::to_utc`] writes them.
    pub const DRAWN: [u32; 5] = [HEAD, CHEST, RIGHT_HAND, LEFT_HAND, CLOAK];
}

/// The visual-transform scopes of a creature's parts
/// (`OBJECT_VISUAL_TRANSFORM_DATA_SCOPE_CREATURE_*`).
pub(crate) const HEAD_SCOPE: i64 = 254;
pub(crate) const TAIL_SCOPE: i64 = 253;
pub(crate) const WINGS_SCOPE: i64 = 252;
pub(crate) const CLOAK_SCOPE: i64 = 243;

/// A creature's looks as values, its worn and held items included:
/// everything a preview reads of a creature's fields (UTC, or a creature
/// placed in an area) and of the items in the slots it draws. A creature
/// with no blueprint (one a game client is told of) is drawn by giving
/// these ([`crate::dressed_look`]); a blueprint is read into them
/// ([`DressedLook::from_utc`]) and drawn the same way, so that the two
/// cannot differ.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DressedLook {
    /// appearance.2da row (`Appearance_Type`).
    pub appearance: u16,
    /// gender.2da row (`Gender`: 0 male, 1 female).
    pub gender: u8,
    /// phenotype.2da row (`Phenotype`).
    pub phenotype: u32,
    /// The head's model number (`Appearance_Head`, part-based creatures);
    /// 0 is no head.
    pub head: u16,
    /// The body parts' model numbers, in [`BODY_PARTS`]' order (the
    /// engine's: right foot first, left hand last); 0 is none.
    pub body_parts: [u16; 18],
    /// PLT colors: skin, hair, tattoo 1 and 2 (`Color_Skin`,
    /// `Color_Hair`, `Color_Tattoo1`, `Color_Tattoo2`).
    pub colors: [u8; 4],
    /// wingmodel.2da and tailmodel.2da rows (`Wings_New`, `Tail_New`, or
    /// the older `Wings` and `Tail`); 0 is none.
    pub wings: u32,
    pub tail: u32,
    /// The visual transforms of its head (or helmet), tail, wings and
    /// cloak (`VisTransformList`'s scopes 254, 253, 252 and 243).
    pub head_transform: Option<VisualTransform>,
    pub tail_transform: Option<VisualTransform>,
    pub wings_transform: Option<VisualTransform>,
    pub cloak_transform: Option<VisualTransform>,
    /// What it wears and holds, by slot ([`slot`]): chest, head, cloak,
    /// right hand and left hand.
    pub armor: Option<ItemLook>,
    pub helmet: Option<ItemLook>,
    pub cloak: Option<ItemLook>,
    pub right_hand: Option<ItemLook>,
    pub left_hand: Option<ItemLook>,
    /// The brightest Light property of what it wears in the slots a
    /// preview does not draw (a ring, an amulet, boots...): it lights what
    /// is around it as a drawn item's does ([`crate::item_light`]).
    pub light: Option<LightProperty>,
}

impl DressedLook {
    /// The item in an equipment slot (an `Equip_ItemList` struct id,
    /// [`slot`]), to read or to set; `None` for a slot a preview does not
    /// draw (boots, rings...).
    pub fn slot_mut(&mut self, slot: u32) -> Option<&mut Option<ItemLook>> {
        Some(match slot {
            slot::HEAD => &mut self.helmet,
            slot::CHEST => &mut self.armor,
            slot::RIGHT_HAND => &mut self.right_hand,
            slot::LEFT_HAND => &mut self.left_hand,
            slot::CLOAK => &mut self.cloak,
            _ => return None,
        })
    }

    /// The item in an equipment slot ([`DressedLook::slot_mut`]).
    pub fn slot(&self, slot: u32) -> Option<&ItemLook> {
        match slot {
            slot::HEAD => self.helmet.as_ref(),
            slot::CHEST => self.armor.as_ref(),
            slot::RIGHT_HAND => self.right_hand.as_ref(),
            slot::LEFT_HAND => self.left_hand.as_ref(),
            slot::CLOAK => self.cloak.as_ref(),
            _ => None,
        }
    }

    /// What a preview reads of a creature's fields, everything in its
    /// drawn slots worn: a blueprint's `EquippedRes` loaded with `item`,
    /// an item struct itself (an area's creature, a character) as it is.
    pub fn from_utc(utc: &Struct, item: &dyn Fn(ResRef) -> Option<Gff>) -> DressedLook {
        DressedLook::read(utc, |slot| equipped(utc, slot, item))
    }

    /// [`DressedLook::from_utc`] without what the game takes off the
    /// creature as it loads it (an item it may not wear:
    /// [`GameData::wear_problem`], by its feats, classes, race and
    /// alignment): what the game shows of a blueprint.
    pub fn from_utc_as_the_game(
        game: &GameData,
        utc: &Struct,
        item: &dyn Fn(ResRef) -> Option<Gff>,
    ) -> DressedLook {
        DressedLook::read(utc, |slot| {
            equipped(utc, slot, item).filter(|it| game.wear_problem(utc, it).is_none())
        })
    }

    fn read(utc: &Struct, worn: impl Fn(u32) -> Option<Struct>) -> DressedLook {
        let byte = |f: &str| utc.integer(f).unwrap_or(0).clamp(0, 255) as u8;
        let mut look = DressedLook {
            appearance: row(part_number(utc, "Appearance_Type"), u16::MAX),
            gender: row(utc.integer("Gender").map(|g| g.max(0)), u8::MAX),
            phenotype: row(utc.integer("Phenotype").map(|p| p.max(0)), u32::MAX),
            head: part(utc, "Appearance_Head"),
            body_parts: BODY_PARTS.map(|(_, field, _)| part(utc, field)),
            colors: ["Color_Skin", "Color_Hair", "Color_Tattoo1", "Color_Tattoo2"].map(byte),
            wings: ["Wings_New", "Wings"]
                .iter()
                .find_map(|f| utc.integer(f))
                .unwrap_or(0)
                .clamp(0, i64::from(u32::MAX)) as u32,
            tail: ["Tail_New", "Tail"]
                .iter()
                .find_map(|f| utc.integer(f))
                .unwrap_or(0)
                .clamp(0, i64::from(u32::MAX)) as u32,
            head_transform: VisualTransform::of(utc, HEAD_SCOPE),
            tail_transform: VisualTransform::of(utc, TAIL_SCOPE),
            wings_transform: VisualTransform::of(utc, WINGS_SCOPE),
            cloak_transform: VisualTransform::of(utc, CLOAK_SCOPE),
            ..DressedLook::default()
        };
        for s in slot::DRAWN {
            let it = worn(s).map(|it| ItemLook::from_uti(&it));
            *look.slot_mut(s).expect("a drawn slot") = it;
        }
        let undrawn = utc.list("Equip_ItemList").unwrap_or(&[]).iter().map(|e| e.id);
        look.light = undrawn
            .filter(|s| !slot::DRAWN.contains(s))
            .filter_map(|s| light_property(&worn(s)?))
            .reduce(|a, b| if b.brightness > a.brightness { b } else { a });
        look
    }

    /// The Light property of all it wears that lights what is around it:
    /// the brightest (the client gives a creature one such light; which
    /// of two as bright is not read).
    pub fn brightest_light(&self) -> Option<LightProperty> {
        let drawn = slot::DRAWN.iter().filter_map(|&s| self.slot(s)?.light);
        drawn.chain(self.light).reduce(|a, b| if b.brightness > a.brightness { b } else { a })
    }

    /// The creature's fields that [`DressedLook::from_utc`] reads back as
    /// this, its items in its `Equip_ItemList` (the item structs
    /// themselves, as an area's creature has them).
    pub fn to_utc(&self) -> Struct {
        let mut utc = Struct::new(0);
        utc.set("Appearance_Type", Value::Word(self.appearance));
        utc.set("Gender", Value::Byte(self.gender));
        utc.set("Phenotype", Value::Int(i32::try_from(self.phenotype).unwrap_or(i32::MAX)));
        set_part(&mut utc, "Appearance_Head", self.head);
        for ((_, field, _), n) in BODY_PARTS.iter().zip(self.body_parts) {
            set_part(&mut utc, field, n);
        }
        for (field, v) in ["Color_Skin", "Color_Hair", "Color_Tattoo1", "Color_Tattoo2"]
            .into_iter()
            .zip(self.colors)
        {
            utc.set(field, Value::Byte(v));
        }
        utc.set("Wings_New", Value::Dword(self.wings));
        utc.set("Tail_New", Value::Dword(self.tail));
        set_transforms(
            &mut utc,
            &[
                (HEAD_SCOPE, self.head_transform),
                (TAIL_SCOPE, self.tail_transform),
                (WINGS_SCOPE, self.wings_transform),
                (CLOAK_SCOPE, self.cloak_transform),
            ],
        );
        let mut worn: Vec<Struct> =
            slot::DRAWN.into_iter().filter_map(|s| self.slot(s).map(|it| it.to_uti(s))).collect();
        // (A light of an undrawn slot's: a ring in the left ring slot.)
        if let Some(light) = self.light {
            let mut ring = Struct::new(slot::LEFT_RING);
            ring.set("BaseItem", Value::Int(slot::RING_BASE_ITEM));
            ring.set("PropertiesList", Value::List(vec![light.to_gff()]));
            worn.push(ring);
        }
        if !worn.is_empty() {
            utc.set("Equip_ItemList", Value::List(worn));
        }
        utc
    }

    /// The fields mg-rules reads of a creature to find its appearance row
    /// and its skeletons: these values' own.
    pub(crate) fn rules_fields(&self) -> Struct {
        let mut utc = Struct::new(0);
        utc.set("Appearance_Type", Value::Word(self.appearance));
        utc.set("Gender", Value::Byte(self.gender));
        utc.set("Phenotype", Value::Int(i32::try_from(self.phenotype).unwrap_or(i32::MAX)));
        utc
    }
}

impl From<crate::CreatureLook> for DressedLook {
    /// The bare body [`crate::CreatureLook`] describes, wearing nothing.
    fn from(look: crate::CreatureLook) -> DressedLook {
        let [skin, hair, tattoo1, tattoo2] = look.colors;
        DressedLook {
            appearance: look.appearance,
            gender: look.gender,
            phenotype: u32::from(look.phenotype),
            head: u16::from(look.head),
            body_parts: BODY_PARTS.map(|(mdlname, _, _)| {
                let bare = matches!(mdlname, "belt" | "shor" | "shol");
                if bare { 0 } else { u16::from(look.body) }
            }),
            colors: [skin, hair, tattoo1, tattoo2],
            wings: look.wings,
            tail: look.tail,
            ..DressedLook::default()
        }
    }
}

impl From<&crate::CreatureLook> for DressedLook {
    /// The bare body [`crate::CreatureLook`] describes, wearing nothing.
    fn from(look: &crate::CreatureLook) -> DressedLook {
        DressedLook::from(*look)
    }
}

/// The item in an equipment slot: a blueprint's `EquippedRes`, loaded with
/// `item`, or the item struct itself (area instances and characters).
fn equipped(utc: &Struct, slot: u32, item: &dyn Fn(ResRef) -> Option<Gff>) -> Option<Struct> {
    let s = utc.list("Equip_ItemList")?.iter().find(|s| s.id == slot)?;
    match s.resref("EquippedRes") {
        Some(r) if !r.is_empty() => item(r).map(|g| g.root),
        _ => Some(s.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axis(v: f32) -> Value {
        let mut a = Struct::new(0);
        a.set("ValueTo", Value::Float(v));
        Value::Struct(a)
    }

    /// A creature's fields as a builder's tools and the game write them:
    /// EE twins past 255 and in place of the BYTE, the older wings field,
    /// a head's visual transform with axes missing, and its items inline
    /// (an armor with a part's own color, a sword with a Light property
    /// after another property) and by blueprint (a cloak, loaded), and an
    /// item in a slot no preview draws (boots, with a Light property).
    fn fields() -> Struct {
        let mut utc = Struct::new(0);
        utc.set("Appearance_Type", Value::Word(6));
        utc.set("Gender", Value::Byte(1));
        utc.set("Phenotype", Value::Int(2));
        utc.set("Appearance_Head", Value::Byte(255));
        utc.set("xAppearance_Head", Value::Word(300));
        utc.set("BodyPart_LShin", Value::Byte(3));
        utc.set("ArmorPart_RFoot", Value::Byte(2));
        utc.set("Color_Skin", Value::Byte(4));
        utc.set("Color_Hair", Value::Byte(5));
        utc.set("Color_Tattoo2", Value::Byte(9));
        utc.set("Wings", Value::Byte(1));
        utc.set("Tail_New", Value::Dword(7));
        let mut head = Struct::new(6);
        head.set("Scope", Value::Int(254));
        head.set("ScaleX", axis(1.5));
        head.set("RotateZ", axis(30.0));
        utc.set("VisTransformList", Value::List(vec![head]));
        let mut armor = Struct::new(slot::CHEST);
        armor.set("BaseItem", Value::Int(16));
        armor.set("ArmorPart_Torso", Value::Byte(12));
        armor.set("ArmorPart_Robe", Value::Byte(3));
        armor.set("ArmorPart_LBicep", Value::Byte(255));
        armor.set("xArmorPart_LBice", Value::Word(260));
        armor.set("Cloth1Color", Value::Byte(20));
        armor.set("Metal2Color", Value::Byte(40));
        mg_rules::items::set_armor_part_color(&mut armor, 7, ArmorChannel::Cloth1, Some(33));
        let mut sword = Struct::new(slot::RIGHT_HAND);
        sword.set("BaseItem", Value::Int(1));
        sword.set("ModelPart1", Value::Byte(6));
        sword.set("ModelPart2", Value::Byte(1));
        sword.set("ModelPart3", Value::Byte(1));
        let mut whole = Struct::new(6);
        whole.set("Scope", Value::Int(255));
        whole.set("TranslateZ", axis(0.5));
        sword.set("VisTransformList", Value::List(vec![whole]));
        let property = |property: u16, cost_value: u16, param1_value: u8| {
            let p = mg_rules::items::ItemProperty {
                property,
                cost_value,
                param1_value,
                ..Default::default()
            };
            p.to_gff()
        };
        sword.set("PropertiesList", Value::List(vec![property(6, 1, 0), property(44, 3, 6)]));
        let mut cloak = Struct::new(slot::CLOAK);
        cloak.set("EquippedRes", Value::ResRef(b"my_cloak".to_vec()));
        let mut boots = Struct::new(0x8);
        boots.set("BaseItem", Value::Int(26));
        boots.set("PropertiesList", Value::List(vec![property(44, 2, 0)]));
        utc.set("Equip_ItemList", Value::List(vec![armor, sword, cloak, boots]));
        utc
    }

    /// The same creature as values, written out by hand.
    fn values() -> DressedLook {
        let mut body_parts = [0; 18];
        body_parts[0] = 2; // footr (the game's field: ArmorPart_RFoot)
        body_parts[3] = 3; // shinl
        let mut armor = ItemLook { base_item: 16, ..ItemLook::default() };
        armor.armor_parts[7] = 12;
        armor.armor_parts[18] = 3;
        armor.armor_parts[13] = 260;
        armor.colors[ArmorChannel::Cloth1 as usize] = Some(20);
        armor.colors[ArmorChannel::Metal2 as usize] = Some(40);
        armor.part_colors[7][ArmorChannel::Cloth1 as usize] = Some(33);
        let sword = ItemLook {
            base_item: 1,
            model_parts: [6, 1, 1],
            scopes: [
                Some(VisualTransform { translate: [0.0, 0.0, 0.5], ..VisualTransform::default() }),
                None,
                None,
            ],
            light: Some(LightProperty { brightness: 3, color: 6 }),
            ..ItemLook::default()
        };
        let cloak = ItemLook { base_item: 80, model_parts: [4, 0, 0], ..ItemLook::default() };
        DressedLook {
            appearance: 6,
            gender: 1,
            phenotype: 2,
            head: 300,
            body_parts,
            colors: [4, 5, 0, 9],
            wings: 1,
            tail: 7,
            head_transform: Some(VisualTransform {
                scale: 1.5,
                rotate: [0.0, 0.0, 30.0],
                ..VisualTransform::default()
            }),
            armor: Some(armor),
            right_hand: Some(sword),
            cloak: Some(cloak),
            light: Some(LightProperty { brightness: 2, color: 0 }),
            ..DressedLook::default()
        }
    }

    fn cloak_blueprint(r: ResRef) -> Option<Gff> {
        (r.as_str() == Some("my_cloak")).then(|| {
            let mut gff = Gff::new(*b"UTI ");
            gff.root.set("BaseItem", Value::Int(80));
            gff.root.set("ModelPart1", Value::Byte(4));
            gff
        })
    }

    #[test]
    fn a_creature_s_fields_read_as_its_values() {
        assert_eq!(DressedLook::from_utc(&fields(), &cloak_blueprint), values());
        // A blueprint's item that is not there is not worn.
        let bare = DressedLook::from_utc(&fields(), &|_| None);
        assert_eq!(bare, DressedLook { cloak: None, ..values() });
        // No fields at all: nothing.
        assert_eq!(DressedLook::from_utc(&Struct::new(0), &|_| None), DressedLook::default());
    }

    #[test]
    fn values_written_as_fields_read_back_as_themselves() {
        let look = values();
        let utc = look.to_utc();
        assert_eq!(DressedLook::from_utc(&utc, &|_| None), look);
        // Past 255 a part is the BYTE at 255 and its EE twin, as the game
        // writes it; below, the BYTE alone.
        assert_eq!(utc.integer("Appearance_Head"), Some(255));
        assert_eq!(utc.integer("xAppearance_Head"), Some(300));
        assert_eq!(utc.get("xBodyPart_LShin"), None);
        // Items inline, in their slots.
        let ids: Vec<u32> = utc.list("Equip_ItemList").unwrap().iter().map(|s| s.id).collect();
        // (The boots' light on a ring: a light of the undrawn slots.)
        assert_eq!(ids, [slot::CHEST, slot::RIGHT_HAND, slot::CLOAK, slot::LEFT_RING]);
        let none = DressedLook::default();
        assert_eq!(DressedLook::from_utc(&none.to_utc(), &|_| None), none);
    }

    /// A creature's light is the brightest Light property of all it
    /// wears, drawn or not: one light, as the client gives it.
    #[test]
    fn a_creature_s_light_is_its_brightest_item_s() {
        let look = values();
        assert_eq!(look.brightest_light(), Some(LightProperty { brightness: 3, color: 6 }));
        let ring = LightProperty { brightness: 4, color: 1 };
        assert_eq!(DressedLook { light: Some(ring), ..look }.brightest_light(), Some(ring));
        assert_eq!(DressedLook::default().brightest_light(), None);
        // An item's Light property written as Aurora writes it reads back.
        let lit = ItemLook { light: Some(ring), ..ItemLook::default() };
        let uti = lit.to_uti(slot::LEFT_HAND);
        let entry = &uti.list("PropertiesList").unwrap()[0];
        assert_eq!(entry.integer("CostTable"), Some(18), "iprp_lightcost");
        assert_eq!(entry.integer("Param1"), Some(9), "iprp_color");
        assert_eq!(ItemLook::from_uti(&uti), lit);
    }

    #[test]
    fn a_bare_look_is_its_fields() {
        let bare = crate::CreatureLook {
            gender: 1,
            phenotype: 2,
            head: 4,
            body: 3,
            colors: [1, 2, 3, 4],
            wings: 5,
            tail: 6,
            ..crate::CreatureLook::new(6)
        };
        assert_eq!(DressedLook::from(bare), DressedLook::from_utc(&bare.to_utc(), &|_| None));
    }

    #[test]
    fn a_slot_is_its_item() {
        let mut look = values();
        assert_eq!(look.slot(slot::CHEST), look.armor.as_ref());
        assert_eq!(look.slot(0x8), None, "boots are not drawn");
        assert!(look.slot_mut(0x8).is_none());
        *look.slot_mut(slot::LEFT_HAND).unwrap() = Some(ItemLook::default());
        assert_eq!(look.left_hand, Some(ItemLook::default()));
        let armor = look.armor.as_ref().unwrap();
        assert_eq!(armor.armor_part("ArmorPart_Robe"), 3);
        assert_eq!(armor.armor_part("Robe"), 0);
    }

    /// A transform that changes nothing is no matrix; one read from an
    /// entry is the entry's matrix ([`crate::scope_matrix`]), and written
    /// as an entry it reads back as itself.
    #[test]
    fn a_transform_s_matrix_is_the_scope_s() {
        assert_eq!(VisualTransform::default().matrix(), None);
        let utc = fields();
        let t = VisualTransform::of(&utc, HEAD_SCOPE).unwrap();
        assert!(t.matrix().is_some());
        assert_eq!(t.matrix(), crate::scope_matrix(&utc, HEAD_SCOPE));
        let mut o = Struct::new(0);
        o.set("VisTransformList", Value::List(vec![t.to_entry(HEAD_SCOPE)]));
        assert_eq!(VisualTransform::of(&o, HEAD_SCOPE), Some(t));
        assert_eq!(VisualTransform::of(&o, TAIL_SCOPE), None);
    }
}
