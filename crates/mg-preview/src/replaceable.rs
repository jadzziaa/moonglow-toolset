//! The textures a model draws, use by use ([`uses`]: which node, as what,
//! named by what), and where each comes from ([`texture_file`]): what the
//! tileset editor's and Tile Properties' Textures show. And what an
//! object's texture and animation replacements can name: the textures its
//! models are drawn with (their meshes', the materials' the meshes are
//! drawn with, their emitters' and their lights' flares), and the
//! animations its base model plays (its own and its supermodels'). The
//! Visuals page lists them to pick from.

use std::sync::Arc;

use mg_core::{ResRef, ResType};
use mg_image::mtr::{Mtr, SLOTS};
use mg_image::txi::Txi;
use mg_mdl::{Mesh, Model, NodeKind};
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
    /// The TXI of the texture `of` (a mesh's diffuse texture): its
    /// `envmaptexture`, or a water texture's `bumpyshinytexture`. Where it
    /// says `default`, the area's map is drawn (the tileset's `EnvMap`,
    /// else `chrome1`), and `default` is true.
    Txi { of: String, default: bool },
}

/// What a texture is drawn as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// A mesh's texture slot, numbered as an MTR's are (0 diffuse, 1
    /// normal, 2 specular…: [`mg_image::mtr::slot_name`]).
    Slot(usize),
    /// The environment map a mesh reflects.
    Environment,
    /// A particle emitter's particles.
    Particles,
    /// A light's lens flare.
    Flare,
}

/// One use of a texture by a model: a node draws it as something, and
/// something names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureUse {
    /// Lower case, as the model or the material names it.
    pub name: String,
    /// The node that draws it: a mesh, an emitter or a light (in an
    /// emitter's chunk model, `emitter › chunk › node`).
    pub node: String,
    pub role: Role,
    pub source: Source,
    /// The material (an MTR) the mesh is drawn with, if any.
    pub material: Option<String>,
    /// Whether it is drawn: not a mesh's own texture that its material's
    /// in the same slot stands in for (the name is still the model's, a
    /// replacement's to name).
    pub drawn: bool,
}

/// A name a model or a material gives, lower case; `None` for none
/// (blank, `null`).
fn clean(name: Option<&str>) -> Option<String> {
    let name = name?.trim().to_ascii_lowercase();
    (!name.is_empty() && name != "null").then_some(name)
}

/// How deep [`uses`] follows emitters' chunk models into theirs.
const CHUNK_DEPTH: u8 = 2;

/// Between an emitter, the chunk model it throws and that model's node,
/// in a use's [`TextureUse::node`].
pub const CHUNK_SEP: &str = " › ";

/// Every texture `model` draws, use by use, in the order of its nodes:
/// its meshes' (each slot as the renderer binds it: the mesh's `bitmap`
/// and `texture1`–`texture3`, then the material it is drawn with, the
/// MTR its `materialname` names, else one named like its bitmap, slot by
/// slot over them), the environment maps their diffuse textures' TXIs
/// name, its emitters' and the models they throw, and its lights' flares.
/// Meshes it doesn't draw (`render 0`) are left out. `area_env` is what a
/// TXI's `default` draws: the area's environment map (the tileset's
/// `EnvMap`; `chrome1` where `None`).
pub fn uses(game: &GameData, model: &Model, area_env: Option<&str>) -> Vec<TextureUse> {
    let mut out = Vec::new();
    walk(game, model, "", area_env, 0, &mut out);
    out
}

