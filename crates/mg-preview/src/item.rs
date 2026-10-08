//! Items by baseitems.2da `ModelType`: simple (one model), layered (a
//! helmet with PLT colours, or a cloak), composite (bottom, middle and top
//! parts) and armour (worn by a human body, as the toolset shows it).

use mg_core::ResRef;
use mg_gff::{Struct, Value};
use mg_rules::GameData;

use crate::{Lookup, Part, Preview, PreviewError, cell, cell_int, env_map, item_colors};

/// An item blueprint (UTI fields); armour as a man wears it.
pub fn item(game: &GameData, uti: &Struct) -> Result<Preview, PreviewError> {
    item_on(game, uti, false)
}

/// [`item`], armour and cloaks as a woman wears them where `female` (the
/// game has a model of each armour part and cloak for either).
pub fn item_on(game: &GameData, uti: &Struct, female: bool) -> Result<Preview, PreviewError> {
    let baseitems = game.table("baseitems")?;
    let base = uti.integer("BaseItem").unwrap_or(0);
    let row =
        usize::try_from(base).map_err(|_| PreviewError::NoRow { table: "baseitems", row: base })?;
    if cell_int(&baseitems, row, "ModelType") == Some(3) {
        return armor(game, uti, female);
    }
    let mut parts = item_parts(game, uti, [0; 10], female)?.into_iter();
    let first = parts.next().ok_or_else(|| PreviewError::NoModel(format!("item {base}")))?;
    Ok(Preview { base: first, parts: parts.collect(), idle: None, lights: Vec::new() })
}

/// An item as it lies in an area, as Aurora draws it there. Armour,
/// which is its wearer's body parts, is the game's model of armour
/// dropped (`gi_armor01`…`04`, by its weight); a cloak, which hangs from
/// shoulders, is the game's model of a cloak dropped (`gi_cloak01`, folded,
/// in the cloak's colors); every other item is its own model, as [`item`]
/// shows it. Where the game's model is not there, the base item's
/// `DefaultModel`, then the bag the game drops things as (`it_bag`). How
/// each is turned there: [`ground_turn`].
pub fn item_placed(game: &GameData, uti: &Struct) -> Result<Preview, PreviewError> {
    let lk = Lookup { game };
    let baseitems = game.table("baseitems")?;
    let base = uti.integer("BaseItem").unwrap_or(0);
    let row =
        usize::try_from(base).map_err(|_| PreviewError::NoRow { table: "baseitems", row: base })?;
    let class = cell(&baseitems, row, "ItemClass").unwrap_or_default().to_ascii_lowercase();
    let armor = cell_int(&baseitems, row, "ModelType") == Some(3);
    if !armor && class != "cloak" {
        return item(game, uti);
    }
    let one = |mut part: Part| {
        // (A model with a PLT texture takes the item's colors.)
        part.colors = Some(item_colors(uti, [0; 10]));
        Preview { base: part, parts: Vec::new(), idle: None, lights: Vec::new() }
    };
    let default = cell(&baseitems, row, "DefaultModel").map(str::to_ascii_lowercase);
    if armor {
        // The game's own models of armor lying on the ground, by how
        // heavy it is (its torso's parts_chest.2da `ACBONUS`): cloth,
        // leather, chain, plate. Which model is which weight is told by
        // their looks; the base item's `DefaultModel` where one is not
        // there.
        let torso = mg_rules::items::part_number(uti, "ArmorPart_Torso").unwrap_or(0);
        let parts = game.table("parts_chest")?;
        let ac = usize::try_from(torso)
            .ok()
            .and_then(|row| crate::cell_f32(&parts, row, "ACBONUS"))
            .unwrap_or(0.0);
        let by_weight = match ac as i32 {
            ..=0 => "gi_armor01",
            1..=3 => "gi_armor04",
            4..=5 => "gi_armor03",
            _ => "gi_armor02",
        };
        let model =
            std::iter::once(by_weight.to_string()).chain(default.clone()).find(|m| lk.has_model(m));
        if let Some(model) = model {
            return Ok(one(Part::new(model)));
        }
    }
    if !armor {
        let model = std::iter::once(CLOAK.to_string()).chain(default).find(|m| lk.has_model(m));
        if let Some(model) = model {
            return Ok(one(Part::new(model)));
        }
    }
    if !lk.has_model(BAG) {
        return Err(PreviewError::NoModel(format!("{class} on the ground")));
    }
    Ok(one(Part::new(BAG)))
}

/// How an item is turned where it lies in an area: baseitems.2da's
/// `RotateOnGround` (0 as its model is, 1 a quarter turn about the model's
/// Y axis, a sword or a shield on its flat; 2 about its X axis, a potion
/// stood up; the game's own cloaks and armor have 0).
pub fn ground_turn(game: &GameData, uti: &Struct) -> u8 {
    let Ok(baseitems) = game.table("baseitems") else { return 0 };
    let Ok(row) = usize::try_from(uti.integer("BaseItem").unwrap_or(0)) else { return 0 };
    cell_int(&baseitems, row, "RotateOnGround").and_then(|v| u8::try_from(v).ok()).unwrap_or(0)
}

