//! The names of the models an item's parts and a body of parts are drawn
//! with, made from the object's part numbers and the rules 2DAs: the one
//! naming the blueprint previews (`mg_preview`), the usage report and the
//! appearance rule ([`crate::appearance`]) go by.
//!
//! An item is drawn by its base item's `ModelType`: a simple or layered
//! item (0 and 1) as `<ItemClass>_<ModelPart1>` (`helm_005`), a cloak as
//! its cloakmodel.2da row's `MODEL` worn by a human (`pmh0_cloak_003`), a
//! composite one (2) as its bottom, middle and top
//! (`wswls_b_011`, `_m_`, `_t_`), and armor (3) as its parts on a body
//! (`pmh0_chest012`, a man's, or `pfh0_` a woman's). A body of parts
//! (appearance.2da `MODELTYPE` P) is a skeleton named for its gender, its
//! row's `RACE` letter and its phenotype (`pmh0`; where that is not
//! there, its phenotype.2da `DefaultPhenoType`'s), and a model per part
//! hung on it (`pmh0_head001`, `pmh0_shinr002`).

use mg_2da::TwoDa;
use mg_gff::Struct;
use mg_resman::ResMan;

use crate::items::part_number;

/// Per capart.2da `MDLNAME`: the part's name in its models, the
/// creature's field with its part number and the armor's. (The
/// creature's right foot really is `ArmorPart_RFoot`.) In the order of
/// the game's armor parts ([`crate::items::ARMOR_PARTS`]), the robe,
/// which is the armor's alone, left out ([`ROBE`]).
pub const BODY_PARTS: [(&str, &str, &str); 18] = [
    ("footr", "ArmorPart_RFoot", "ArmorPart_RFoot"),
    ("footl", "BodyPart_LFoot", "ArmorPart_LFoot"),
    ("shinr", "BodyPart_RShin", "ArmorPart_RShin"),
    ("shinl", "BodyPart_LShin", "ArmorPart_LShin"),
    ("legl", "BodyPart_LThigh", "ArmorPart_LThigh"),
    ("legr", "BodyPart_RThigh", "ArmorPart_RThigh"),
    ("pelvis", "BodyPart_Pelvis", "ArmorPart_Pelvis"),
    ("chest", "BodyPart_Torso", "ArmorPart_Torso"),
    ("belt", "BodyPart_Belt", "ArmorPart_Belt"),
    ("neck", "BodyPart_Neck", "ArmorPart_Neck"),
    ("forer", "BodyPart_RFArm", "ArmorPart_RFArm"),
    ("forel", "BodyPart_LFArm", "ArmorPart_LFArm"),
    ("bicepr", "BodyPart_RBicep", "ArmorPart_RBicep"),
    ("bicepl", "BodyPart_LBicep", "ArmorPart_LBicep"),
    ("shor", "BodyPart_RShoul", "ArmorPart_RShoul"),
    ("shol", "BodyPart_LShoul", "ArmorPart_LShoul"),
    ("handr", "BodyPart_RHand", "ArmorPart_RHand"),
    ("handl", "BodyPart_LHand", "ArmorPart_LHand"),
];

/// The robe: its name in its models and the armor's field.
pub const ROBE: (&str, &str) = ("robe", "ArmorPart_Robe");

/// A composite item's pieces, bottom first: each one's letter in its
/// models' names and the item's field with its number.
pub const COMPOSITE: [(&str, &str); 3] =
    [("b", "ModelPart1"), ("m", "ModelPart2"), ("t", "ModelPart3")];

/// A simple or layered item's model: `<ItemClass>_<n>` (`helm_005`).
pub fn item_model(class: &str, n: i64) -> String {
    format!("{}_{n:03}", class.to_ascii_lowercase())
}

/// A composite item's piece `piece` (`b`, `m` or `t`): `wswls_b_011`.
pub fn composite_model(class: &str, piece: &str, n: i64) -> String {
    format!("{}_{piece}_{n:03}", class.to_ascii_lowercase())
}

/// A cloak (an item) as a human wears it, by its cloakmodel.2da row's
/// `MODEL`: a man's (`pmh0_cloak_003`) or a woman's.
pub fn cloak_model(female: bool, number: i64) -> String {
    let g = if female { 'f' } else { 'm' };
    format!("p{g}h0_cloak_{number:03}")
}

/// A helmet's model: `helm_<ModelPart1>`.
pub fn helmet_model(n: i64) -> String {
    item_model("helm", n)
}

/// A body of parts' skeleton: `p`, its gender (`m`, `f`), its row's
/// `RACE` letter and its phenotype (`pmh0`).
pub fn skeleton(gender: &str, race: &str, phenotype: i64) -> String {
    format!("p{gender}{race}{phenotype}").to_ascii_lowercase()
}

/// A part of a body hung on `skeleton`: `pmh0_shinr002`.
pub fn body_part_model(skeleton: &str, part: &str, n: i64) -> String {
    format!("{skeleton}_{part}{n:03}")
}

