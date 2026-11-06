//! Previews of blueprints: the models the game shows for a creature, an
//! item, a placeable or a door, assembled from the blueprint and the rules
//! 2DAs, with their PLT colours and environment maps
//! (`docs/research/notes_models.md` part D).
//!
//! A [`Preview`] names models and where they hang; `compose::Composed`
//! loads them and turns them into renderer instances at a moment of an
//! animation. The renderer's half (`compose`, with mg-render and wgpu) is
//! the feature `render`, on by default: what a preview is made of and the
//! files it loads (`files`, `replaceable`) need no GPU, and `mg` builds
//! with them alone.

use std::collections::BTreeMap;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use mg_2da::TwoDa;
use mg_core::{ResRef, ResType};
use mg_gff::Struct;
use mg_mdl::{Model, NodeKind};
use mg_rules::GameData;

#[cfg(feature = "render")]
pub mod compose;
mod creature;
pub mod files;
mod item;
mod object;
pub mod replaceable;
#[cfg(feature = "usage")]
pub mod usage;

pub use creature::{CreatureLook, creature, creature_look};
pub use item::{armor_part, ground_turn, item, item_on, item_placed};
pub use object::{PLACEABLE_DEFAULT_ENV_MAP, door, placeable, sound, store, waypoint};

/// Why a preview could not be made.
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("{0}")]
    Rules(#[from] mg_rules::RulesError),
    #[error("{table}.2da has no row {row}")]
    NoRow { table: &'static str, row: i64 },
    #[error("no model {0}")]
    NoModel(String),
    /// The object's appearance names no usable model
    /// ([`mg_rules::appearance`], as Verify and the Properties judge it).
    #[error("{0}")]
    Appearance(#[from] mg_rules::appearance::ModelProblem),
}

/// The model an object's appearance names, by [`mg_rules::appearance`]
/// (`None`: a body of parts, or an item, which name none of their own),
/// and the row it is read from.
fn appearance(
    game: &GameData,
    kind: mg_rules::ObjectKind,
    object: &Struct,
) -> Result<(usize, Option<String>), PreviewError> {
    let at = mg_rules::appearance::AppearanceRow::of(kind, object)
        .ok_or_else(|| PreviewError::NoModel(format!("{kind:?}")))?;
    let table = game.table(at.table)?;
    let model = at.model(Some(&table), &game.resman)?;
    // (A row that is there is no less than 0.)
    Ok((at.row as usize, model))
}

/// A model in a preview.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// The model's resource name, lower case.
    pub model: String,
    /// The base model's node it hangs from (lower case); `None`: the base's
    /// origin.
    pub attach: Option<String>,
    pub scale: f32,
    /// Textures to use instead of the model's (lower case, from → to).
    pub textures: BTreeMap<String, String>,
    /// PLT colours by layer: skin, hair, metal 1 and 2, cloth 1 and 2,
    /// leather 1 and 2, tattoo 1 and 2.
    pub colors: Option<[u8; 10]>,
    /// The object's environment map (texture alpha is reflectivity); `None`:
    /// only what its textures ask for.
    pub env_map: Option<String>,
    /// `env_map` is what the object's own `default` comes to (a
    /// placeable's `Reflection` `default`: `chrome1`), which a texture's
    /// `envmaptexture default` passes over for the area's map
    /// (`mg_render::Instance::env_map_default`).
    pub env_map_default: bool,
    /// Values its materials' programs take in place of their MTRs'
    /// parameters (`mg_render::Instance::material_values`): a placeable's
    /// tint ([`tinted`]).
    pub material_values: Vec<mg_image::mtr::MaterialValue>,
    /// Plays the animation on its own nodes: skinned robes and cloaks (their
    /// bones copy the skeleton's), wings and tails (with their own
    /// animations).
    pub animated: bool,
    /// Its own visual transform, where the object has one for it (a
    /// placed creature's head, tail, wings or cloak: [`scope_matrix`]):
    /// the model placed so within the node it hangs from, as the whole
    /// object's is placed within the object ([`visual_matrix`]).
    pub local: Option<Mat4>,
    /// The model of an item a creature wears or holds (a helmet, a cloak,
    /// a weapon or shield), not of its body: the creature's texture
    /// replacements don't reach it ([`replaced`]).
    pub item: bool,
}

impl Part {
    fn new(model: impl Into<String>) -> Part {
        Part {
            model: model.into().to_ascii_lowercase(),
            attach: None,
            scale: 1.0,
            textures: BTreeMap::new(),
            colors: None,
            env_map: None,
            env_map_default: false,
            material_values: Vec::new(),
            animated: false,
            local: None,
            item: false,
        }
    }
}

/// A visual transform as the game client draws it (`visual_transforms_look`
/// in `client_render.rs`): the model scaled by `ScaleX` every way (`scale`);
/// leaned about its Y by `RotateZ`, then tipped about its X by `RotateY`,
/// then turned about its upright axis by `RotateX` (`rotate`, degrees);
/// then moved by the translation (meters).
pub fn visual_matrix(scale: f32, rotate: [f32; 3], translate: [f32; 3]) -> Mat4 {
    Mat4::from_translation(Vec3::from(translate))
        * visual_turn(rotate)
        * Mat4::from_scale(Vec3::splat(scale))
}

/// The rotation alone of [`visual_matrix`].
pub fn visual_turn(rotate: [f32; 3]) -> Mat4 {
    let r = Vec3::from(rotate) * (std::f32::consts::PI / 180.0);
    Mat4::from_rotation_z(r.x) * Mat4::from_rotation_x(r.y) * Mat4::from_rotation_y(r.z)
}

/// The visual transform of scope `scope` (`VisTransformList`'s entry: a
/// creature's head 254, tail 253, wings 252, cloak 243) of the placed
/// object `object`, as a matrix ([`visual_matrix`], each axis's goal,
/// `ValueTo`); `None` without one, or with one that changes nothing.
pub fn scope_matrix(object: &Struct, scope: i64) -> Option<Mat4> {
    let list = object.list("VisTransformList")?;
    let entry = list.iter().find(|e| e.integer("Scope").unwrap_or(0) == scope)?;
    let axis = |label: &str, rest: f32| {
        entry.child(label).and_then(|a| a.float("ValueTo")).unwrap_or(rest)
    };
    let three =
        |prefix: &str, rest: f32| ["X", "Y", "Z"].map(|a| axis(&format!("{prefix}{a}"), rest));
    let (scale, rotate, translate) =
        (axis("ScaleX", 1.0), three("Rotate", 0.0), three("Translate", 0.0));
    let none = scale == 1.0 && rotate == [0.0; 3] && translate == [0.0; 3];
    (!none).then(|| visual_matrix(scale, rotate, translate))
}

/// A light a preview carries (a placeable's).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewLight {
    /// From the base's origin, metres.
    pub offset: Vec3,
    /// lightcolor.2da values.
    pub color: Vec3,
    pub radius: f32,
    /// Whether it shines: a placeable's **Illuminated** (`LightState`);
    /// one off is still its light, shown as such in the area view.
    pub on: bool,
}