/// The bag an item with no model of its own lies in an area as.
const BAG: &str = "it_bag";

/// The game's model of a cloak lying on the ground.
const CLOAK: &str = "gi_cloak01";

/// The models of an item, at its origin (held items hang all of them from
/// the hand). Colours for layered items start from `colors` (the wearer's
/// skin and hair); a cloak is a woman's where `female`.
pub(crate) fn item_parts(
    game: &GameData,
    uti: &Struct,
    colors: [u8; 10],
    female: bool,
) -> Result<Vec<Part>, PreviewError> {
    let lk = Lookup { game };
    let baseitems = game.table("baseitems")?;
    let base = uti.integer("BaseItem").unwrap_or(0);
    let row =
        usize::try_from(base).map_err(|_| PreviewError::NoRow { table: "baseitems", row: base })?;
    let class = cell(&baseitems, row, "ItemClass")
        .ok_or(PreviewError::NoRow { table: "baseitems", row: base })?
        .to_ascii_lowercase();
    let n = |field: &str| mg_rules::items::part_number(uti, field).unwrap_or(0);
    // Item environment maps are the default one whatever baseitems says.
    let part = |model: String| {
        let mut p = Part::new(model);
        p.env_map = Some("chrome1".into());
        p
    };
    let parts = match cell_int(&baseitems, row, "ModelType").unwrap_or(0) {
        // Composite: bottom, middle, top.
        2 => ["b", "m", "t"]
            .iter()
            .zip(["ModelPart1", "ModelPart2", "ModelPart3"])
            .map(|(p, f)| format!("{class}_{p}_{:03}", n(f)))
            .filter(|m| lk.has_model(m))
            .map(part)
            .collect(),
        // Layered: helmets; cloaks shown as worn by a human (a man, or a
        // woman where `female` and the cloak has her model).
        1 if class == "cloak" => {
            let cloaks = game.table("cloakmodel")?;
            let r = n("ModelPart1").max(0) as usize;
            let number = cell_int(&cloaks, r, "MODEL").unwrap_or(1);
            let hers = format!("pfh0_cloak_{number:03}");
            let model = if female && lk.has_model(&hers) {
                hers
            } else {
                format!("pmh0_cloak_{number:03}")
            };
            if lk.has_model(&model) {
                let mut p = part(model);
                let texture = format!("cloak_{:03}", cell_int(&cloaks, r, "TEXTURE").unwrap_or(1));
                p.textures =
                    lk.bitmaps(&p.model).into_iter().map(|b| (b, texture.clone())).collect();
                p.colors = Some(item_colors(uti, colors));
                p.env_map = env_map(cell(&cloaks, r, "ENVMAP"));
                vec![p]
            } else {
                Vec::new()
            }
        }
        1 => {
            let model = format!("{class}_{:03}", n("ModelPart1"));
            if lk.has_model(&model) {
                let mut p = part(model);
                p.colors = Some(item_colors(uti, colors));
                vec![p]
            } else {
                Vec::new()
            }
        }
        _ => {
            let model = format!("{class}_{:03}", n("ModelPart1"));
            if lk.has_model(&model) { vec![part(model)] } else { Vec::new() }
        }
    };
    if !parts.is_empty() {
        return Ok(parts);
    }
    // No model for this part number: the base item's default one.
    let default = cell(&baseitems, row, "DefaultModel")
        .map(str::to_ascii_lowercase)
        .filter(|m| lk.has_model(m))
        .ok_or_else(|| PreviewError::NoModel(format!("{class} {}", n("ModelPart1"))))?;
    Ok(vec![part(default)])
}

/// Armour, worn by a human (a man, or a woman where `female`) of
/// phenotype 0 with head 1 and bare body parts where the armour has none.
fn armor(game: &GameData, uti: &Struct, female: bool) -> Result<Preview, PreviewError> {
    let mut utc = Struct::new(0);
    for (field, v) in [
        ("Appearance_Type", 6u16), // Human
        ("Gender", u16::from(female)),
        ("Phenotype", 0),
        ("Appearance_Head", 1),
    ] {
        utc.set(field, Value::Word(v));
    }
    // Bare skin where the armour has no part (no bare shoulders or belt).
    for field in [
        "ArmorPart_RFoot",
        "BodyPart_LFoot",
        "BodyPart_RShin",
        "BodyPart_LShin",
        "BodyPart_LThigh",
        "BodyPart_RThigh",
        "BodyPart_Pelvis",
        "BodyPart_Torso",
        "BodyPart_Neck",
        "BodyPart_RFArm",
        "BodyPart_LFArm",
        "BodyPart_RBicep",
        "BodyPart_LBicep",
        "BodyPart_RHand",
        "BodyPart_LHand",
    ] {
        utc.set(field, Value::Byte(1));
    }
    let mut worn = uti.clone();
    worn.id = 0x2;
    utc.set("Equip_ItemList", Value::List(vec![worn]));
    crate::creature(game, &utc, &|_: ResRef| None)
}
