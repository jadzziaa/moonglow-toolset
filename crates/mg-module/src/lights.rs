//! Lights of any color and reach, as content Moonglow writes (the lights
//! proposal's way B): for each setting a model with one `light` node and
//! nothing to see, and a placeables.2da row that names it, both written
//! into a hak the builder chose, which the module lists. Placing and
//! moving one is then placing a placeable of that row.
//!
//! Off unless asked for (the owner's word, 2026-10-29: "As an option,
//! default off to prevent unwittingly touching PW haks from
//! maintainers"): Options › Areas and the project file's `[lights]
//! generated` (which wins).
//!
//! The placeables.2da written is the copy the game reads now, with the
//! row added at its end (or at the project's `[lights] first_row`): no
//! merge of several haks' copies is needed, as the hak written must be
//! the one whose copy the game reads, or above it ([`plan`] refuses
//! another). Two lights of the same settings share a model and a row.
//!
//! What the client has yet to say (the next client batch): whether such a
//! light lights creatures and placeables or only tiles, and how many
//! dynamic lights the game draws at one place. The node says it lights
//! dynamic objects (`affectdynamic`, [`LightSpec::lights_objects`]) and
//! its priority ([`LightSpec::priority`]): either answer changes a
//! default, not what is written.

use std::path::{Path, PathBuf};

use mg_2da::TwoDa;
use mg_core::{Codepage, ResRef, ResType};
use mg_resman::{ResKey, ResMan, priority};

/// The models' names begin so: `mgl_` and twelve hexadecimal digits of
/// the settings.
pub const MODEL_PREFIX: &str = "mgl_";

/// A light's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightSpec {
    /// Its color as a picker gives it (0 to 255 each).
    pub color: [u8; 3],
    /// Its radius, meters: the light ends at twice it (times the
    /// multiplier), as the client draws a model's light.
    pub radius: f32,
    /// How much further it reaches (the node's `multiplier`: the client
    /// lengthens a light's reach by it, not its brightness).
    pub multiplier: f32,
    /// Creatures' shadows are cast from it (the node's `shadow`).
    pub shadows: bool,
    /// It lights creatures and placeables as well as the ground (the
    /// node's `affectdynamic`).
    pub lights_objects: bool,
    /// 1 (kept first) to 5: which lights the game keeps where more shine
    /// at one place than it draws (`lightpriority`).
    pub priority: u8,
}

impl Default for LightSpec {
    fn default() -> LightSpec {
        LightSpec {
            color: [255, 200, 140],
            radius: 5.0,
            multiplier: 1.0,
            shadows: false,
            lights_objects: true,
            priority: 3,
        }
    }
}

impl LightSpec {
    /// The settings as the file keeps them: the radius and the multiplier
    /// to a hundredth, the priority in 1 to 5.
    pub fn normalized(self) -> LightSpec {
        let cents = |v: f32, lo: f32, hi: f32| (v.clamp(lo, hi) * 100.0).round() / 100.0;
        LightSpec {
            radius: cents(self.radius, 0.1, 100.0),
            multiplier: cents(self.multiplier, 0.1, 10.0),
            priority: self.priority.clamp(1, 5),
            ..self
        }
    }

    /// Where the light ends, meters (as `mg_render::anim::Lamp::placed`
    /// and the client make it of a model's light).
    pub fn reach(&self) -> f32 {
        let s = self.normalized();
        s.radius * s.multiplier * 2.0
    }

    /// Its model's name: [`MODEL_PREFIX`] and a hash of the settings, the
    /// same for the same settings.
    pub fn model_name(&self) -> ResRef {
        let s = self.normalized();
        let text = format!(
            "{:?} {} {} {} {} {}",
            s.color,
            (s.radius * 100.0) as u32,
            (s.multiplier * 100.0) as u32,
            s.shadows,
            s.lights_objects,
            s.priority
        );
        // (FNV-1a, 64 bits; 48 of them kept.)
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in text.bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
        let name = format!("{MODEL_PREFIX}{:012x}", h & 0xffff_ffff_ffff);
        ResRef::from_str(&name).expect("16 characters")
    }

