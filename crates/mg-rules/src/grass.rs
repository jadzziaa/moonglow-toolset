//! An area's grass: what grows on its tiles' walkmesh faces, by surface
//! material (surfacemat.2da row). The tileset's `[GRASS]` grows on grass
//! faces (material 3) unless the area turns it off; overrides grow their
//! own grass on a material, as the game's `SetAreaGrassOverride` does.
//!
//! The engine keeps the area's settings in its GIT's `AreaProperties`
//! (`engine_grass` settles it: read from a module's GIT when the module
//! loads, not from its ARE):
//!
//! - `GrassDefDisabled` (BYTE): the tileset's grass is off;
//! - `GrassOverrides` (struct 101) › `GrassOvrList`: an item (struct 0) for
//!   each material: `GrassMaterialId` (BYTE), `GrassTexture` (CExoString),
//!   `GrassDensity` and `GrassHeight` (FLOAT), `GrassAmbientX`/`Y`/`Z` and
//!   `GrassDiffuseX`/`Y`/`Z` (FLOAT, red, green and blue, 0 to 1).

use mg_gff::{Struct, Value};
use mg_set::Tileset;

/// Where in the GIT the settings are.
pub const PROPERTIES: &str = "AreaProperties";
pub const DISABLED: &str = "GrassDefDisabled";
pub const OVERRIDES: &str = "GrassOverrides";
pub const LIST: &str = "GrassOvrList";
/// The struct id of `GrassOverrides`, as the engine writes it.
pub const OVERRIDES_ID: u32 = 101;

/// The surface material the tileset's grass grows on.
pub const GRASS_MATERIAL: u32 = 3;
/// The texture of a tileset that names none.
pub const DEFAULT_TEXTURE: &str = "grass";

/// The fields of an override, as they are labelled.
pub const MATERIAL: &str = "GrassMaterialId";
pub const TEXTURE: &str = "GrassTexture";
pub const DENSITY: &str = "GrassDensity";
pub const HEIGHT: &str = "GrassHeight";
pub const AMBIENT: [&str; 3] = ["GrassAmbientX", "GrassAmbientY", "GrassAmbientZ"];
pub const DIFFUSE: [&str; 3] = ["GrassDiffuseX", "GrassDiffuseY", "GrassDiffuseZ"];

/// What one kind of grass looks like.
#[derive(Debug, Clone, PartialEq)]
pub struct GrassLook {
    /// The texture (a ResRef, lower case): four tufts, two by two.
    pub texture: String,
    pub density: f32,
    /// Meters.
    pub height: f32,
    /// Red, green and blue, 0 to 1.
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
}

impl GrassLook {
    /// A tileset's `[GRASS]`, if it grows grass.
    pub fn of_tileset(tileset: &Tileset) -> Option<GrassLook> {
        let g = &tileset.grass;
        g.enabled.then(|| GrassLook {
            texture: g.texture.as_deref().unwrap_or(DEFAULT_TEXTURE).to_ascii_lowercase(),
            density: g.density,
            height: g.height,
            ambient: g.ambient,
            diffuse: g.diffuse,
        })
    }

    /// Whether any of it shows.
    pub fn grows(&self) -> bool {
        self.density > 0.0 && self.height > 0.0 && !self.texture.is_empty()
    }
}

/// Grass grown on one surface material in place of what grows there.
#[derive(Debug, Clone, PartialEq)]
pub struct GrassOverride {
    pub material: u32,
    pub look: GrassLook,
}

impl GrassOverride {
    /// An item of `GrassOvrList` (`None`: it names no material).
    pub fn read(s: &Struct) -> Option<GrassOverride> {
        let material = u32::try_from(s.integer(MATERIAL)?).ok()?;
        let float = |l: &str| s.float(l).unwrap_or(0.0);
        let rgb = |labels: [&str; 3]| labels.map(|l| float(l).clamp(0.0, 1.0));
        let texture = s.string(TEXTURE).unwrap_or_default();
        Some(GrassOverride {
            material,
            look: GrassLook {
                texture: String::from_utf8_lossy(texture).trim().to_ascii_lowercase(),
                density: float(DENSITY),
                height: float(HEIGHT),
                ambient: rgb(AMBIENT),
                diffuse: rgb(DIFFUSE),
            },
        })
    }