/// What to draw for a blueprint.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    /// The base model: a creature's skeleton or body, the item, placeable or
    /// door.
    pub base: Part,
    pub parts: Vec<Part>,
    /// The animation it stands in (`pause1`, `cpause1`, a placeable's
    /// state).
    pub idle: Option<String>,
    pub lights: Vec<PreviewLight>,
}

impl Preview {
    /// A model on its own.
    pub fn model(name: &str) -> Preview {
        Preview { base: Part::new(name), parts: Vec::new(), idle: None, lights: Vec::new() }
    }
}

/// An object's texture and animation replacements (EE's
/// `ReplaceObjectTexture` and `ReplaceObjectAnimation`, saved as
/// `TextureReplace` and `AnimationReplace`) applied to its preview: each
/// model's texture of an old name drawn with the new one, a PLT's too
/// (drawn plainly, as the client draws a creature's head replaced), and
/// the idle animation replaced. A creature's reach its body, not what it
/// wears or holds ([`Part::item`]: in the client neither the field nor
/// the script call replaces its helmet's or cloak's,
/// `batch6_replaced_textures` in `client_render.rs`, 2026-11-01).
/// `mg-corpus-tests/tests/engine_ee_fields.rs` checks the engine reads
/// them.
pub fn replaced(mut p: Preview, object: &Struct) -> Preview {
    let pairs = |outer: &str, list: &str, old: &str, new: &str| -> Vec<(String, String)> {
        let Some(items) = object.child(outer).and_then(|s| s.list(list)) else { return Vec::new() };
        items
            .iter()
            .filter_map(|e| {
                let (o, n) = (e.resref(old)?, e.resref(new)?);
                (!o.is_empty())
                    .then(|| (o.to_lowercase().to_string(), n.to_lowercase().to_string()))
            })
            .collect()
    };
    let textures = pairs("TextureReplace", "TextureReplaceLi", "OldTexture", "NewTexture");
    for part in std::iter::once(&mut p.base).chain(p.parts.iter_mut().filter(|p| !p.item)) {
        for (old, new) in &textures {
            // An empty new name restores the original.
            if !new.is_empty() {
                part.textures.entry(old.clone()).or_insert_with(|| new.clone());
            }
        }
    }
    let animations = pairs("AnimationReplace", "AnimationReplace", "OldAnimation", "NewAnimation");
    if let Some(idle) = &p.idle
        && let Some((_, new)) = animations.iter().find(|(old, _)| old.eq_ignore_ascii_case(idle))
        && !new.is_empty()
    {
        p.idle = Some(new.clone());
    }
    p
}