    /// Its model as text: a dummy, a square of 0.2 m drawn by nothing (as
    /// the game's invisible light placeables have), and the light.
    pub fn model_text(&self) -> String {
        let s = self.normalized();
        let n = self.model_name().to_string();
        let [r, g, b] = s.color.map(|c| f32::from(c) / 255.0);
        format!(
            "# A light made by Moonglow Toolset: {} m, multiplier {}\n\
             newmodel {n}\nsetsupermodel {n} NULL\nclassification character\n\
             setanimationscale 1\nbeginmodelgeom {n}\n\
             node dummy {n}\n  parent NULL\nendnode\n\
             node trimesh {n}_base\n  parent {n}\n  position 0 0 0\n  orientation 0 0 0 0\n  \
             ambient 1 1 1\n  diffuse 1 1 1\n  specular 0 0 0\n  shininess 1\n  bitmap NULL\n  \
             render 0\n  shadow 0\n  verts 4\n    -0.1 -0.1 0\n    0.1 -0.1 0\n    0.1 0.1 0\n    \
             -0.1 0.1 0\n  faces 2\n    0 1 2 1 0 0 0 0\n    0 2 3 1 0 0 0 0\nendnode\n\
             node light {n}_light\n  parent {n}\n  position 0 0 0\n  orientation 0 0 0 0\n  \
             radius {}\n  multiplier {}\n  color {r:.4} {g:.4} {b:.4}\n  ambientonly 0\n  \
             isdynamic 1\n  affectdynamic {}\n  shadow {}\n  lightpriority {}\n  \
             fadingLight 1\n  flareradius 0\nendnode\n\
             endmodelgeom {n}\ndonemodel {n}\n",
            s.radius,
            s.multiplier,
            s.radius,
            s.multiplier,
            u8::from(s.lights_objects),
            u8::from(s.shadows),
            s.priority,
        )
    }

    /// Its model compiled, as the game's own are.
    pub fn model_binary(&self) -> Result<Vec<u8>, String> {
        let text = self.model_text();
        let compiled = mg_mdl::compile::compile(text.as_bytes(), &Default::default())
            .map_err(|e| format!("{}: {e}", self.model_name()))?;
        Ok(compiled.binary)
    }

    /// Its placeables.2da row's label.
    pub fn label(&self) -> String {
        let s = self.normalized();
        let [r, g, b] = s.color;
        format!("Light, #{r:02x}{g:02x}{b:02x}, {} m", short(s.reach()))
    }
}

fn short(v: f32) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The table's key.
pub fn placeables_key() -> ResKey {
    ResKey::new(ResRef::from_str("placeables").expect("a name"), ResType::TWODA)
}

/// The hak a light goes into: one the module lists (`hak:NAME`'s layer,
/// its file), or a new one, made at `path` and put at the top of the
/// module's list.
#[derive(Debug, Clone, PartialEq)]
pub struct HakChoice {
    /// Its name, without `.hak`.
    pub name: String,
    pub path: PathBuf,
    /// Not there yet, or not in the module's list: written new (or added
    /// to the list, at its top).
    pub new: bool,
}

/// What writing a light will do, worked out before anything is written.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub spec: LightSpec,
    pub hak: HakChoice,
    pub model: ResRef,
    /// Its placeables.2da row.
    pub row: usize,
    /// The row (and its model) were written before, by the same settings:
    /// nothing is written.
    pub reused: bool,
    /// Where the placeables.2da written comes from: the layer whose copy
    /// the game reads now.
    pub table_from: String,
    /// The table written (the copy the game reads, with the row).
    table: TwoDa,
}

impl Plan {
    /// What the window says before anything is written.
    pub fn describe(&self) -> String {
        if self.reused {
            return format!(
                "A light of these settings is there already: row {} of placeables.2da, model \
                 {}, in {}.hak. Nothing is written.",
                self.row, self.model, self.hak.name
            );
        }
        let hak = match self.hak.new {
            true => format!(
                "{} (a new hak, put at the top of the module's list)",
                self.hak.path.display()
            ),
            false => self.hak.path.display().to_string(),
        };
        format!(
            "Writes into {hak}: the model {} and placeables.2da with row {} added (the copy \
             the game reads now, from {}). The module's players must have this hak: a \
             persistent world ships it to them (NWSync, or by hand).",
            self.model, self.row, self.table_from
        )
    }
}