    /// The item of `GrassOvrList` it is written as.
    pub fn item(&self) -> Struct {
        let mut s = Struct::new(0);
        s.set(MATERIAL, Value::Byte(self.material.min(255) as u8));
        s.set(TEXTURE, Value::String(self.look.texture.as_bytes().to_vec()));
        for (label, v) in self.fields() {
            s.set(label, v);
        }
        s
    }

    /// Its fields but the material and the texture, as they are stored.
    pub fn fields(&self) -> Vec<(&'static str, Value)> {
        let l = &self.look;
        let mut out = vec![(DENSITY, Value::Float(l.density)), (HEIGHT, Value::Float(l.height))];
        for (labels, rgb) in [(AMBIENT, l.ambient), (DIFFUSE, l.diffuse)] {
            out.extend(labels.iter().zip(rgb).map(|(l, c)| (*l, Value::Float(c.clamp(0.0, 1.0)))));
        }
        out
    }

    /// A new override for `material`: the tileset's grass (else the
    /// game's texture, of 1 meter at density 5), as a start.
    pub fn new(material: u32, tileset: Option<&Tileset>) -> GrassOverride {
        let look = tileset.and_then(GrassLook::of_tileset).unwrap_or(GrassLook {
            texture: DEFAULT_TEXTURE.into(),
            density: 5.0,
            height: 1.0,
            ambient: [0.5, 0.6, 0.3],
            diffuse: [0.5, 0.6, 0.3],
        });
        GrassOverride { material, look }
    }
}

/// An area's grass settings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AreaGrass {
    /// The tileset's grass is turned off.
    pub default_disabled: bool,
    /// In the order they are stored.
    pub overrides: Vec<GrassOverride>,
}

impl AreaGrass {
    /// From the area's GIT (its root struct).
    pub fn read(git: &Struct) -> AreaGrass {
        let Some(p) = git.child(PROPERTIES) else { return AreaGrass::default() };
        let overrides = p.child(OVERRIDES).and_then(|o| o.list(LIST)).unwrap_or(&[]);
        AreaGrass {
            default_disabled: p.integer(DISABLED).is_some_and(|v| v != 0),
            overrides: overrides.iter().filter_map(GrassOverride::read).collect(),
        }
    }

    /// The override for `material`, if there is one (the last, should
    /// there be two: the engine keeps one a material).
    pub fn for_material(&self, material: u32) -> Option<&GrassOverride> {
        self.overrides.iter().rev().find(|o| o.material == material)
    }

    /// What grows on each material, by material: the overrides, and the
    /// tileset's grass on grass faces unless it is turned off or
    /// overridden.
    pub fn growing(&self, tileset: Option<&Tileset>) -> Vec<(u32, GrassLook)> {
        self.growing_over(tileset.and_then(GrassLook::of_tileset))
    }

    /// [`growing`](Self::growing), with the tileset's grass (`None`: it
    /// grows none).
    pub fn growing_over(&self, own: Option<GrassLook>) -> Vec<(u32, GrassLook)> {
        let mut out: Vec<(u32, GrassLook)> = Vec::new();
        for o in &self.overrides {
            out.retain(|(m, _)| *m != o.material);
            out.push((o.material, o.look.clone()));
        }
        let overridden = out.iter().any(|(m, _)| *m == GRASS_MATERIAL);
        if !self.default_disabled && !overridden {
            out.extend(own.map(|l| (GRASS_MATERIAL, l)));
        }
        out.retain(|(_, l)| l.grows());
        out.sort_by_key(|(m, _)| *m);
        out
    }
}

