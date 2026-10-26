//! What an object's texture and animation replacements can name: the
//! textures its models' meshes are drawn with, and the animations its base
//! model plays (its own and its supermodels'). The Visuals page lists them
//! to pick from.

use std::sync::Arc;

use mg_core::{ResRef, ResType};
use mg_mdl::Model;
use mg_resman::ResKey;
use mg_rules::GameData;

use crate::{Lookup, Preview};

/// A texture of an object's models.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelTexture {
    /// Lower case, as the meshes name it.
    pub name: String,
    /// Only a PLT has the name (no DDS or TGA): drawn in the object's
    /// colors, and not replaced by the game.
    pub plt: bool,
    /// The models whose meshes use it.
    pub models: Vec<String>,
    /// The PLT colors of the first part that uses it.
    pub colors: Option<[u8; 10]>,
}

/// The textures the meshes of a preview's models are drawn with (the base
/// and its parts), each once, by name.
pub fn textures(game: &GameData, preview: &Preview) -> Vec<ModelTexture> {
    let lk = Lookup { game };
    let mut out: Vec<ModelTexture> = Vec::new();
    for part in std::iter::once(&preview.base).chain(&preview.parts) {
        for name in lk.bitmaps(&part.model) {
            match out.iter_mut().find(|t| t.name == name) {
                Some(t) => {
                    if !t.models.contains(&part.model) {
                        t.models.push(part.model.clone());
                    }
                }
                None => out.push(ModelTexture {
                    plt: is_plt(game, &name),
                    name,
                    models: vec![part.model.clone()],
                    colors: part.colors,
                }),
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Whether the texture of this name is a PLT alone.
fn is_plt(game: &GameData, name: &str) -> bool {
    let Some(r) = ResRef::cut(name) else { return false };
    game.resman.texture_type(r).is_none() && game.resman.contains(&ResKey::new(r, ResType::PLT))
}

/// The animations a preview's base model plays: its own and its
/// supermodels', nearest first, each name once (lower case).
pub fn animations(game: &GameData, preview: &Preview) -> Vec<String> {
    let lk = Lookup { game };
    let Some(model) = lk.model(&preview.base.model) else { return Vec::new() };
    let load = |name: &str| -> Option<Arc<Model>> { lk.model(name) };
    mg_render::anim::animations(&model, &load)
        .into_iter()
        .map(|(n, _)| n.to_ascii_lowercase())
        .collect()
}

/// Whether a texture of this name is one a replacement can draw with: a
/// DDS or TGA (the game's `ReplaceObjectTexture` takes simple textures,
/// not PLTs).
pub fn is_simple_texture(game: &GameData, name: &str) -> bool {
    ResRef::cut(name).is_some_and(|r| game.resman.texture_type(r).is_some())
}

#[cfg(test)]
mod tests {
    use mg_resman::{LayerClass, MemContainer, ResMan, priority};

    use super::*;

    fn model(name: &str, supermodel: &str, bitmaps: &[&str], animations: &[&str]) -> String {
        let mut s = format!(
            "newmodel {name}\nsetsupermodel {name} {supermodel}\nclassification character\n\
             beginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n"
        );
        for (i, b) in bitmaps.iter().enumerate() {
            s += &format!(
                "node trimesh m{i}\n  parent {name}\n  bitmap {b}\n\
                 verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
                 tverts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
                 faces 1\n    0 1 2 1 0 1 2 0\nendnode\n"
            );
        }
        s += &format!("endmodelgeom {name}\n");
        for a in animations {
            s += &format!(
                "newanim {a} {name}\n  length 1\n  transtime 0.25\n  animroot {name}\n\
                 node dummy {name}\n    parent NULL\n  endnode\ndoneanim {a} {name}\n"
            );
        }
        s + &format!("donemodel {name}\n")
    }

    fn game() -> GameData {
        let key = |name: &str| ResKey::from_filename(name).unwrap();
        let mut c = MemContainer::new();
        let base = model("mg_base", "mg_super", &["MG_Wood", "mg_skin", "mg_wood"], &["open"]);
        c.insert(key("mg_base.mdl"), base.as_bytes());
        let part = model("mg_part", "NULL", &["mg_wood", "mg_metal"], &[]);
        c.insert(key("mg_part.mdl"), part.as_bytes());
        let sup = model("mg_super", "NULL", &[], &["Close", "open", "default"]);
        c.insert(key("mg_super.mdl"), sup.as_bytes());
        c.insert(key("mg_wood.dds"), &b"dds"[..]);
        c.insert(key("mg_skin.plt"), &b"plt"[..]);
        // A PLT with a simple texture of its name too: drawn with that.
        c.insert(key("mg_metal.plt"), &b"plt"[..]);
        c.insert(key("mg_metal.tga"), &b"tga"[..]);
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "hak:x", LayerClass::Erf, c);
        GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH))
    }

    #[test]
    fn an_object_s_textures_are_its_models_meshes() {
        let game = game();
        let mut p = Preview::model("mg_base");
        let mut part = crate::Part::new("mg_part");
        part.colors = Some([3; 10]);
        p.parts.push(part);
        let found = textures(&game, &p);
        let names: Vec<(&str, bool)> = found.iter().map(|t| (t.name.as_str(), t.plt)).collect();
        assert_eq!(names, [("mg_metal", false), ("mg_skin", true), ("mg_wood", false)]);
        let wood = &found[2];
        assert_eq!(wood.models, ["mg_base", "mg_part"]);
        assert_eq!(found[0].colors, Some([3; 10]));
        assert!(is_simple_texture(&game, "MG_WOOD"));
        assert!(!is_simple_texture(&game, "mg_skin"));
        // No model, none.
        assert!(textures(&game, &Preview::model("nothing")).is_empty());
    }

    #[test]
    fn an_object_s_animations_are_its_own_and_its_supermodels() {
        let game = game();
        assert_eq!(animations(&game, &Preview::model("mg_base")), ["open", "close", "default"]);
        assert!(animations(&game, &Preview::model("mg_part")).is_empty());
        assert!(animations(&game, &Preview::model("nothing")).is_empty());
    }
}