/// A placeable with the tint its locals give (The Domains of Dread's
/// colorways, [`mg_rules::tint`]), as its module's script sets it when
/// the module loads: `tdod_tint` on every material whose MTR declares
/// `tdodTint`, then `tdod_tint_<mtr>` on that MTR, at full strength. Not a
/// static one: the client bakes it into the tiles as made, untinted (seen
/// in the game client, The Domains of Dread's trial of 2026-11-06).
pub fn tinted(mut p: Preview, utp: &Struct) -> Preview {
    use mg_rules::tint;
    if !tint::takes_tint(utp) {
        return p;
    }
    let tints = tint::of_object(utp);
    let value = |material: Option<String>, color| mg_image::mtr::MaterialValue {
        material,
        name: tint::PARAMETER.to_string(),
        value: mg_image::mtr::Param::Float(tint::uniform(color).to_vec()),
    };
    let values: Vec<_> = tints
        .all
        .map(|c| value(None, c))
        .into_iter()
        .chain(tints.by_material.into_iter().map(|(m, c)| value(Some(m), c)))
        .collect();
    p.base.material_values.extend(values);
    p
}

/// The materials of a placeable's model that take a tint (The Domains of
/// Dread's colorways, [`mg_rules::tint`]): the MTRs its meshes draw with
/// (the one a mesh names, else one named as its texture, as the renderer
/// takes them) that declare `tdodTint`, lower case, each once.
pub fn tintable_materials(game: &GameData, utp: &Struct) -> Vec<String> {
    let Ok(p) = placeable(game, utp) else { return Vec::new() };
    let Some(model) =
        game.resman.get_named(&p.base.model, ResType::MDL).ok().and_then(|d| Model::read(&d).ok())
    else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    for node in &model.nodes {
        let Some(mesh) = node.mesh().filter(|m| m.render) else { continue };
        let names = [mesh.material.as_deref(), mesh.textures[0].as_deref()];
        let Some((name, mtr)) = names.into_iter().flatten().find_map(|n| {
            let data = game.resman.get_named(n, ResType::MTR).ok()?;
            Some((n.to_ascii_lowercase(), mg_image::mtr::Mtr::parse(&data)))
        }) else {
            continue;
        };
        let tints =
            mtr.params.iter().any(|(n, _)| n.eq_ignore_ascii_case(mg_rules::tint::PARAMETER));
        if tints && !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// Where [`of_object`] shows an object, which tells how an item looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// On its own (a blueprint's view, a picture of it): an item as
    /// [`item_on`] makes it, armor and cloaks on a woman where `female`.
    Alone { female: bool },
    /// Placed in an area: an item as it lies there ([`item_placed`]).
    Placed,
}

impl Default for Shown {
    fn default() -> Shown {
        Shown::Alone { female: false }
    }
}

/// What to draw for an object of `kind` (a blueprint's fields, or a
/// placed object's entry in its area's GIT), with its texture and
/// animation replacements ([`replaced`]): a creature (the items it wears
/// read through `items`), door, item (as `shown`), placeable, waypoint,
/// or a store's or sound's marker. `None` for a kind of no model (an
/// encounter, a trigger). The one way the area view, a blueprint's view
/// and `mg model-files` make an object's preview.
pub fn of_object(
    game: &GameData,
    kind: mg_rules::ObjectKind,
    object: &Struct,
    items: &dyn Fn(ResRef) -> Option<mg_gff::Gff>,
    shown: Shown,
) -> Option<Result<Preview, PreviewError>> {
    use mg_rules::ObjectKind as K;
    let preview = match kind {
        K::Creature => creature(game, object, items),
        K::Door => door(game, object),
        K::Item => match shown {
            Shown::Alone { female } => item_on(game, object, female),
            Shown::Placed => item_placed(game, object),
        },
        K::Placeable => placeable(game, object),
        K::Waypoint => waypoint(game, object),
        K::Store => store(game),
        K::Sound => sound(game, object),
        K::Encounter | K::Trigger => return None,
    };
    Some(preview.map(|p| {
        let p = replaced(p, object);
        // (A static placeable is baked into the tiles as made: nothing set
        // on it shows in the client, The Domains of Dread's trial of
        // 2026-11-06.)
        if kind == K::Placeable && !mg_rules::tint::takes_tint(object) {
            return p;
        }
        let p = shader_params(p, object);
        if kind == K::Placeable { tinted(p, object) } else { p }
    }))
}

/// The preview with the shader parameters saved on the object
/// (`Material` › `ShaderParams`: what `SetMaterialShaderUniformInt` and
/// `…Vec4` set, which the game reads back as it loads the object,
/// `engine_ee_fields.rs`) on its materials, by MTR name. A creature's
/// reach its body, not what it wears or holds, as its texture
/// replacements ([`replaced`]).
pub fn shader_params(mut p: Preview, object: &Struct) -> Preview {
    let Some(params) = object.child("Material").and_then(|m| m.list("ShaderParams")) else {
        return p;
    };
    let text = |s: &Struct, l: &str| {
        String::from_utf8_lossy(s.string(l).unwrap_or_default()).trim().to_string()
    };
    let values: Vec<mg_image::mtr::MaterialValue> = params
        .iter()
        .filter_map(|e| {
            let (material, name) = (text(e, "Material"), text(e, "Param"));
            let value = match e.integer("Type")? {
                1 => mg_image::mtr::Param::Int(e.integer("Int").unwrap_or(0) as i32),
                2 => mg_image::mtr::Param::Float(
                    (1..=4).map(|k| e.float(&format!("Float{k}")).unwrap_or(0.0)).collect(),
                ),
                _ => return None,
            };
            (!material.is_empty() && !name.is_empty()).then(|| mg_image::mtr::MaterialValue {
                material: Some(material.to_ascii_lowercase()),
                name,
                value,
            })
        })
        .collect();
    for part in std::iter::once(&mut p.base).chain(p.parts.iter_mut().filter(|p| !p.item)) {
        part.material_values.extend(values.iter().cloned());
    }
    p
}

/// PLT layers.
const SKIN: usize = 0;
const HAIR: usize = 1;
const TATTOO1: usize = 8;
const TATTOO2: usize = 9;

/// An item's colours in PLT layer order (metal, cloth, leather), over
/// `colors`.
fn item_colors(item: &Struct, mut colors: [u8; 10]) -> [u8; 10] {
    for channel in mg_rules::items::ArmorChannel::ALL {
        if let Some(v) = item.integer(channel.field()) {
            colors[channel.plt_layer()] = v.clamp(0, 255) as u8;
        }
    }
    colors
}

/// A 2DA cell, trimmed; `None` for `****` or blank.
fn cell<'a>(t: &'a TwoDa, row: usize, column: &str) -> Option<&'a str> {
    t.get(row, column).map(str::trim).filter(|v| !v.is_empty() && *v != "****")
}