impl GrassLook {
    /// As `mg areas` prints it: `grass, density 5, height 1.2 m, ambient
    /// 0.5 0.6 0.3, diffuse 0.5 0.6 0.3`.
    pub fn describe(&self) -> String {
        let rgb = |c: [f32; 3]| c.map(|v| v.to_string()).join(" ");
        format!(
            "{}, density {}, height {} m, ambient {}, diffuse {}",
            self.texture,
            self.density,
            self.height,
            rgb(self.ambient),
            rgb(self.diffuse)
        )
    }
}

/// The colour of the game's own grass (its `oldgrassinfo.txi`, and the
/// rural tilesets'), for an override that gives none.
pub const GAME_GRASS_COLOR: [f32; 3] = [0.5, 0.6, 0.3];

impl GrassOverride {
    /// `MATERIAL:TEXTURE:DENSITY:HEIGHT[:R,G,B[:R,G,B]]` (`mg areas
    /// --grass`): the material a surfacemat.2da row (a number, or a name
    /// `material` knows), the ambient color and the diffuse (red, green
    /// and blue, 0 to 1; the diffuse the ambient's when it is not given,
    /// both the game's grass color when neither is).
    pub fn parse(text: &str, material: impl Fn(&str) -> Option<u32>) -> Result<Self, String> {
        let parts: Vec<&str> = text.split(':').map(str::trim).collect();
        let [name, texture, density, height, colors @ ..] = parts.as_slice() else {
            return Err(format!(
                "{text}: expected MATERIAL:TEXTURE:DENSITY:HEIGHT[:R,G,B[:R,G,B]]"
            ));
        };
        let material = name
            .parse::<u32>()
            .ok()
            .or_else(|| material(name))
            .ok_or_else(|| format!("{name}: not a surface material (a surfacemat.2da row)"))?;
        if texture.is_empty() || texture.len() > 16 {
            return Err(format!("{text}: the texture is a ResRef of 1 to 16 characters"));
        }
        let number = |what: &str, v: &str| -> Result<f32, String> {
            v.parse::<f32>()
                .ok()
                .filter(|n| n.is_finite() && *n >= 0.0)
                .ok_or_else(|| format!("{what} {v:?}: not a number of 0 or more"))
        };
        let rgb = |v: &str| -> Result<[f32; 3], String> {
            let c: Vec<f32> =
                v.split(',').map(|c| number("color", c.trim())).collect::<Result<_, _>>()?;
            match c.as_slice() {
                [r, g, b] => Ok([*r, *g, *b].map(|c| c.clamp(0.0, 1.0))),
                _ => Err(format!("{v}: a color is R,G,B")),
            }
        };
        let (ambient, diffuse) = match colors {
            [] => (GAME_GRASS_COLOR, GAME_GRASS_COLOR),
            [a] => (rgb(a)?, rgb(a)?),
            [a, d] => (rgb(a)?, rgb(d)?),
            _ => return Err(format!("{text}: at most two colors (ambient, diffuse)")),
        };
        Ok(GrassOverride {
            material,
            look: GrassLook {
                texture: texture.to_ascii_lowercase(),
                density: number("density", density)?,
                height: number("height", height)?,
                ambient,
                diffuse,
            },
        })
    }
}