/// Works out writing `spec` into `hak` against the game data `rm` (the
/// module's haks layered, as the game reads them): which row, from which
/// copy of placeables.2da. Refused where the hak would not be read: a
/// layer above it (another hak, a development folder) has the table.
/// `first_row`: the project's `[lights] first_row`, the least row a light
/// takes.
pub fn plan(
    rm: &ResMan,
    codepage: Codepage,
    spec: LightSpec,
    hak: HakChoice,
    first_row: Option<usize>,
) -> Result<Plan, String> {
    let spec = spec.normalized();
    let key = placeables_key();
    let top = rm.find(&key).ok_or("the game data has no placeables.2da")?;
    let layers = rm.layers();
    let label = format!("hak:{}", hak.name.to_lowercase());
    let at = layers.iter().position(|l| l.label.eq_ignore_ascii_case(&label));
    let read_over = match at {
        // (Listed: the copy read must be its own or below it.)
        Some(i) if !hak.new => top < i,
        // (New, it goes above every hak.)
        _ => layers[top].priority > priority::HAK_USER,
    };
    if read_over {
        return Err(format!(
            "placeables.2da is read from {} over {}.hak: a light written there would not be \
             found. Choose {}, or move {}.hak above it in the module's list",
            layers[top].label,
            hak.name,
            layers[top]
                .label
                .strip_prefix("hak:")
                .map_or("another hak".into(), |h| format!("{h}.hak")),
            hak.name
        ));
    }
    let data = layers[top].container.read(&key).map_err(|e| format!("placeables.2da: {e}"))?;
    let mut table = TwoDa::parse(&data, codepage).map_err(|e| format!("placeables.2da: {e}"))?;
    let model = spec.model_name();
    let model_text = model.to_string();
    let had = (0..table.len())
        .find(|&r| table.get(r, "ModelName").is_some_and(|m| m.eq_ignore_ascii_case(&model_text)));
    let has_model = match at {
        Some(i) => layers[i].container.contains(&ResKey::new(model, ResType::MDL)),
        None => false,
    };
    if let Some(row) = had.filter(|_| has_model) {
        return Ok(Plan {
            spec,
            hak,
            model,
            row,
            reused: true,
            table_from: layers[top].label.clone(),
            table,
        });
    }
    let row = match had {
        Some(row) => row,
        None => {
            while table.len() < first_row.unwrap_or(0) {
                table.add_row();
            }
            let row = table.add_row();
            // (As the game's own invisible lights have it, "Light, White":
            // a static placeable with a shadow of 1, its light its model's.)
            let cells = [
                ("Label", spec.label()),
                ("ModelName", model_text.clone()),
                ("ShadowSize", "1".into()),
                ("BodyBag", "0".into()),
                ("Static", "1".into()),
            ];
            for (name, value) in cells {
                if let Some(c) = table.column(name) {
                    table.set(row, c, Some(value)).map_err(|e| format!("placeables.2da: {e}"))?;
                }
            }
            row
        }
    };
    Ok(Plan { spec, hak, model, row, reused: false, table_from: layers[top].label.clone(), table })
}

/// Writes what `plan` worked out: the model and placeables.2da into its
/// hak (made if new; one that is there kept beside itself as `.bak`
/// first). Nothing for a plan that reuses what is there.
pub fn write(plan: &Plan, codepage: Codepage) -> Result<(), String> {
    if plan.reused {
        return Ok(());
    }
    use crate::hak_edit::Source;
    let model = plan.spec.model_binary()?;
    let layout = mg_2da::Layout::Aligned;
    let table = plan.table.to_bytes_as(codepage, layout).map_err(|e| e.to_string())?;
    let resources = vec![
        (ResKey::new(plan.model, ResType::MDL), Source::Bytes(model.into())),
        (placeables_key(), Source::Bytes(table.into())),
    ];
    if let Some(dir) = plan.hak.path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    crate::hak_edit::write_into(&plan.hak.path, resources).map(|_| ())
}

