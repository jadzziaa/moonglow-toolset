//! Items by baseitems.2da `ModelType`: simple (one model), layered (a
//! helmet with PLT colours, or a cloak), composite (bottom, middle and top
//! parts) and armour (worn by a human body, as the toolset shows it).

use mg_core::ResRef;
use mg_gff::{Struct, Value};
use mg_rules::GameData;

use crate::{Lookup, Part, Preview, PreviewError, cell, cell_int, env_map, item_colors};

/// An item blueprint (UTI fields).
pub fn item(game: &GameData, uti: &Struct) -> Result<Preview, PreviewError> {
    let baseitems = game.table("baseitems")?;
    let base = uti.integer("BaseItem").unwrap_or(0);
    let row =
        usize::try_from(base).map_err(|_| PreviewError::NoRow { table: "baseitems", row: base })?;
    if cell_int(&baseitems, row, "ModelType") == Some(3) {
        return armor(game, uti);
    }
    let mut parts = item_parts(game, uti, [0; 10])?.into_iter();
    let first = parts.next().ok_or_else(|| PreviewError::NoModel(format!("item {base}")))?;
    Ok(Preview { base: first, parts: parts.collect(), idle: None, lights: Vec::new() })
}

/// The models of an item, at its origin (held items hang all of them from
/// the hand). Colours for layered items start from `colors` (the wearer's
/// skin and hair).
pub(crate) fn item_parts(
    game: &GameData,
    uti: &Struct,
    colors: [u8; 10],
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
        // Layered: helmets; cloaks shown as worn by a human male.
        1 if class == "cloak" => {
            let cloaks = game.table("cloakmodel")?;
            let r = n("ModelPart1").max(0) as usize;
            let model = format!("pmh0_cloak_{:03}", cell_int(&cloaks, r, "MODEL").unwrap_or(1));
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

/// Armour, worn by a human male of phenotype 0 with head 1 and bare body
/// parts where the armour has none.
fn armor(game: &GameData, uti: &Struct) -> Result<Preview, PreviewError> {
    let mut utc = Struct::new(0);
    for (field, v) in [
        ("Appearance_Type", 6u16), // Human
        ("Gender", 0),
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