/// A cloak worn by a body on `skeleton`: `pmh0_cloak_003`.
pub fn body_cloak_model(skeleton: &str, n: i64) -> String {
    format!("{skeleton}_cloak_{n:03}")
}

/// The skeletons a body of parts is looked for in, in order: its
/// phenotype's, then the phenotype's `DefaultPhenoType`'s (the same where
/// that is its own). `appearance` is the creature's appearance.2da row,
/// `gender` and `phenotype` the tables (`None`: not there, a man of
/// phenotype 0's default 0). `None` for a row that is no body of parts.
pub fn skeletons(
    utc: &Struct,
    appearance: &TwoDa,
    gender: Option<&TwoDa>,
    phenotype: Option<&TwoDa>,
) -> Option<[String; 2]> {
    let number = |label: &str| utc.integer(label).unwrap_or(0);
    let cell = |t: &TwoDa, row: i64, column: &str| -> Option<String> {
        let v = t.get(usize::try_from(row).ok()?, column)?.trim();
        (!v.is_empty() && v != "****").then(|| v.to_string())
    };
    let row = number("Appearance_Type");
    let parts =
        cell(appearance, row, "MODELTYPE").is_some_and(|t| t.to_ascii_uppercase().starts_with('P'));
    if !parts {
        return None;
    }
    let race = cell(appearance, row, "RACE")?.to_ascii_lowercase();
    let g = gender
        .and_then(|t| cell(t, number("Gender").max(0), "GENDER"))
        .map(|v| v.to_ascii_lowercase())
        .filter(|v| v == "m" || v == "f")
        .unwrap_or_else(|| "m".into());
    let pheno = number("Phenotype").max(0);
    let default = phenotype
        .and_then(|t| cell(t, pheno, "DefaultPhenoType"))
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);
    Some([skeleton(&g, &race, pheno), skeleton(&g, &race, default)])
}

/// A model named by one of an object's parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    /// The object's field with the part's number (`ModelPart2`,
    /// `ArmorPart_Torso`, `BodyPart_LShin`, `Appearance_Head`).
    pub field: &'static str,
    /// The models it is drawn as, any one of which will do, in the order
    /// they are looked for (lower case).
    pub models: Vec<String>,
}

/// The models an item's parts name (an item of base item row `row` of
/// `baseitems`, with cloakmodel.2da `cloaks`): one for a simple or
/// layered item, the bottom, middle and top of a composite one, a cloak's
/// as a man or a woman wears it, and each part of an armor numbered above
/// 0 as a man or a woman of phenotype 0 wears it. Empty for a row that
/// isn't there.
pub fn item_models(uti: &Struct, baseitems: &TwoDa, cloaks: Option<&TwoDa>) -> Vec<Named> {
    let Ok(row) = usize::try_from(uti.integer("BaseItem").unwrap_or(0)) else {
        return Vec::new();
    };
    let cell = |t: &TwoDa, row: usize, column: &str| -> Option<String> {
        let v = t.get(row, column)?.trim();
        (!v.is_empty() && v != "****").then(|| v.to_string())
    };
    let Some(class) = cell(baseitems, row, "ItemClass") else { return Vec::new() };
    let n = |field: &str| part_number(uti, field).unwrap_or(0);
    let model_type = cell(baseitems, row, "ModelType").and_then(|v| v.parse::<i64>().ok());
    match model_type.unwrap_or(0) {
        2 => COMPOSITE
            .iter()
            .map(|(piece, field)| Named {
                field,
                models: vec![composite_model(&class, piece, n(field))],
            })
            .collect(),
        3 => BODY_PARTS
            .iter()
            .map(|(part, _, field)| (*part, *field))
            .chain([ROBE])
            .filter(|(_, field)| n(field) > 0)
            .map(|(part, field)| Named {
                field,
                models: ["pmh0", "pfh0"]
                    .iter()
                    .map(|s| body_part_model(s, part, n(field)))
                    .collect(),
            })
            .collect(),
        1 if class.eq_ignore_ascii_case("cloak") => {
            let r = n("ModelPart1").max(0) as usize;
            let number = cloaks
                .and_then(|t| cell(t, r, "MODEL"))
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(1);
            let models = vec![cloak_model(false, number), cloak_model(true, number)];
            vec![Named { field: "ModelPart1", models }]
        }
        _ => vec![Named { field: "ModelPart1", models: vec![item_model(&class, n("ModelPart1"))] }],
    }
}

/// The models a body of parts' own parts name (its head and each body
/// part numbered above 0) on its `skeletons` ([`skeletons`]).
pub fn body_models(utc: &Struct, skeletons: &[String; 2]) -> Vec<Named> {
    let number = |label: &str| utc.integer(label).unwrap_or(0);
    BODY_PARTS
        .iter()
        .map(|(part, field, _)| (*part, *field))
        .chain([("head", "Appearance_Head")])
        .filter(|(_, field)| number(field) > 0)
        .map(|(part, field)| Named {
            field,
            models: skeletons.iter().map(|s| body_part_model(s, part, number(field))).collect(),
        })
        .collect()
}