/// Whether `model` is a light Moonglow made ([`LightSpec::model_name`]).
pub fn is_generated(model: &str) -> bool {
    model.len() == MODEL_PREFIX.len() + 12
        && model.get(..MODEL_PREFIX.len()).is_some_and(|p| p.eq_ignore_ascii_case(MODEL_PREFIX))
}

/// The hak called `name` where a module's new haks go: the user's `hak/`.
pub fn new_hak(hak_dir: &Path, name: &str) -> Result<HakChoice, String> {
    let name = name.trim().to_lowercase();
    if name.is_empty()
        || name.len() > 16
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!("\"{name}\": a hak's name is up to 16 letters, digits and _"));
    }
    let path = hak_dir.join(format!("{name}.hak"));
    Ok(HakChoice { name, path, new: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_resman::{LayerClass, MemContainer};

    fn table(rows: &[&str]) -> Vec<u8> {
        let mut t = String::from(
            "2DA V2.0\n\n Label StrRef ModelName LightColor ShadowSize BodyBag Static\n",
        );
        for (i, r) in rows.iter().enumerate() {
            t.push_str(&format!("{i} {r}\n"));
        }
        t.into_bytes()
    }

    fn game(layers: &[(&str, u32, Option<Vec<u8>>)]) -> ResMan {
        let mut rm = ResMan::new();
        for (label, p, data) in layers {
            let mut mem = MemContainer::new();
            if let Some(d) = data {
                mem.insert(placeables_key(), d.clone());
            }
            rm.add(*p, *label, LayerClass::Erf, mem);
        }
        rm
    }

    /// The same settings, the same model; another, another. Its model
    /// compiles, and reads back as a light of those settings.
    #[test]
    fn a_light_s_model_is_named_by_its_settings_and_compiles() {
        let a = LightSpec::default();
        let b = LightSpec { radius: 5.004, ..a };
        let c = LightSpec { color: [255, 0, 0], ..a };
        assert_eq!(a.model_name(), b.model_name(), "to a hundredth");
        assert_ne!(a.model_name(), c.model_name());
        assert!(is_generated(a.model_name().as_str().unwrap()));
        assert!(!is_generated("dag_invisible"));
        assert_eq!(a.reach(), 10.0);
        assert_eq!(LightSpec { multiplier: 2.0, ..a }.reach(), 20.0);
        let binary = c.model_binary().unwrap();
        let model = mg_mdl::Model::read(&binary).unwrap();
        let light = model.nodes.iter().find_map(|n| match &n.kind {
            mg_mdl::NodeKind::Light(l) => Some((n, l.clone())),
            _ => None,
        });
        let (node, light) = light.expect("a light node");
        assert!(light.affect_dynamic && !light.shadow && light.priority == 3, "{light:?}");
        assert!(node.name.ends_with("_light"));
        assert_eq!(c.label(), "Light, #ff0000, 10 m");
    }

    /// The row goes at the end of the copy the game reads, its cells as the
    /// game's own lights have them; a hak above that copy, or the chosen
    /// one, takes it; a hak below it is refused; the project's first row is
    /// kept to; written, it reads back, and the same settings again reuse
    /// it.
    #[test]
    fn a_light_s_row_goes_into_the_copy_the_game_reads() {
        let dir = mg_testkit::scratch_dir("mg-lights-plan");
        let base = table(&[
            "Chest **** plc_c01 **** 1 0 0",
            "\"Light, White\" **** dag_invisible 1 1 0 1",
        ]);
        let theirs = table(&[
            "Chest **** plc_c01 **** 1 0 0",
            "\"Light, White\" **** dag_invisible 1 1 0 1",
            "Their_Own **** x_own **** 1 0 0",
        ]);
        let rm = game(&[
            ("hak:theirs", priority::HAK_USER, Some(theirs)),
            ("hak:lights", priority::HAK_USER, None),
            ("game", priority::KEY, Some(base)),
        ]);
        let cp = Codepage::default();
        let spec = LightSpec { color: [255, 128, 0], ..Default::default() };
        let lights = HakChoice { name: "lights".into(), path: dir.join("lights.hak"), new: false };
        // Below the hak whose copy is read: refused, and why.
        let refused = plan(&rm, cp, spec, lights.clone(), None).unwrap_err();
        assert!(refused.contains("hak:theirs") && refused.contains("theirs.hak"), "{refused}");
        // The one whose copy is read: its own row added at its end.
        let theirs_hak =
            HakChoice { name: "theirs".into(), path: dir.join("theirs.hak"), new: false };
        let p = plan(&rm, cp, spec, theirs_hak, None).unwrap();
        assert_eq!((p.row, p.reused, p.table_from.as_str()), (3, false, "hak:theirs"));
        assert_eq!(p.table.get(2, "Label"), Some("Their_Own"), "their rows kept");
        assert_eq!(p.table.get(3, "ModelName"), Some(spec.model_name().as_str().unwrap()));
        assert_eq!(p.table.get(3, "LightColor"), None, "its light is its model's");
        assert_eq!(p.table.get(3, "Static"), Some("1"));
        assert!(p.describe().contains("players must have this hak"), "{}", p.describe());
        // A new hak goes above every hak: it takes the table read; the
        // project's first row kept to.
        let new = new_hak(&dir, "MG_Lights").unwrap();
        assert_eq!(new.name, "mg_lights");
        assert!(new_hak(&dir, "no spaces").is_err() && new_hak(&dir, &"x".repeat(17)).is_err());
        let p = plan(&rm, cp, spec, new.clone(), Some(20000)).unwrap();
        assert_eq!(p.row, 20000);
        assert!(p.describe().contains("a new hak"), "{}", p.describe());
        write(&p, cp).unwrap();
        // Written: the hak has the model and the table, which read back.
        let bytes = std::fs::read(&new.path).unwrap();
        let erf = mg_erf::Erf::read(&bytes).unwrap();
        let model = erf.find(&spec.model_name(), ResType::MDL).expect("the model");
        assert!(mg_mdl::Model::read(&erf.data(model).unwrap()).is_ok());
        let t = erf.find(&ResRef::from_str("placeables").unwrap(), ResType::TWODA).unwrap();
        let t = TwoDa::parse(&erf.data(t).unwrap(), cp).unwrap();
        assert_eq!(t.get(20000, "Label"), Some(spec.label().as_str()));
        assert_eq!(t.get(2, "Label"), Some("Their_Own"));
        // Layered as the module would have it: the same settings reuse it.
        let mut rm =
            game(&[("game", priority::KEY, Some(table(&["Chest **** plc_c01 **** 1 0 0"])))]);
        let hak = mg_resman::ErfContainer::open(&new.path).unwrap();
        rm.add(priority::HAK_USER, "hak:mg_lights", LayerClass::Erf, hak);
        let listed = HakChoice { new: false, ..new };
        let again = plan(&rm, cp, spec, listed.clone(), None).unwrap();
        assert!(again.reused && again.row == 20000, "{again:?}");
        assert!(again.describe().contains("Nothing is written"));
        let other = plan(&rm, cp, LightSpec { radius: 8.0, ..spec }, listed, None).unwrap();
        assert_eq!((other.row, other.reused), (20001, false));
    }

    /// A development folder above the haks with its own table: a new hak
    /// would not be read, and is refused.
    #[test]
    fn a_table_above_the_haks_refuses_a_new_hak() {
        let dir = mg_testkit::scratch_dir("mg-lights-dev");
        let rm = game(&[
            ("development", priority::DEVELOPMENT, Some(table(&["A **** a **** 1 0 0"]))),
            ("game", priority::KEY, Some(table(&["A **** a **** 1 0 0"]))),
        ]);
        let new = new_hak(&dir, "lights").unwrap();
        let e = plan(&rm, Codepage::default(), LightSpec::default(), new, None).unwrap_err();
        assert!(e.contains("development"), "{e}");
        assert!(
            plan(
                &game(&[]),
                Codepage::default(),
                LightSpec::default(),
                new_hak(&dir, "x").unwrap(),
                None
            )
            .is_err()
        );
    }
}