fn walk(
    game: &GameData,
    model: &Model,
    prefix: &str,
    area_env: Option<&str>,
    depth: u8,
    out: &mut Vec<TextureUse>,
) {
    let mut push = |u: TextureUse| {
        if !out.iter().any(|o| o.name == u.name && o.node == u.node && o.role == u.role) {
            out.push(u);
        }
    };
    for n in &model.nodes {
        let node = format!("{prefix}{}", n.name);
        match &n.kind {
            NodeKind::Mesh(mesh) if mesh.render => {
                for u in mesh_uses(game, mesh, &node, area_env) {
                    push(u);
                }
            }
            NodeKind::Emitter(e) => {
                if let Some(name) = clean(e.texture.as_deref()) {
                    let source = Source::Emitter;
                    let role = Role::Particles;
                    push(TextureUse {
                        name,
                        node: node.clone(),
                        role,
                        source,
                        material: None,
                        drawn: true,
                    });
                }
                if let Some(chunk) = clean(e.chunk.as_deref())
                    && depth < CHUNK_DEPTH
                    && let Some(m) = (Lookup { game }).model(&chunk)
                {
                    let mut inner = Vec::new();
                    walk(
                        game,
                        &m,
                        &format!("{node}{CHUNK_SEP}{chunk}{CHUNK_SEP}"),
                        area_env,
                        depth + 1,
                        &mut inner,
                    );
                    for u in inner {
                        push(u);
                    }
                }
            }
            NodeKind::Light(l) => {
                for t in &l.flare_textures {
                    if let Some(name) = clean(Some(t)) {
                        let (role, source) = (Role::Flare, Source::Flare);
                        push(TextureUse {
                            name,
                            node: node.clone(),
                            role,
                            source,
                            material: None,
                            drawn: true,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

/// A drawn mesh's textures, slot by slot as the renderer binds them
/// (`Renderer::slots`), then the environment map its diffuse texture's
/// TXI names (`load_texture`).
fn mesh_uses(game: &GameData, mesh: &Mesh, node: &str, area_env: Option<&str>) -> Vec<TextureUse> {
    let bitmap = clean(mesh.textures[0].as_deref());
    // The material: the one `materialname` names, else one named like
    // the bitmap.
    let found = |name: Option<String>| name.and_then(|n| Some((material(game, &n)?, n)));
    let mtr = found(clean(mesh.material.as_deref())).or_else(|| found(bitmap.clone()));
    let material = mtr.as_ref().map(|(_, n)| n.clone());
    // (The mesh's own: its bitmap, then `texture1`–`texture3`.)
    let own: [Option<String>; 4] = std::array::from_fn(|i| {
        if i == 0 { bitmap.clone() } else { clean(mesh.textures[i].as_deref()) }
    });
    let theirs = |i: usize| mtr.as_ref().and_then(|(m, _)| clean(m.textures[i].as_deref()));
    let mut out = Vec::new();
    let mut diffuse = None;
    for i in 0..SLOTS {
        let mine = own.get(i).cloned().flatten();
        let other = theirs(i);
        let use_of = |name: String, source: Source, drawn: bool| TextureUse {
            name,
            node: node.to_string(),
            role: Role::Slot(i),
            source,
            material: material.clone(),
            drawn,
        };
        // (A material's texture in the slot stands in for the mesh's own;
        // the mesh's that is the material's is the mesh's.)
        if let Some(name) = &mine {
            let drawn = other.as_ref().is_none_or(|o| o == name);
            out.push(use_of(name.clone(), Source::Mesh, drawn));
        }
        if let Some(name) = other.filter(|o| mine.as_ref() != Some(o)) {
            let m = material.clone().unwrap_or_default();
            out.push(use_of(name, Source::Material(m), true));
        }
        if i == 0 {
            diffuse = out.iter().find(|u| u.drawn).map(|u| u.name.clone());
        }
    }
    // The environment map its diffuse texture's TXI names.
    if let Some(of) = diffuse
        && let Some(txi) = txi(game, &of)
    {
        let named = txi.envmap().or_else(|| txi.water().then(|| txi.bumpy_shiny()).flatten());
        if let Some(env) = clean(named) {
            let default = env == "default";
            let name =
                if default { clean(area_env).unwrap_or_else(|| "chrome1".into()) } else { env };
            out.push(TextureUse {
                name,
                node: node.to_string(),
                role: Role::Environment,
                source: Source::Txi { of, default },
                material,
                drawn: true,
            });
        }
    }
    out
}

/// The material (an MTR) of this name, if there is one.
fn material(game: &GameData, name: &str) -> Option<Mtr> {
    let r = ResRef::cut(name)?;
    game.resman.get(&ResKey::new(r, ResType::MTR)).ok().map(|d| Mtr::parse(&d))
}

/// The TXI of this texture, if there is one.
fn txi(game: &GameData, name: &str) -> Option<Txi> {
    let r = ResRef::cut(name)?;
    game.resman.get(&ResKey::new(r, ResType::TXI)).ok().map(|d| Txi::parse(&d))
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

/// The textures a model names that a replacement can name, each once with
/// what names it first: the meshes it draws (their `bitmap`, whether
/// drawn or stood in for by their material's, and the textures of the
/// materials they are drawn with), its emitters' and its lights' flares;
/// not the environment maps TXIs name.
fn named(game: &GameData, m: &Model) -> Vec<(String, Source)> {
    let mut out: Vec<(String, Source)> = Vec::new();
    for u in uses(game, m, None) {
        // (Not a chunk model's an emitter throws: whether a replacement
        // reaches it isn't known.)
        let theirs = match u.source {
            _ if u.node.contains(CHUNK_SEP) => false,
            Source::Txi { .. } => false,
            Source::Mesh => u.role == Role::Slot(0),
            _ => true,
        };
        if theirs && !out.iter().any(|(n, _)| *n == u.name) {
            out.push((u.name, u.source));
        }
    }
    out
}

/// Where a texture comes from in the load order, and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureFile {
    /// DDS, TGA or PLT: what the game draws for the name (a DDS over a
    /// TGA within a layer class, a PLT only where neither is).
    pub restype: ResType,
    /// The label of the layer it is read from (`hak:name`, `override`,
    /// `module`, `key:nwn_base`…).
    pub layer: String,
    pub width: u32,
    pub height: u32,
    /// How its pixels are kept: `DXT1`, `DXT5`, `uncompressed` (DDS),
    /// `24-bit` (TGA), `PLT`.
    pub format: String,
    /// The file's size.
    pub bytes: usize,
    /// A cube map, six files `name0`…`name5` (the rest is the first's).
    pub cube: bool,
    /// The layer of its TXI, where it has one.
    pub txi: Option<String>,
    /// Why the file can't be read as an image, where it can't.
    pub problem: Option<String>,
}

/// Where the texture a model names `name` comes from, as the game finds
/// it (cut to 16 characters, as the game cuts a longer name); `None`
/// where the load order has none of the name: missing.
pub fn texture_file(game: &GameData, name: &str) -> Option<TextureFile> {
    let rm = &game.resman;
    let r = ResRef::cut(name)?;
    let label = |i: usize| rm.layers()[i].label.clone();
    let txi_layer = rm.find(&ResKey::new(r, ResType::TXI)).map(label);
    if let Some((restype, i)) = rm.texture_layer(r) {
        let data = rm.get_in(i, &ResKey::new(r, restype)).ok();
        let bytes = data.as_ref().map_or(0, |d| d.len());
        let read = data.as_deref().map(|d| (mg_image::read(restype, d), d));
        let mut f = TextureFile {
            restype,
            layer: label(i),
            width: 0,
            height: 0,
            format: String::new(),
            bytes,
            cube: false,
            txi: txi_layer,
            problem: None,
        };
        match read {
            Some((Ok(t), d)) => {
                (f.width, f.height) = (t.width, t.height);
                f.format = format_name(restype, t.format, d);
            }
            Some((Err(e), _)) => f.problem = Some(e.to_string()),
            None => f.problem = Some("it can't be read".into()),
        }
        return Some(f);
    }
    let plt = ResKey::new(r, ResType::PLT);
    if let Some(i) = rm.find(&plt) {
        let data = rm.get_in(i, &plt).ok();
        let read = data.as_deref().map(mg_image::plt::Plt::read);
        let (width, height, problem) = match read {
            Some(Ok(p)) => (p.width, p.height, None),
            Some(Err(e)) => (0, 0, Some(e.to_string())),
            None => (0, 0, Some("it can't be read".into())),
        };
        return Some(TextureFile {
            restype: ResType::PLT,
            layer: label(i),
            width,
            height,
            format: "PLT".into(),
            bytes: data.map_or(0, |d| d.len()),
            cube: false,
            txi: txi_layer,
            problem,
        });
    }
    // A cube map: its TXI says so, and its faces are `name0`…`name5`.
    if txi(game, name).is_some_and(|t| t.cube()) && name.len() < ResRef::MAX_LEN {
        let mut face = texture_file(game, &format!("{name}0"))?;
        face.cube = true;
        face.txi = txi_layer;
        let face_type =
            |k: u32| ResRef::cut(&format!("{name}{k}")).and_then(|r| rm.texture_type(r));
        if let Some(gone) = (1..6).find(|&k| face_type(k).is_none()) {
            face.problem = Some(format!("its face {name}{gone} is found nowhere"));
        }
        return Some(face);
    }
    None
}

/// How a texture's pixels are kept, in a builder's words.
fn format_name(restype: ResType, format: mg_image::Format, data: &[u8]) -> String {
    use mg_image::Format;
    match (restype, format) {
        (ResType::TGA, _) => format!("{}-bit", data.get(16).copied().unwrap_or(0)),
        (_, Format::Bc1) => "DXT1".into(),
        (_, Format::Bc2) => "DXT3".into(),
        (_, Format::Bc3) => "DXT5".into(),
        (_, Format::Bc4) => "BC4".into(),
        (_, Format::Bc5) => "BC5".into(),
        (_, Format::Rgba8) => "uncompressed".into(),
    }
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
             node emitter sparks\n  parent mg_fx\n  texture FX_Spark\n  chunkname mg_rock\nendnode\n\
             node light glow\n  parent mg_fx\n  texturenames 1\n    fx_flare\nendnode\n\
             endmodelgeom mg_fx\ndonemodel mg_fx\n";
        c.insert(key("mg_fx.mdl"), mdl.as_bytes());
        // The model the emitter throws.
        let rock = model("mg_rock", "NULL", &["mg_stone"], &[]);
        c.insert(key("mg_rock.mdl"), rock.as_bytes());
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
        // The thrown model's texture is drawn, under the emitter's node,
        // but not offered as a replacement (whether one reaches it isn't
        // known).
        let m = Lookup { game: &game }.model("mg_fx").unwrap();
        let stone = uses(&game, &m, None).into_iter().find(|u| u.name == "mg_stone").unwrap();
        assert_eq!(stone.node, format!("sparks{CHUNK_SEP}mg_rock{CHUNK_SEP}m0"));
        assert_eq!((stone.role, stone.drawn), (Role::Slot(0), true));
    }

    /// A BioWare DDS of `side`×`side`, DXT1 (3 channels) or DXT5 (4).
    fn dds(side: u32, channels: u32) -> Vec<u8> {
        let format = if channels == 3 { mg_image::Format::Bc1 } else { mg_image::Format::Bc3 };
        let size = format.level_size(side, side);
        let mut f = Vec::new();
        for v in [side, side, channels, size as u32] {
            f.extend_from_slice(&v.to_le_bytes());
        }
        f.extend_from_slice(&1.0f32.to_le_bytes());
        f.extend(std::iter::repeat_n(0, size));
        f
    }

    /// A tile's model use by use: a mesh drawn with its own bitmap (and a
    /// `texture1` of its own) whose TXI names the area's environment map,
    /// one whose `materialname` names an MTR (its slots over the mesh's,
    /// its bitmap stood in for), one whose bitmap names an MTR of the same
    /// name, one not drawn, a light's flare; and each texture's file: its
    /// layer, format and size, a PLT, a TGA, and a name found nowhere.
    #[test]
    fn a_model_s_textures_use_by_use_and_where_they_come_from() {
        let key = |name: &str| ResKey::from_filename(name).unwrap();
        let mesh = |name: &str, lines: &str| {
            format!(
                "node trimesh {name}\n  parent mg_tile\n{lines}\
                 verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
                 tverts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
                 faces 1\n    0 1 2 1 0 1 2 0\nendnode\n"
            )
        };
        let mdl = format!(
            "newmodel mg_tile\nsetsupermodel mg_tile NULL\nclassification tile\n\
             beginmodelgeom mg_tile\nnode dummy mg_tile\n  parent NULL\nendnode\n{}{}{}{}\
             node light mg_tileml1\n  parent mg_tile\n  texturenames 1\n    fx_flare\nendnode\n\
             endmodelgeom mg_tile\ndonemodel mg_tile\n",
            mesh("floor", "  bitmap MG_Floor\n  texture1 mg_floor_n\n"),
            mesh("wall", "  bitmap mg_old\n  materialname mg_wall\n"),
            mesh("roof", "  bitmap mg_roof\n"),
            mesh("hidden", "  bitmap mg_unseen\n  render 0\n"),
        );
        let mut hak = MemContainer::new();
        hak.insert(key("mg_tile.mdl"), mdl.as_bytes());
        hak.insert(key("mg_floor.dds"), &dds(64, 3)[..]);
        hak.insert(key("mg_floor.txi"), &b"envmaptexture default\n"[..]);
        let wall = "texture0 mg_brick\ntexture1 mg_brick_n\ntexture2 mg_brick_s\ntexture3 null\n";
        hak.insert(key("mg_wall.mtr"), wall.as_bytes());
        hak.insert(key("mg_brick.dds"), &dds(256, 4)[..]);
        hak.insert(key("mg_brick_n.dds"), &dds(256, 4)[..]);
        // (Named like the bitmap: its texture0 is the bitmap's own.)
        hak.insert(key("mg_roof.mtr"), &b"texture0 mg_roof\ntexture1 mg_roof_n\n"[..]);
        hak.insert(key("mg_roof.plt"), &plt(8)[..]);
        let mut folder = MemContainer::new();
        let mut tga = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 2, 0, 24, 0];
        tga.extend([0; 12]);
        folder.insert(key("mg_floor_n.tga"), &tga[..]);
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "hak:x", LayerClass::Erf, hak);
        rm.add(priority::OVERRIDE, "override", LayerClass::Directory, folder);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let model = Lookup { game: &game }.model("mg_tile").unwrap();
        let found = uses(&game, &model, Some("mg_ref01"));
        let seen: Vec<(&str, &str, Role, bool)> =
            found.iter().map(|u| (u.name.as_str(), u.node.as_str(), u.role, u.drawn)).collect();
        assert_eq!(
            seen,
            [
                ("mg_floor", "floor", Role::Slot(0), true),
                ("mg_floor_n", "floor", Role::Slot(1), true),
                ("mg_ref01", "floor", Role::Environment, true),
                ("mg_old", "wall", Role::Slot(0), false),
                ("mg_brick", "wall", Role::Slot(0), true),
                ("mg_brick_n", "wall", Role::Slot(1), true),
                ("mg_brick_s", "wall", Role::Slot(2), true),
                ("mg_roof", "roof", Role::Slot(0), true),
                ("mg_roof_n", "roof", Role::Slot(1), true),
                ("fx_flare", "mg_tileml1", Role::Flare, true),
            ]
        );
        let wall = Source::Material("mg_wall".into());
        assert_eq!(found[3].source, Source::Mesh);
        assert_eq!((&found[4].source, found[4].material.as_deref()), (&wall, Some("mg_wall")));
        let env = Source::Txi { of: "mg_floor".into(), default: true };
        assert_eq!(found[2].source, env);
        assert_eq!(found[1].material, None);
        // The roof's bitmap is its material's texture0: the mesh's, drawn
        // through the material.
        assert_eq!(
            (&found[7].source, found[7].material.as_deref()),
            (&Source::Mesh, Some("mg_roof"))
        );
        // Without an area, `default` is `chrome1`.
        let bare = uses(&game, &model, None);
        assert_eq!(bare[2].name, "chrome1");

        // Where each comes from.
        let floor = texture_file(&game, "mg_floor").unwrap();
        assert_eq!((floor.restype, floor.layer.as_str()), (ResType::DDS, "hak:x"));
        assert_eq!((floor.width, floor.height, floor.format.as_str()), (64, 64, "DXT1"));
        assert_eq!((floor.bytes, floor.txi.as_deref()), (dds(64, 3).len(), Some("hak:x")));
        let brick = texture_file(&game, "MG_BRICK").unwrap();
        assert_eq!((brick.format.as_str(), brick.width, brick.txi), ("DXT5", 256, None));
        let normal = texture_file(&game, "mg_floor_n").unwrap();
        assert_eq!((normal.restype, normal.layer.as_str()), (ResType::TGA, "override"));
        assert_eq!((normal.width, normal.format.as_str(), normal.problem), (2, "24-bit", None));
        let roof = texture_file(&game, "mg_roof").unwrap();
        assert_eq!((roof.restype, roof.format.as_str(), roof.width), (ResType::PLT, "PLT", 8));
        // Missing: found nowhere.
        for gone in ["mg_brick_s", "mg_roof_n", "fx_flare", "mg_ref01"] {
            assert_eq!(texture_file(&game, gone), None, "{gone}");
        }
        // The Visuals page's list from the same walk: the bitmap stood in
        // for, as the model names it; the mesh's own `texture1` and the
        // TXI's map left out.
        let named: Vec<String> = named(&game, &model).into_iter().map(|(n, _)| n).collect();
        assert_eq!(
            named,
            [
                "mg_floor",
                "mg_old",
                "mg_brick",
                "mg_brick_n",
                "mg_brick_s",
                "mg_roof",
                "mg_roof_n",
                "fx_flare"
            ]
        );
    }

    /// A cube map: its TXI says `cube 1`, its six faces `name0`…`name5`.
    #[test]
    fn a_cube_map_is_found_by_its_faces() {
        let key = |name: &str| ResKey::from_filename(name).unwrap();
        let mut c = MemContainer::new();
        c.insert(key("mg_sky.txi"), &b"cube 1\n"[..]);
        for k in 0..6 {
            c.insert(key(&format!("mg_sky{k}.dds")), &dds(32, 3)[..]);
        }
        c.insert(key("mg_half.txi"), &b"cube 1\n"[..]);
        c.insert(key("mg_half0.dds"), &dds(32, 3)[..]);
        // (A file cut short: said, not read.)
        c.insert(key("mg_short.dds"), &dds(32, 3)[..30]);
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "hak:x", LayerClass::Erf, c);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let sky = texture_file(&game, "mg_sky").unwrap();
        assert!(sky.cube);
        assert_eq!((sky.width, sky.txi.as_deref(), sky.problem), (32, Some("hak:x"), None));
        let half = texture_file(&game, "mg_half").unwrap();
        assert_eq!(half.problem.as_deref(), Some("its face mg_half1 is found nowhere"));
        let short = texture_file(&game, "mg_short").unwrap();
        assert_eq!((short.restype, short.width, short.bytes), (ResType::DDS, 0, 30));
        assert!(short.problem.is_some());
    }

    /// A PLT of `side`×`side`.
    fn plt(side: u32) -> Vec<u8> {
        let mut f = b"PLT V1  ".to_vec();
        f.extend([0; 8]);
        f.extend(side.to_le_bytes());
        f.extend(side.to_le_bytes());
        f.extend(std::iter::repeat_n(0, (side * side * 2) as usize));
        f
    }

    #[test]
    fn an_object_s_animations_are_its_own_and_its_supermodels() {
        let game = game();
        assert_eq!(animations(&game, &Preview::model("mg_base")), ["open", "close", "default"]);
        assert!(animations(&game, &Preview::model("mg_part")).is_empty());
        assert!(animations(&game, &Preview::model("nothing")).is_empty());
    }
}