/// Whether the module, its haks or the game has model `name`.
pub fn has_model(resman: &ResMan, name: &str) -> bool {
    mg_resman::ResKey::parse(name, mg_core::ResType::MDL).is_some_and(|k| resman.contains(&k))
}

#[cfg(test)]
mod tests {
    use mg_gff::Value;

    use super::*;

    fn table(text: &str) -> TwoDa {
        TwoDa::parse(text.as_bytes(), Default::default()).unwrap()
    }

    fn object(fields: &[(&str, i64)]) -> Struct {
        let mut s = Struct::default();
        for (label, v) in fields {
            s.set(label, Value::Dword(*v as u32));
        }
        s
    }

    #[test]
    fn an_item_s_models_by_its_model_type() {
        let baseitems = table(
            "2DA V2.0\n\n  label ItemClass ModelType\n0 shortsword WSwSs 2\n1 helmet Helm 1\n\
             2 cloak Cloak 1\n3 armor Armor 3\n4 ring IT_Ring 0\n5 **** **** ****\n",
        );
        let cloaks = table("2DA V2.0\n\n  LABEL MODEL\n0 none 1\n1 Cloak 3\n");
        let models = |fields: &[(&str, i64)]| -> Vec<(&str, Vec<String>)> {
            item_models(&object(fields), &baseitems, Some(&cloaks))
                .into_iter()
                .map(|n| (n.field, n.models))
                .collect()
        };
        let one = |f: &'static str, m: &str| (f, vec![m.to_string()]);
        assert_eq!(
            models(&[("BaseItem", 0), ("ModelPart1", 1), ("ModelPart2", 22), ("ModelPart3", 3)]),
            [
                one("ModelPart1", "wswss_b_001"),
                one("ModelPart2", "wswss_m_022"),
                one("ModelPart3", "wswss_t_003")
            ]
        );
        assert_eq!(models(&[("BaseItem", 1), ("ModelPart1", 5)]), [one("ModelPart1", "helm_005")]);
        assert_eq!(
            models(&[("BaseItem", 4), ("ModelPart1", 7)]),
            [one("ModelPart1", "it_ring_007")]
        );
        assert_eq!(
            models(&[("BaseItem", 2), ("ModelPart1", 1)]),
            [("ModelPart1", vec!["pmh0_cloak_003".to_string(), "pfh0_cloak_003".into()])]
        );
        // Armor: its parts above 0, the robe too, a man's and a woman's.
        assert_eq!(
            models(&[("BaseItem", 3), ("ArmorPart_Torso", 12), ("ArmorPart_Robe", 4)]),
            [
                ("ArmorPart_Torso", vec!["pmh0_chest012".to_string(), "pfh0_chest012".into()]),
                ("ArmorPart_Robe", vec!["pmh0_robe004".to_string(), "pfh0_robe004".into()]),
            ]
        );
        assert!(models(&[("BaseItem", 5)]).is_empty() && models(&[("BaseItem", 9)]).is_empty());
    }

    #[test]
    fn a_body_s_skeleton_and_parts() {
        let appearance = table(
            "2DA V2.0\n\n  LABEL RACE MODELTYPE\n0 Dwarf D P\n1 Badger c_badger S\n2 Elf e p\n",
        );
        let gender = table("2DA V2.0\n\n  LABEL GENDER\n0 Male M\n1 Female F\n2 Both B\n");
        let pheno =
            table("2DA V2.0\n\n  Label DefaultPhenoType\n0 Normal 0\n2 Large 2\n7 Mine 0\n");
        let of = |fields: &[(&str, i64)]| {
            skeletons(&object(fields), &appearance, Some(&gender), Some(&pheno))
        };
        assert_eq!(of(&[("Appearance_Type", 0)]), Some(["pmd0".into(), "pmd0".into()]));
        assert_eq!(of(&[("Appearance_Type", 1)]), None, "one model of its own");
        assert_eq!(
            of(&[("Appearance_Type", 2), ("Gender", 1), ("Phenotype", 7)]),
            Some(["pfe7".into(), "pfe0".into()]),
            "a phenotype of a server's own, by its default"
        );
        // (A gender of neither, or no tables: a man of phenotype 0's.)
        assert_eq!(of(&[("Gender", 2)]).unwrap()[0], "pmd0");
        let none = skeletons(&object(&[("Phenotype", 7)]), &appearance, None, None);
        assert_eq!(none, Some(["pmd7".into(), "pmd0".into()]));
        let skel = of(&[("Appearance_Type", 2), ("Phenotype", 7)]).unwrap();
        let parts = body_models(
            &object(&[("Appearance_Head", 3), ("BodyPart_LShin", 2), ("BodyPart_Neck", 0)]),
            &skel,
        );
        let parts: Vec<_> = parts.into_iter().map(|n| (n.field, n.models)).collect();
        assert_eq!(
            parts,
            [
                ("BodyPart_LShin", vec!["pme7_shinl002".to_string(), "pme0_shinl002".into()]),
                ("Appearance_Head", vec!["pme7_head003".to_string(), "pme0_head003".into()]),
            ]
        );
    }
}