/// One change a grass edit makes to a GIT's `AreaProperties`, in order:
/// what [`set_in`] and its kin make in place (`mg areas`), and what the
/// area view's undoable edits make (`mg_area::grass`), so that the two
/// can't come to differ. Indices are of `GrassOvrList` as it is when the
/// step is made (after the steps before it).
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Sets a field of `AreaProperties` (the tileset's grass off; the
    /// overrides' struct, made with its list where there is none).
    Property { label: &'static str, value: Value },
    /// Sets a field of override `index`.
    Field { index: usize, label: &'static str, value: Value },
    /// Puts an override into the list at `index`.
    Insert { index: usize, item: Struct },
    /// Takes override `index` out of the list.
    Remove(usize),
}

/// The overrides' list in `props` (a GIT's `AreaProperties`), if it has
/// one.
fn list_in(props: Option<&Struct>) -> Option<&[Struct]> {
    props?.child(OVERRIDES)?.list(LIST)
}

/// The places of the overrides for `material` in `list`, last first (as
/// they are taken out: the indices before stay as they are).
fn places_of(list: &[Struct], material: u32) -> impl Iterator<Item = usize> + '_ {
    let same = move |s: &Struct| GrassOverride::read(s).is_some_and(|o| o.material == material);
    list.iter().enumerate().rev().filter(move |(_, s)| same(s)).map(|(i, _)| i)
}

/// What grows `o` on its material in `props` (a GIT's `AreaProperties`):
/// the item it has for the material (the last, should there be two)
/// changed where it differs, what else the item holds kept, and the
/// others for the material taken out; else a new one at the end, in a
/// list made if there is none. Nothing, if it grows `o` already.
pub fn set_plan(props: Option<&Struct>, o: &GrassOverride) -> Vec<Step> {
    let Some(list) = list_in(props) else {
        let mut overrides = Struct::new(OVERRIDES_ID);
        overrides.set(LIST, Value::List(vec![o.item()]));
        return vec![Step::Property { label: OVERRIDES, value: Value::Struct(overrides) }];
    };
    let mut places = places_of(list, o.material);
    let Some(index) = places.next() else {
        return vec![Step::Insert { index: list.len(), item: o.item() }];
    };
    let mut fields = o.fields();
    fields.push((TEXTURE, Value::String(o.look.texture.as_bytes().to_vec())));
    let there = &list[index];
    let mut steps: Vec<Step> = fields
        .into_iter()
        .filter(|(label, v)| there.get(label) != Some(v))
        .map(|(label, value)| Step::Field { index, label, value })
        .collect();
    // (The others are before it: taken out after its fields are set.)
    steps.extend(places.map(Step::Remove));
    steps
}

/// What takes the overrides for `material` out of `props`.
pub fn remove_plan(props: Option<&Struct>, material: u32) -> Vec<Step> {
    list_in(props)
        .map(|list| places_of(list, material).map(Step::Remove).collect())
        .unwrap_or_default()
}

/// What moves the grass `props` grows on material `from` to material `to`
/// (in place of what grows there): the item for `from` keeps its place
/// in the list and what else it holds.
pub fn move_plan(props: Option<&Struct>, from: u32, to: u32) -> Vec<Step> {
    let Some(list) = list_in(props).filter(|_| from != to) else { return Vec::new() };
    let Some(index) = places_of(list, from).next() else { return Vec::new() };
    // (Those for `to` go first: the item's index less as many as were
    // before it.)
    let gone = remove_plan(props, to);
    let before = gone.iter().filter(|s| matches!(s, Step::Remove(i) if *i < index)).count();
    let mut steps = gone;
    let value = Value::Byte(to.min(255) as u8);
    steps.push(Step::Field { index: index - before, label: MATERIAL, value });
    steps
}

/// What turns the tileset's grass off (or on) in `props`; nothing if it
/// is so already (no field is off).
pub fn disabled_plan(props: Option<&Struct>, disabled: bool) -> Vec<Step> {
    let was = props.and_then(|p| p.integer(DISABLED)).is_some_and(|v| v != 0);
    if was == disabled {
        return Vec::new();
    }
    vec![Step::Property { label: DISABLED, value: Value::Byte(u8::from(disabled)) }]
}

/// Makes `steps` in `props`, in order. A step whose place isn't there (a
/// plan made of other properties) is passed by.
pub fn apply(props: &mut Struct, steps: &[Step]) {
    for step in steps {
        if let Step::Property { label, value } = step {
            props.set(label, value.clone());
            continue;
        }
        let Some(list) = props.child_mut(OVERRIDES).and_then(|o| o.list_mut(LIST)) else {
            continue;
        };
        match step {
            Step::Field { index, label, value } => {
                if let Some(item) = list.get_mut(*index) {
                    item.set(label, value.clone());
                }
            }
            Step::Insert { index, item } => list.insert((*index).min(list.len()), item.clone()),
            Step::Remove(index) if *index < list.len() => _ = list.remove(*index),
            _ => {}
        }
    }
}

/// Grows `o` on its material in `props` ([`set_plan`]). What it was
/// before, if anything changed.
pub fn set_in(props: &mut Struct, o: &GrassOverride) -> Option<Option<GrassOverride>> {
    let steps = set_plan(Some(props), o);
    if steps.is_empty() {
        return None;
    }
    let was = list_in(Some(props)).and_then(|l| places_of(l, o.material).next().map(|i| &l[i]));
    let was = was.and_then(GrassOverride::read);
    apply(props, &steps);
    Some(was)
}

/// Takes the overrides for `material` out of `props`; the one there was.
pub fn remove_in(props: &mut Struct, material: u32) -> Option<GrassOverride> {
    let list = list_in(Some(props))?;
    let was = places_of(list, material).next().and_then(|i| GrassOverride::read(&list[i]));
    let steps = remove_plan(Some(props), material);
    apply(props, &steps);
    was
}

/// Turns the tileset's grass off (or on) in `props`; whether that changed
/// it.
pub fn set_disabled_in(props: &mut Struct, disabled: bool) -> bool {
    let steps = disabled_plan(Some(props), disabled);
    apply(props, &steps);
    !steps.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_with(items: Vec<Struct>, disabled: Option<u8>) -> Struct {
        let mut props = Struct::new(100);
        props.set("AmbientSndDay", Value::Int(3));
        if let Some(d) = disabled {
            props.set(DISABLED, Value::Byte(d));
        }
        let mut overrides = Struct::new(OVERRIDES_ID);
        overrides.set(LIST, Value::List(items));
        props.set(OVERRIDES, Value::Struct(overrides));
        let mut root = Struct::new(u32::MAX);
        root.set(PROPERTIES, Value::Struct(props));
        root
    }

    fn look(texture: &str, density: f32) -> GrassLook {
        GrassLook {
            texture: texture.into(),
            density,
            height: 1.0,
            ambient: [0.1, 0.2, 0.3],
            diffuse: [0.4, 0.5, 0.6],
        }
    }

    fn tileset() -> Tileset {
        let set = "[GENERAL]\nName=TST01\nTransition=5\n[GRASS]\nGrass=1\nDensity=5\nHeight=1.2\n\
                   AmbientRed=0.5\nAmbientGreen=0.6\nAmbientBlue=0.3\nDiffuseRed=0.5\n\
                   DiffuseGreen=0.6\nDiffuseBlue=0.3\n[TERRAIN TYPES]\nCount=0\n[TILES]\nCount=0\n\
                   [GROUPS]\nCount=0\n";
        Tileset::parse(set.as_bytes(), mg_core::Codepage::WINDOWS_1252).unwrap()
    }

    #[test]
    fn an_override_reads_back_as_it_was_written() {
        let o = GrassOverride { material: 5, look: look("mggrass", 3.5) };
        let git = git_with(vec![o.item()], Some(1));
        let grass = AreaGrass::read(&git);
        assert!(grass.default_disabled);
        assert_eq!(grass.overrides, [o]);
        let item = &git.child(PROPERTIES).unwrap().child(OVERRIDES).unwrap().list(LIST).unwrap()[0];
        assert!(matches!(item.get(MATERIAL), Some(Value::Byte(5))));
        assert!(matches!(item.get(TEXTURE), Some(Value::String(_))));
        assert!(matches!(item.get("GrassAmbientZ"), Some(Value::Float(_))));
    }

    #[test]
    fn colors_out_of_range_are_clamped_as_the_engine_clamps_them() {
        let mut o = GrassOverride { material: 1, look: look("grass", 1.0) };
        o.look.ambient = [2.0, -1.0, 0.5];
        let read = GrassOverride::read(&o.item()).unwrap();
        assert_eq!(read.look.ambient, [1.0, 0.0, 0.5]);
    }

    #[test]
    fn the_tileset_grows_on_grass_unless_turned_off_or_overridden() {
        let set = tileset();
        let none = AreaGrass::default();
        let grows = none.growing(Some(&set));
        assert_eq!(grows.len(), 1);
        assert_eq!(grows[0].0, GRASS_MATERIAL);
        assert_eq!(grows[0].1.texture, "grass");
        assert_eq!(grows[0].1.height, 1.2);
        let off = AreaGrass { default_disabled: true, ..AreaGrass::default() };
        assert!(off.growing(Some(&set)).is_empty());
        let dirt = GrassOverride { material: 1, look: look("mgdirt", 2.0) };
        let grass = GrassOverride { material: 3, look: look("mggrass", 9.0) };
        let both = AreaGrass { default_disabled: false, overrides: vec![dirt, grass] };
        let grows = both.growing(Some(&set));
        let named: Vec<(u32, &str)> = grows.iter().map(|(m, l)| (*m, l.texture.as_str())).collect();
        assert_eq!(named, [(1, "mgdirt"), (3, "mggrass")]);
        // Without a texture or any density nothing grows.
        let bare = GrassOverride { material: 4, look: look("", 2.0) };
        let thin = GrassOverride { material: 5, look: look("grass", 0.0) };
        let none = AreaGrass { default_disabled: true, overrides: vec![bare, thin] };
        assert!(none.growing(Some(&set)).is_empty());
    }

    #[test]
    fn an_area_without_settings_has_none() {
        assert_eq!(AreaGrass::read(&Struct::new(0)), AreaGrass::default());
        let mut props = Struct::new(100);
        props.set("AmbientSndDay", Value::Int(3));
        let mut root = Struct::new(0);
        root.set(PROPERTIES, Value::Struct(props));
        assert_eq!(AreaGrass::read(&root), AreaGrass::default());
    }

    #[test]
    fn an_override_is_parsed_from_its_text() {
        let named = |n: &str| (n.eq_ignore_ascii_case("dirt")).then_some(1);
        let o = GrassOverride::parse("Dirt:TTZ_Grass:3:0.5", named).unwrap();
        assert_eq!(o.material, 1);
        assert_eq!(o.look.texture, "ttz_grass");
        assert_eq!((o.look.density, o.look.height), (3.0, 0.5));
        assert_eq!(o.look.ambient, GAME_GRASS_COLOR);
        let o = GrassOverride::parse("5:grass:1:2:1,0,0", named).unwrap();
        assert_eq!(
            (o.material, o.look.ambient, o.look.diffuse),
            (5, [1.0, 0.0, 0.0], [1.0, 0.0, 0.0])
        );
        let o = GrassOverride::parse("5:grass:1:2:1,0,0:0,2,0", named).unwrap();
        assert_eq!(o.look.diffuse, [0.0, 1.0, 0.0]);
        for bad in [
            "5:grass:1",
            "stone:grass:1:1",
            "5::1:1",
            "5:grass:x:1",
            "5:grass:1:-1",
            "5:grass:1:1:1,0",
        ] {
            assert!(GrassOverride::parse(bad, named).is_err(), "{bad}");
        }
    }

    #[test]
    fn overrides_are_set_in_place_and_removed() {
        let mut root = git_with(Vec::new(), None);
        let props = root.child_mut(PROPERTIES).unwrap();
        let dirt = GrassOverride { material: 1, look: look("mgdirt", 2.0) };
        assert_eq!(set_in(props, &dirt), Some(None));
        assert_eq!(set_in(props, &dirt), None, "nothing to change");
        props.child_mut(OVERRIDES).unwrap().list_mut(LIST).unwrap()[0].set("MgKeep", Value::Int(1));
        let denser = GrassOverride { material: 1, look: look("mgdirt", 4.0) };
        assert_eq!(set_in(props, &denser), Some(Some(dirt.clone())));
        let item = &props.child(OVERRIDES).unwrap().list(LIST).unwrap()[0];
        assert_eq!(item.integer("MgKeep"), Some(1));
        assert!(set_disabled_in(props, true));
        assert!(!set_disabled_in(props, true));
        assert_eq!(remove_in(props, 1), Some(denser));
        assert_eq!(remove_in(props, 1), None);
        let grass = AreaGrass::read(&root);
        assert!(grass.default_disabled && grass.overrides.is_empty());
    }
}