fn cell_f32(t: &TwoDa, row: usize, column: &str) -> Option<f32> {
    cell(t, row, column)?.parse().ok()
}

fn cell_int(t: &TwoDa, row: usize, column: &str) -> Option<i64> {
    let v = cell(t, row, column)?;
    v.parse::<i64>().ok().or_else(|| v.parse::<f32>().ok().map(|f| f as i64))
}

/// An environment map column: `default` (the area's map, the tileset's
/// `EnvMap`, which the renderer resolves: `chrome1` where there is no
/// area; read from the client), a texture, or blank or `****`: none.
fn env_map(value: Option<&str>) -> Option<String> {
    value.map(str::to_ascii_lowercase)
}

/// Resources the builders look up.
struct Lookup<'a> {
    game: &'a GameData,
}

impl Lookup<'_> {
    /// Whether the game has the model (by its lists: nothing is read).
    fn has_model(&self, name: &str) -> bool {
        mg_resman::ResKey::parse(name, ResType::MDL).is_some_and(|k| self.game.resman.contains(&k))
    }

    fn model(&self, name: &str) -> Option<Arc<Model>> {
        let data = self.game.resman.get_named(name, ResType::MDL).ok()?;
        Model::read(&data).ok().map(Arc::new)
    }

    /// Whether a texture name resolves to anything (DDS, TGA, PLT, or an
    /// MTR that names a `texture0`), by the game's lists: cut to 16
    /// characters, as the renderer and the game find it (a longer name was
    /// taken for none here, and another texture put in its place). An MTR
    /// with no `texture0` (a part's normal and specular maps alone) gives
    /// the name no image, and the game goes on to the fallbacks (GitHub
    /// issue 37: `pfa0_bicepl001`'s own material, drawn in
    /// `pfh0_bicepl001.plt`).
    fn has_texture(&self, name: &str) -> bool {
        let Some(r) = ResRef::cut(name) else { return false };
        let key = |t| mg_resman::ResKey::new(r, t);
        [ResType::DDS, ResType::TGA, ResType::PLT]
            .into_iter()
            .any(|t| self.game.resman.contains(&key(t)))
            || self
                .game
                .resman
                .get(&key(ResType::MTR))
                .is_ok_and(|d| mg_image::mtr::Mtr::parse(&d).textures[0].is_some())
    }

    /// The meshes' textures (lower case).
    fn bitmaps(&self, model: &str) -> Vec<String> {
        self.model(model).map_or_else(Vec::new, |m| bitmaps(&m))
    }
}

