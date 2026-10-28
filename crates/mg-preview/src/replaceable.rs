//! What an object's texture and animation replacements can name: the
//! textures its models are drawn with (their meshes', the materials'
//! the meshes are drawn with, their emitters' and their lights' flares),
//! and the animations its base model plays (its own and its
//! supermodels'). The Visuals page lists them to pick from.

use std::sync::Arc;

use mg_core::{ResRef, ResType};
use mg_image::mtr::Mtr;
use mg_mdl::{Model, NodeKind};
use mg_resman::ResKey;
use mg_rules::GameData;

use crate::{Lookup, Preview};

/// A texture of an object's models.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelTexture {
    /// Lower case, as the models name it.
    pub name: String,
    /// Only a PLT has the name (no DDS or TGA): drawn in the object's
    /// colors, and not replaced by the game.
    pub plt: bool,
    /// The models that use it.
    pub models: Vec<String>,
    /// The PLT colors of the first part that uses it.
    pub colors: Option<[u8; 10]>,
    /// What names it, where the first model to use it does.
    pub source: Source,
}

/// What in a model names a texture it is drawn with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A mesh's `bitmap`.
    Mesh,
    /// A material (an MTR) a mesh is drawn with: its name. (A mesh's
    /// `materialname`, or a `bitmap` an MTR has the name of, which EE
    /// draws as that material.)
    Material(String),
    /// A particle emitter.
    Emitter,
    /// A light's lens flare.
    Flare,
}

/// The textures a preview's models are drawn with (the base and its
/// parts), each once, by name.
pub fn textures(game: &GameData, preview: &Preview) -> Vec<ModelTexture> {
    let lk = Lookup { game };
    let mut out: Vec<ModelTexture> = Vec::new();
    for part in std::iter::once(&preview.base).chain(&preview.parts) {
        let Some(model) = lk.model(&part.model) else { continue };
        for (name, source) in named(game, &model) {
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
                    source,
                }),
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// The textures a model names, each with what names it (lower case): the
/// meshes it draws (their `bitmap`, and the textures of the materials they
/// are drawn with), its emitters' and its lights' flares.
fn named(game: &GameData, m: &Model) -> Vec<(String, Source)> {
    let mut out: Vec<(String, Source)> = Vec::new();
    let mut add = |name: &str, source: Source| {
        let name = name.trim().to_ascii_lowercase();
        if !name.is_empty() && name != "null" && !out.iter().any(|(n, _)| *n == name) {
            out.push((name, source));
        }
    };
    for n in &m.nodes {
        match &n.kind {
            NodeKind::Mesh(mesh) if mesh.render => {
                if let Some(b) = &mesh.textures[0] {
                    add(b, Source::Mesh);
                }
                let materials = mesh.material.iter().chain(mesh.textures[0].iter());
                for material in materials {
                    for t in material_textures(game, material) {
                        add(&t, Source::Material(material.to_ascii_lowercase()));
                    }
                }
            }
            NodeKind::Emitter(e) => {
                if let Some(t) = &e.texture {
                    add(t, Source::Emitter);
                }
            }
            NodeKind::Light(l) => {
                for t in &l.flare_textures {
                    add(t, Source::Flare);
                }
            }
            _ => {}
        }
    }
    out
}

/// The textures of the material of this name (an MTR), if there is one.
fn material_textures(game: &GameData, name: &str) -> Vec<String> {
    let Some(r) = ResRef::cut(name) else { return Vec::new() };
    let Ok(data) = game.resman.get(&ResKey::new(r, ResType::MTR)) else { return Vec::new() };
    Mtr::parse(&data).textures.into_iter().flatten().collect()
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
    mg_mdl::animations(&model, &load).into_iter().map(|(n, _)| n.to_ascii_lowercase()).collect()
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

    /// A mesh drawn with a material lists the material's textures; an
    /// emitter's and a light's flare are listed too.
    #[test]
    fn materials_emitters_and_flares_name_textures_too() {
        let key = |name: &str| ResKey::from_filename(name).unwrap();
        let mut c = MemContainer::new();
        let mdl = "newmodel mg_fx\nsetsupermodel mg_fx NULL\nclassification effect\n\
             beginmodelgeom mg_fx\nnode dummy mg_fx\n  parent NULL\nendnode\n\
             node trimesh shell\n  parent mg_fx\n  bitmap mg_old\n  materialname mg_mat\n\
             verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
             tverts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
             faces 1\n    0 1 2 1 0 1 2 0\nendnode\n\
             node emitter sparks\n  parent mg_fx\n  texture FX_Spark\nendnode\n\
             node light glow\n  parent mg_fx\n  texturenames 1\n    fx_flare\nendnode\n\
             endmodelgeom mg_fx\ndonemodel mg_fx\n";
        c.insert(key("mg_fx.mdl"), mdl.as_bytes());
        let mtr = "// a material\ntexture0 mg_diffuse\ntexture1 mg_normal\ntexture2 null\n";
        c.insert(key("mg_mat.mtr"), mtr.as_bytes());
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "hak:x", LayerClass::Erf, c);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let found = textures(&game, &Preview::model("mg_fx"));
        let named: Vec<(&str, &Source)> =
            found.iter().map(|t| (t.name.as_str(), &t.source)).collect();
        let material = Source::Material("mg_mat".into());
        assert_eq!(
            named,
            [
                ("fx_flare", &Source::Flare),
                ("fx_spark", &Source::Emitter),
                ("mg_diffuse", &material),
                ("mg_normal", &material),
                ("mg_old", &Source::Mesh),
            ]
        );
    }

    #[test]
    fn an_object_s_animations_are_its_own_and_its_supermodels() {
        let game = game();
        assert_eq!(animations(&game, &Preview::model("mg_base")), ["open", "close", "default"]);
        assert!(animations(&game, &Preview::model("mg_part")).is_empty());
        assert!(animations(&game, &Preview::model("nothing")).is_empty());
    }
}