/// Whether a mesh the model draws names no texture.
fn unnamed_mesh(m: &Model) -> bool {
    m.nodes.iter().any(
        |n| matches!(&n.kind, NodeKind::Mesh(mesh) if mesh.render && mesh.textures[0].is_none()),
    )
}

/// The textures of the meshes a model draws (lower case).
fn bitmaps(m: &Model) -> Vec<String> {
    let mut out: Vec<String> = m
        .nodes
        .iter()
        .filter_map(|n| match &n.kind {
            NodeKind::Mesh(mesh) if mesh.render => mesh.textures[0].clone(),
            _ => None,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use mg_gff::Value;

    use super::*;

    fn pair(old: &str, new: &str, labels: (&str, &str)) -> Struct {
        let mut e = Struct::new(0);
        e.set(labels.0, Value::resref(ResRef::from_str(old).unwrap()));
        e.set(labels.1, Value::resref(ResRef::from_str(new).unwrap()));
        e
    }

    /// A scope's entry as a matrix: its goals (`ValueTo`), scaled by
    /// `ScaleX` alone and turned as the whole object's is; none for a scope
    /// the list lacks, or one that changes nothing.
    #[test]
    fn a_scope_s_visual_transform_is_a_matrix() {
        let axis = |v: f32| {
            let mut a = Struct::new(0);
            a.set("ValueTo", Value::Float(v));
            Value::Struct(a)
        };
        let mut head = Struct::new(6);
        head.set("Scope", Value::Int(254));
        head.set("ScaleX", axis(2.0));
        head.set("ScaleZ", axis(5.0));
        head.set("RotateX", axis(90.0));
        head.set("TranslateZ", axis(0.25));
        let mut plain = Struct::new(6);
        plain.set("Scope", Value::Int(253));
        plain.set("ScaleX", axis(1.0));
        let mut o = Struct::new(0);
        assert_eq!(scope_matrix(&o, 254), None);
        o.set("VisTransformList", Value::List(vec![head, plain]));
        let m = scope_matrix(&o, 254).unwrap();
        assert!(m.transform_point3(Vec3::X).abs_diff_eq(Vec3::new(0.0, 2.0, 0.25), 1e-5), "{m}");
        assert!(m.transform_vector3(Vec3::Z).abs_diff_eq(Vec3::Z * 2.0, 1e-5), "ScaleX alone");
        assert_eq!(scope_matrix(&o, 253), None, "changes nothing");
        assert_eq!(scope_matrix(&o, 0), None, "no base entry");
    }

    /// A dynamic placeable's tint locals become its material values, the
    /// object's for every MTR declaring `tdodTint` and a material's own
    /// by its name; a static one's none, as the client draws it.
    #[test]
    fn a_dynamic_placeable_s_tint_locals_reach_its_preview() {
        let var = |name: &str, value: &str| {
            let mut v = Struct::new(0);
            v.set("Name", Value::String(name.into()));
            v.set("Type", Value::Dword(3));
            v.set("Value", Value::String(value.into()));
            v
        };
        let mut utp = Struct::new(0);
        utp.set(
            "VarTable",
            Value::List(vec![var("tdod_tint", "#ff0000"), var("tdod_tint_dd_ctn_bedc", "#00ff00")]),
        );
        let values = tinted(Preview::model("bed"), &utp).base.material_values;
        let value = |m: Option<&str>, v: [f32; 4]| mg_image::mtr::MaterialValue {
            material: m.map(str::to_string),
            name: "tdodTint".into(),
            value: mg_image::mtr::Param::Float(v.to_vec()),
        };
        assert_eq!(
            values,
            [value(None, [1.0, 0.0, 0.0, 1.0]), value(Some("dd_ctn_bedc"), [0.0, 1.0, 0.0, 1.0])]
        );
        utp.set("Static", Value::Byte(1));
        assert_eq!(tinted(Preview::model("bed"), &utp).base.material_values, []);
    }

    /// The shader parameters saved on an object (`Material` ›
    /// `ShaderParams`, as the game writes them) reach its materials by
    /// name, an integer and a vector, before the tint its locals give.
    #[test]
    fn saved_shader_parameters_reach_the_preview() {
        let entry = |param: &str, ty: u8| {
            let mut e = Struct::new(0);
            e.set("Material", Value::String(b"MG_Mat".to_vec()));
            e.set("Param", Value::String(param.into()));
            e.set("Type", Value::Byte(ty));
            e.set("Int", Value::Int(7));
            e.set("Float1", Value::Float(0.5));
            e
        };
        let mut m = Struct::new(7);
        m.set("ShaderParams", Value::List(vec![entry("nOn", 1), entry("vTint", 2), entry("x", 9)]));
        let mut o = Struct::new(0);
        o.set("Material", Value::Struct(m));
        let values = shader_params(Preview::model("chest"), &o).base.material_values;
        let names: Vec<_> =
            values.iter().map(|v| (v.material.as_deref(), v.name.as_str())).collect();
        assert_eq!(
            names,
            [(Some("mg_mat"), "nOn"), (Some("mg_mat"), "vTint")],
            "unknown types left out"
        );
        assert_eq!(values[0].value, mg_image::mtr::Param::Int(7));
        assert_eq!(values[1].value, mg_image::mtr::Param::Float(vec![0.5, 0.0, 0.0, 0.0]));
    }

    #[test]
    fn replacements_reach_the_preview() {
        let mut o = Struct::new(0);
        let mut t = Struct::new(9);
        t.set(
            "TextureReplaceLi",
            Value::List(vec![
                pair("PLC_Chest1", "MG_New", ("OldTexture", "NewTexture")),
                pair("restored", "", ("OldTexture", "NewTexture")),
            ]),
        );
        o.set("TextureReplace", Value::Struct(t));
        let mut a = Struct::new(11);
        let e = pair("pause1", "mg_idle", ("OldAnimation", "NewAnimation"));
        a.set("AnimationReplace", Value::List(vec![e]));
        o.set("AnimationReplace", Value::Struct(a));
        let mut p = Preview::model("plc_chest1");
        p.idle = Some("pause1".into());
        // A PLT's fallback stays.
        p.base.textures.insert("restored".into(), "fallback".into());
        let p = replaced(p, &o);
        assert_eq!(p.base.textures.get("plc_chest1").map(String::as_str), Some("mg_new"));
        assert_eq!(p.base.textures.get("restored").map(String::as_str), Some("fallback"));
        assert_eq!(p.idle.as_deref(), Some("mg_idle"));
    }

    /// A creature's replacements reach its body (its PLT head too), not
    /// the items it wears or holds: in the client neither a helmet's nor a
    /// cloak's texture is replaced, by the field or the script call
    /// (`batch6_replaced_textures`, 2026-11-01).
    #[test]
    fn a_creature_s_replacements_leave_its_items_alone() {
        let mut o = Struct::new(0);
        let mut t = Struct::new(9);
        let list = ["pmh0_head001", "helm_001"]
            .map(|old| pair(old, "mgred", ("OldTexture", "NewTexture")))
            .to_vec();
        t.set("TextureReplaceLi", Value::List(list));
        o.set("TextureReplace", Value::Struct(t));
        let mut p = Preview::model("c_body");
        p.parts.push(Part::new("pmh0_head001"));
        p.parts.push(Part { item: true, ..Part::new("helm_001") });
        let p = replaced(p, &o);
        let new = |part: &Part, old: &str| part.textures.get(old).cloned();
        assert_eq!(new(&p.parts[0], "pmh0_head001").as_deref(), Some("mgred"), "the head");
        assert_eq!(new(&p.parts[1], "helm_001"), None, "the helmet");
    }

    /// A container that lists its resources and gives none.
    #[derive(Debug)]
    struct Listed(Vec<mg_resman::ResKey>);

    impl mg_resman::Container for Listed {
        fn contains(&self, key: &mg_resman::ResKey) -> bool {
            self.0.contains(key)
        }
        fn read(
            &self,
            key: &mg_resman::ResKey,
        ) -> Result<std::borrow::Cow<'_, [u8]>, mg_resman::ResError> {
            // (A material is read for its `texture0`: one with none is no
            // texture, `no_image` here.)
            match key.restype {
                ResType::MTR if key.resref.as_str() == Some("no_image") => {
                    Ok(b"texture1 normal_n\n".as_slice().into())
                }
                ResType::MTR => Ok(b"texture0 metal_d\n".as_slice().into()),
                _ => panic!("{key} was read to ask whether it is there"),
            }
        }
        fn keys(&self) -> Box<dyn Iterator<Item = mg_resman::ResKey> + '_> {
            Box::new(self.0.iter().copied())
        }
        fn len(&self) -> usize {
            self.0.len()
        }
    }

    /// Whether a model or a texture is there is asked of the lists, not by
    /// reading it (a creature asks of every part it might wear, and a
    /// hak's resource is read from its file each time); only a material
    /// that is a name's one resource is read, for its `texture0`.
    #[test]
    fn asking_whether_a_model_or_a_texture_is_there_reads_nothing() {
        let key = |n: &str, t| mg_resman::ResKey::parse(n, t).unwrap();
        let listed = Listed(vec![
            key("c_orc", ResType::MDL),
            key("c_orc", ResType::DDS),
            key("old", ResType::TGA),
            key("skin", ResType::PLT),
            key("metal", ResType::MTR),
            key("no_image", ResType::MTR),
            key("c_drgdeep_hed_ne", ResType::DDS),
        ]);
        let mut rm = mg_resman::ResMan::new();
        rm.add(mg_resman::priority::HAK_USER, "hak:x", mg_resman::LayerClass::Erf, listed);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let lk = Lookup { game: &game };
        assert!(lk.has_model("c_orc") && !lk.has_model("c_elf"));
        assert!(!lk.has_model("a name too long for a resource"));
        for texture in ["c_orc", "old", "skin", "metal"] {
            assert!(lk.has_texture(texture), "{texture}");
        }
        assert!(!lk.has_texture("none"));
        assert!(!lk.has_texture("no_image"), "a material with no texture0");
        // A longer name, as the game reads it: its first 16 characters.
        assert!(lk.has_texture("c_drgdeep_hed_new"));
    }
}
