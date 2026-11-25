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
//! In the client (`model_light_uniforms`), placeable model light nodes light
//! dynamic draws when `affectdynamic` is set ([`LightSpec::lights_objects`]),
//! and each draw keeps at most 32 lights sorted by `lightpriority`, then
//! distance. The node marks whether it casts shadows ([`LightSpec::shadows`]);
//! its priority ([`LightSpec::priority`]): either answer changes a
//! default, not what is written.
//!
//! A light can flicker ([`Flicker`]): its node's `color` keyed over a
//! short cycle in the model's `default` animation, which the client plays,
//! cycled, on a dynamic placeable and not on a static one (2026-10-30,
//! `reading_static_placeable_default_animation`). A flickering light's row
//! is `Static` 0 and its placed instances are dynamic
//! ([`dynamic_if_flickering`]); a steady light writes what it always did.

use std::path::{Path, PathBuf};

use mg_2da::TwoDa;
use mg_core::{Codepage, ResRef, ResType};
use mg_gff::{Struct, Value};
use mg_resman::{ResKey, ResMan, priority};

/// (Kept in mg-rules, which the area's view tells them by without this
/// crate.)
pub use mg_rules::lights::{MODEL_PREFIX, is_generated};

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
    /// How it flickers: none, or its color keyed in its model's `default`
    /// (a dynamic placeable then).
    pub flicker: Flicker,
}

/// A light's flicker: its brightness keyed over a short cycle in its
/// model's `default` animation, as the light node's `colorkey` (the color
/// scaled; the client's light node keys its color, measured, and its
/// multiplier lengthens the reach, not the brightness).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Flicker {
    /// A steady light (a static placeable, as the game's own lights).
    #[default]
    None,
    /// Small, quick, slight.
    Candle,
    /// Quicker and deeper.
    Torch,
    /// Quick and deep, as a fire's light leaps.
    Fire,
    /// Slow, even swelling and fading.
    Pulse,
}

impl Flicker {
    pub const ALL: [Flicker; 5] =
        [Flicker::None, Flicker::Candle, Flicker::Torch, Flicker::Fire, Flicker::Pulse];

    /// Its name, as the Custom Light section lists it.
    pub fn name(self) -> &'static str {
        match self {
            Flicker::None => "None",
            Flicker::Candle => "Candle",
            Flicker::Torch => "Torch",
            Flicker::Fire => "Fire",
            Flicker::Pulse => "Pulse",
        }
    }

    /// The cycle: its length, seconds, and the brightness (a share of the
    /// color) at even steps through it; the first again at its end, so it
    /// loops without a jump. `None` for a steady light.
    pub fn cycle(self) -> Option<(f32, &'static [f32])> {
        match self {
            Flicker::None => None,
            Flicker::Candle => {
                Some((2.0, &[1.0, 0.92, 0.97, 0.88, 1.0, 0.94, 0.85, 0.96, 0.9, 1.0, 0.93, 0.97]))
            }
            Flicker::Torch => Some((1.5, &[1.0, 0.8, 0.95, 0.7, 0.9, 1.0, 0.75, 0.88, 0.82, 0.97])),
            Flicker::Fire => {
                Some((1.2, &[1.0, 0.6, 0.85, 0.5, 0.95, 0.7, 1.0, 0.55, 0.8, 0.65, 0.9, 0.6]))
            }
            Flicker::Pulse => Some((2.4, &[1.0, 0.93, 0.76, 0.56, 0.42, 0.56, 0.76, 0.93])),
        }
    }
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
            flicker: Flicker::None,
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
        self.hashed_name(f32::round)
    }

    /// The name Moonglow gave the model of these settings before its
    /// hundredths were rounded, when they were cut: 0.53 m (52.999996
    /// hundredths in an f32) was hashed as 52, the name of 0.52 m too. A light placed
    /// then keeps its row: [`plan`] finds it by this name.
    fn name_before_rounding(&self) -> ResRef {
        self.hashed_name(f32::trunc)
    }

    fn hashed_name(&self, hundredths: fn(f32) -> f32) -> ResRef {
        let s = self.normalized();
        let mut text = format!(
            "{:?} {} {} {} {} {}",
            s.color,
            hundredths(s.radius * 100.0) as u32,
            hundredths(s.multiplier * 100.0) as u32,
            s.shadows,
            s.lights_objects,
            s.priority
        );
        // (A steady light's text as it was before lights flickered: its
        // name, and its model, the same.)
        if s.flickers() {
            text += &format!(" {}", s.flicker.name());
        }
        // (FNV-1a, 64 bits; 48 of them kept.)
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in text.bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
        }
        let name = format!("{MODEL_PREFIX}{:012x}", h & 0xffff_ffff_ffff);
        ResRef::from_str(&name).expect("16 characters")
    }

    /// It flickers: its placeable must be dynamic for the game to play it.
    pub fn flickers(&self) -> bool {
        self.flicker != Flicker::None
    }

    /// Its model as text: a dummy, a square of 0.2 m drawn by nothing (as
    /// the game's invisible light placeables have), and the light; a
    /// flickering light's `default` animation keys the light's color.
    pub fn model_text(&self) -> String {
        self.model_text_named(self.model_name())
    }

    /// [`model_text`](Self::model_text), its model named `name`.
    fn model_text_named(&self, name: ResRef) -> String {
        let s = self.normalized();
        let n = name.to_string();
        let [r, g, b] = s.color.map(|c| f32::from(c) / 255.0);
        let mut text = format!(
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
        );
        if let Some(anim) = flicker_animation(&n, s.flicker, [r, g, b]) {
            let done = format!("donemodel {n}\n");
            text.truncate(text.len() - done.len());
            text += &anim;
            text += &done;
        }
        text
    }

    /// Its model compiled, as the game's own are.
    pub fn model_binary(&self) -> Result<Vec<u8>, String> {
        self.model_binary_named(self.model_name())
    }

    /// [`model_binary`](Self::model_binary), its model named `name`.
    fn model_binary_named(&self, name: ResRef) -> Result<Vec<u8>, String> {
        let text = self.model_text_named(name);
        let compiled = mg_mdl::compile::compile(text.as_bytes(), &Default::default())
            .map_err(|e| format!("{name}: {e}"))?;
        Ok(compiled.binary)
    }

    /// Its placeables.2da row's label.
    pub fn label(&self) -> String {
        let s = self.normalized();
        let [r, g, b] = s.color;
        let label = format!("Light, #{r:02x}{g:02x}{b:02x}, {} m", short(s.reach()));
        match s.flicker {
            Flicker::None => label,
            f => format!("{label}, {} flicker", f.name().to_lowercase()),
        }
    }
}

/// A flickering light's `default` animation (model `n`, its light
/// `{n}_light`): the color scaled by the cycle's brightness at each key,
/// as the client's own reading keyed one (`length`, `transtime 0`, the
/// root as `animroot`).
fn flicker_animation(n: &str, flicker: Flicker, [r, g, b]: [f32; 3]) -> Option<String> {
    let (length, steps) = flicker.cycle()?;
    let step = length / steps.len() as f32;
    let keys: Vec<String> = steps
        .iter()
        .chain(steps.first())
        .enumerate()
        .map(|(i, k)| {
            let t = i as f32 * step;
            format!("    {t:.4} {:.4} {:.4} {:.4}\n", r * k, g * k, b * k)
        })
        .collect();
    Some(format!(
        "newanim default {n}\n  length {length}\n  transtime 0\n  animroot {n}\n\
         node dummy {n}\n  parent NULL\nendnode\n\
         node light {n}_light\n  parent {n}\n  colorkey {}\n{}endnode\n\
         doneanim default {n}\n",
        keys.len(),
        keys.concat()
    ))
}

/// Whether placeables.2da's row `row` is a light Moonglow made that
/// flickers: its model one of [`MODEL_PREFIX`]'s and the row `Static` 0, as
/// [`plan`] writes a flickering light's.
pub fn row_flickers(table: &TwoDa, row: usize) -> bool {
    table.get(row, "ModelName").is_some_and(is_generated)
        && table.get(row, "Static").is_some_and(|s| s.trim() == "0")
}

/// The rows of placeables.2da that are flickering lights ([`row_flickers`]):
/// what Make Placeables Static leaves dynamic.
pub fn flickering_rows(table: &TwoDa) -> std::collections::BTreeSet<u32> {
    (0..table.len())
        .filter(|&r| row_flickers(table, r))
        .filter_map(|r| u32::try_from(r).ok())
        .collect()
}

/// A placeable instance (or blueprint) whose `Appearance` is a flickering
/// light's row made dynamic, as the client plays a dynamic placeable's
/// `default` and holds a static one's still. Whether it was changed.
pub fn dynamic_if_flickering(item: &mut Struct, table: &TwoDa) -> bool {
    let row = item.integer("Appearance").and_then(|r| usize::try_from(r).ok());
    row.is_some_and(|r| row_flickers(table, r)) && make_flickering(item)
}

/// A flickering light's placeable made dynamic, and Plot: a static
/// placeable can't be destroyed, a dynamic one can (an area spell, a
/// fireball, takes placeables), and a light should not be. Whether it was
/// changed.
pub fn make_flickering(item: &mut Struct) -> bool {
    let mut changed = false;
    for (label, value) in [("Static", 0), ("Plot", 1)] {
        if item.integer(label) != Some(value) {
            item.set(label, Value::Byte(value as u8));
            changed = true;
        }
    }
    changed
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
    /// The placeables.2da it writes: the copy the game reads, with its row.
    pub fn table(&self) -> &TwoDa {
        &self.table
    }

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
    // (Not the walkmesh bake's pending rows' layer, which is over every hak
    // while a baked row waits for the project's build: no hak can be moved
    // above it. Nor the module file's own copy, which the engine does not
    // read: a light numbered after its rows, in a table made from it, was
    // not the table the game had.)
    let top = crate::bake::placeables_layer(rm).ok_or("the game data has no placeables.2da")?;
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
    // (The row of these settings by their name, or by the name they had
    // before their hundredths were rounded: a light placed then is the
    // same light, not a second row.)
    let row_named = |name: ResRef| {
        let text = name.to_string();
        (0..table.len())
            .find(|&r| table.get(r, "ModelName").is_some_and(|m| m.eq_ignore_ascii_case(&text)))
            .map(|r| (r, name))
    };
    // (The earlier name was shared by two settings a hundredth apart,
    // 0.52 and 0.53 m: its row is this light's only where its model in the
    // hak has this radius and multiplier, or, the model not there, the row
    // has this light's label.)
    let earlier = spec.name_before_rounding();
    let this_light = |row: usize| match at
        .and_then(|i| layers[i].container.read(&ResKey::new(earlier, ResType::MDL)).ok())
    {
        Some(model) => model_has(&model, &spec),
        None => table.get(row, "Label").is_some_and(|l| l == spec.label()),
    };
    let found = row_named(spec.model_name())
        .or_else(|| row_named(earlier).filter(|&(row, _)| this_light(row)));
    let model = found.map_or(spec.model_name(), |(_, name)| name);
    let had = found.map(|(r, _)| r);
    let model_text = model.to_string();
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
            // The row it would take: held to the last the game can use
            // before the table is padded to it.
            let next = table.len().max(first_row.unwrap_or(0));
            let last = crate::bake::LAST_ROW as usize;
            if next > last {
                let why = match first_row {
                    Some(first) if first > last => format!("the project's first_row is {first}"),
                    _ => format!("placeables.2da has {} rows", table.len()),
                };
                return Err(format!(
                    "a light would take row {next} of placeables.2da, past row {last}, the last \
                     the game can use ({why}): it carries an object's row as a 16-bit number and \
                     would draw another row's model"
                ));
            }
            while table.len() < next {
                table.add_row();
            }
            let row = table.add_row();
            // (As the game's own invisible lights have it, "Light, White":
            // a static placeable with a shadow of 1, its light its model's.)
            let cells = [
                ("Label", spec.label()),
                ("ModelName", model_text),
                ("ShadowSize", "1".into()),
                ("BodyBag", "0".into()),
                // (A flickering light's is dynamic: the game plays no
                // static placeable's animation.)
                ("Static", if spec.flickers() { "0" } else { "1" }.into()),
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

/// Whether a compiled light model has `spec`'s radius and multiplier.
fn model_has(model: &[u8], spec: &LightSpec) -> bool {
    let Ok(model) = mg_mdl::Model::read(model) else { return false };
    let near = |n: &mg_mdl::Node, name: &str, want: f32| {
        n.value(name).and_then(|v| v.first()).is_some_and(|v| (v - want).abs() < 0.005)
    };
    model.nodes.iter().any(|n| {
        matches!(n.kind, mg_mdl::NodeKind::Light(_))
            && near(n, "radius", spec.radius)
            && near(n, "multiplier", spec.multiplier)
    })
}

/// Writes what `plan` worked out: the model and placeables.2da into its
/// hak (made if new; one that is there kept beside itself as `.bak`
/// first). Nothing for a plan that reuses what is there.
pub fn write(plan: &Plan, codepage: Codepage) -> Result<(), String> {
    if plan.reused {
        return Ok(());
    }
    use crate::hak_edit::Source;
    let model = plan.spec.model_binary_named(plan.model)?;
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

    /// A module file's own placeables.2da is not the copy the game reads
    /// (the engine reads none of a module's: `engine_placeable_walkmesh`),
    /// so a light's table is the game's or a hak's with its row added,
    /// as the walkmesh bake's is ([`crate::bake::table_under`]): made from
    /// the module's copy, it had rows the game never showed and lacked
    /// the hak-less game's own.
    #[test]
    fn a_module_s_own_table_is_not_the_copy_the_game_reads() {
        let dir = mg_testkit::scratch_dir("mg-lights-module-table");
        let base = table(&["Chest **** plc_c01 **** 1 0 0", "Lamp **** plc_lamp **** 1 0 0"]);
        let own = table(&["Mine **** x_mine **** 1 0 0"]);
        let rm = game(&[
            ("module", priority::MODULE, Some(own.clone())),
            ("game", priority::KEY, Some(base.clone())),
        ]);
        let (cp, spec) = (Codepage::default(), LightSpec::default());
        let p = plan(&rm, cp, spec, new_hak(&dir, "lights").unwrap(), None).unwrap();
        assert_eq!((p.row, p.table_from.as_str()), (2, "game"));
        assert_eq!(p.table.get(0, "Label"), Some("Chest"));
        let (under, from) = crate::bake::table_under(&rm, cp).unwrap();
        assert_eq!((under.len(), from), (2, p.table_from.clone()), "as the bake reads it");
        // With a hak's above it: the hak's, as before.
        let theirs = table(&["Chest **** plc_c01 **** 1 0 0"]);
        let rm = game(&[
            ("hak:theirs", priority::HAK_USER, Some(theirs)),
            ("module", priority::MODULE, Some(own)),
            ("game", priority::KEY, Some(base)),
        ]);
        let p = plan(&rm, cp, spec, new_hak(&dir, "lights").unwrap(), None).unwrap();
        assert_eq!((p.row, p.table_from.as_str()), (1, "hak:theirs"));
    }

    /// While a baked row waits for the project's build, Moonglow reads
    /// placeables.2da from a layer of its own over the haks: a light still
    /// goes into the hak whose copy the game reads, listed or new (the
    /// review's V3: both were refused, with advice to move the hak above a
    /// layer that is no hak).
    #[test]
    fn pending_baked_rows_do_not_refuse_a_light() {
        let dir = mg_testkit::scratch_dir("mg-lights-pending");
        let base = table(&["Chest **** plc_c01 **** 1 0 0"]);
        let theirs = table(&["Chest **** plc_c01 **** 1 0 0", "Their_Own **** x_own **** 1 0 0"]);
        let pending = table(&[
            "Chest **** plc_c01 **** 1 0 0",
            "Their_Own **** x_own **** 1 0 0",
            "Baked **** mgbk_0001 **** 1 0 0",
        ]);
        let rm = game(&[
            (crate::bake::PENDING_LABEL, crate::bake::PENDING_PRIORITY, Some(pending)),
            ("hak:theirs", priority::HAK_USER, Some(theirs)),
            ("game", priority::KEY, Some(base)),
        ]);
        let (cp, spec) = (Codepage::default(), LightSpec::default());
        let listed = HakChoice { name: "theirs".into(), path: dir.join("theirs.hak"), new: false };
        let p = plan(&rm, cp, spec, listed, None).unwrap();
        assert_eq!((p.row, p.table_from.as_str()), (2, "hak:theirs"));
        let p = plan(&rm, cp, spec, new_hak(&dir, "lights").unwrap(), None).unwrap();
        assert_eq!((p.row, p.table_from.as_str()), (2, "hak:theirs"));
    }

    /// A light's row is one the game can carry (16 bits): a first row past
    /// 65535 is refused before the table is padded to it (the review's V8:
    /// four billion rows were added, or row 70000 written, which the game
    /// draws as row 4464), and the last row itself is taken.
    #[test]
    fn a_row_past_the_last_the_game_carries_is_refused() {
        let dir = mg_testkit::scratch_dir("mg-lights-last-row");
        let rm = game(&[("game", priority::KEY, Some(table(&["A **** a **** 1 0 0"])))]);
        let (cp, spec) = (Codepage::default(), LightSpec::default());
        let new = || new_hak(&dir, "lights").unwrap();
        for first in [65536, 70000, 4_000_000_000] {
            let e = plan(&rm, cp, spec, new(), Some(first)).unwrap_err();
            assert!(e.contains("past row 65535") && e.contains("first_row"), "{e}");
        }
        assert_eq!(plan(&rm, cp, spec, new(), Some(65535)).unwrap().row, 65535);
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

    /// A light placed before the hundredths were rounded (its row named by
    /// them cut) is found again by its settings: re-placed, it reuses its
    /// row and model, no second row; its model missing, the row is kept and
    /// the model written under the row's name.
    #[test]
    fn a_light_placed_before_rounding_keeps_its_row() {
        let dir = mg_testkit::scratch_dir("mg-lights-before-rounding");
        let spec = LightSpec { radius: 0.53, ..Default::default() };
        let before = spec.name_before_rounding();
        assert_ne!(before, spec.model_name(), "0.53 (52.999996 hundredths) was cut to 52");
        let label = spec.label();
        let rows = table(&[
            "Chest **** plc_c01 **** 1 0 0",
            &format!("\"{label}\" **** {before} **** 1 0 1"),
        ]);
        let cp = Codepage::default();
        let with = |model: Option<Vec<u8>>, rows: &[u8]| {
            let mut hak = MemContainer::new();
            hak.insert(placeables_key(), rows.to_vec());
            if let Some(m) = model {
                hak.insert(ResKey::new(before, ResType::MDL), m);
            }
            let mut rm = ResMan::new();
            rm.add(priority::HAK_USER, "hak:lights", LayerClass::Erf, hak);
            rm
        };
        let lights = HakChoice { name: "lights".into(), path: dir.join("lights.hak"), new: false };
        let model = spec.model_binary_named(before).unwrap();
        let p = plan(&with(Some(model), &rows), cp, spec, lights.clone(), None).unwrap();
        assert_eq!((p.row, p.reused, p.model), (1, true, before));
        // The model gone from the hak: the row (its label this light's)
        // kept, its model written again.
        let p = plan(&with(None, &rows), cp, spec, lights.clone(), None).unwrap();
        assert_eq!((p.row, p.reused, p.model, p.table.len()), (1, false, before, 2));
        write(&p, cp).unwrap();
        let bytes = std::fs::read(dir.join("lights.hak")).unwrap();
        let erf = mg_erf::Erf::read(&bytes).unwrap();
        assert!(erf.find(&before, ResType::MDL).is_some());

        // The earlier name was also the 0.52 m light's (its name then and
        // now): its row and model are that light's, and a 0.53 m light
        // takes a row of its own, never 0.52's.
        let other = LightSpec { radius: 0.52, ..Default::default() };
        assert_eq!(other.model_name(), before);
        let rows52 = table(&[
            "Chest **** plc_c01 **** 1 0 0",
            &format!("\"{}\" **** {before} **** 1 0 1", other.label()),
        ]);
        for model in [Some(other.model_binary().unwrap()), None] {
            let p = plan(&with(model, &rows52), cp, spec, lights.clone(), None).unwrap();
            assert_eq!((p.row, p.reused, p.model), (2, false, spec.model_name()));
        }
        // And the 0.52 m light itself finds its row by its own name.
        let p = plan(&with(None, &rows52), cp, other, lights, None).unwrap();
        assert_eq!((p.row, p.model), (1, before));
    }

    fn fnv(bytes: &[u8]) -> u64 {
        bytes
            .iter()
            .fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3))
    }

    /// A steady light writes what it wrote before lights could flicker:
    /// its model's name, text and compiled bytes, its label and its row's
    /// `Static` 1 (pinned from the code before; this test passed there).
    #[test]
    fn a_steady_light_writes_what_it_wrote_before_flicker() {
        let other = LightSpec {
            color: [255, 60, 20],
            radius: 6.0,
            multiplier: 1.5,
            shadows: true,
            lights_objects: false,
            priority: 1,
            flicker: Flicker::None,
        };
        let pins = [
            (
                LightSpec::default(),
                "mgl_5b0a9fcccd8a",
                0x344d_a681_34b1_1f2e,
                0xabe6_8ea5_1ca8_5e51,
                "Light, #ffc88c, 10 m",
            ),
            (
                other,
                "mgl_b1457bbc4677",
                0x0b49_e38b_244d_f26c,
                0x9fba_ee5a_d765_1021,
                "Light, #ff3c14, 18 m",
            ),
        ];
        for (spec, name, text, binary, label) in pins {
            assert!(!spec.flickers());
            assert_eq!(spec.model_name().to_string(), name);
            assert_eq!(fnv(spec.model_text().as_bytes()), text, "{}", spec.model_text());
            assert_eq!(fnv(&spec.model_binary().unwrap()), binary);
            assert_eq!(spec.label(), label);
        }
        let rm = game(&[("game", priority::KEY, Some(table(&["A **** a **** 1 0 0"])))]);
        let dir = mg_testkit::scratch_dir("mg-lights-steady");
        let p =
            plan(&rm, Codepage::default(), LightSpec::default(), new_hak(&dir, "l").unwrap(), None)
                .unwrap();
        assert_eq!(p.table.get(p.row, "Static"), Some("1"));
        assert!(!row_flickers(&p.table, p.row));
    }

    /// A flickering light: a model of its own (not the steady light's),
    /// labelled so, whose `default` animation keys the light's color over
    /// its cycle (the first key again at its end), compiled into the binary
    /// model; its row is dynamic (`Static` 0), and an instance of the row
    /// is made dynamic, a steady light's left as it is.
    #[test]
    fn a_flickering_light_keys_its_color_in_its_default_animation() {
        let steady = LightSpec { color: [200, 100, 50], ..Default::default() };
        let mut names = std::collections::HashSet::new();
        for flicker in Flicker::ALL {
            let spec = LightSpec { flicker, ..steady };
            assert!(names.insert(spec.model_name()), "each its own model: {flicker:?}");
            if flicker == Flicker::None {
                assert!(!spec.model_text().contains("newanim"));
                continue;
            }
            assert!(spec.flickers());
            let (length, steps) = flicker.cycle().unwrap();
            assert!(steps.iter().all(|k| (0.0..=1.0).contains(k)) && steps[0] == 1.0);
            assert_eq!(
                spec.label(),
                format!("Light, #c86432, 10 m, {} flicker", flicker.name().to_lowercase())
            );
            // The compiled model (as the game reads it) has the keys.
            let model = mg_mdl::Model::read(&spec.model_binary().unwrap()).unwrap();
            let anim = model.animation("default").expect("its default animation");
            assert!((anim.length - length).abs() < 1e-4, "{flicker:?}: {}", anim.length);
            let light = format!("{}_light", spec.model_name());
            let node = anim.nodes.iter().find(|n| n.name.eq_ignore_ascii_case(&light));
            let node = node.unwrap_or_else(|| panic!("{flicker:?}: {anim:#?}"));
            let color = node.controllers.iter().find(|c| c.name == "color").expect("colorkey");
            assert_eq!((color.columns, color.times.len()), (3, steps.len() + 1), "{flicker:?}");
            assert_eq!(color.times[0], 0.0);
            assert!((color.times[steps.len()] - length).abs() < 1e-3, "{:?}", color.times);
            let row = |i: usize| &color.values[i * 3..i * 3 + 3];
            assert_eq!(row(0), row(steps.len()), "the cycle closes");
            let base = steady.color.map(|c| f32::from(c) / 255.0);
            for (i, k) in steps.iter().enumerate() {
                for (got, want) in row(i).iter().zip(base) {
                    assert!((got - want * k).abs() < 1e-3, "{flicker:?} key {i}");
                }
            }
        }
        // Its row: dynamic; an instance of it made dynamic.
        let rm = game(&[("game", priority::KEY, Some(table(&["A **** a **** 1 0 0"])))]);
        let dir = mg_testkit::scratch_dir("mg-lights-flicker");
        let cp = Codepage::default();
        let torch = LightSpec { flicker: Flicker::Torch, ..steady };
        let p = plan(&rm, cp, torch, new_hak(&dir, "l").unwrap(), None).unwrap();
        assert_eq!((p.row, p.table.get(1, "Static")), (1, Some("0")));
        assert!(row_flickers(&p.table, 1) && !row_flickers(&p.table, 0));
        assert_eq!(flickering_rows(&p.table).into_iter().collect::<Vec<_>>(), [1]);
        let placed = |row: u32| {
            let mut s = Struct::new(9);
            s.set("Appearance", Value::Dword(row));
            s.set("Static", Value::Byte(1));
            s
        };
        let mut lit = placed(1);
        assert!(dynamic_if_flickering(&mut lit, &p.table));
        assert_eq!((lit.integer("Static"), lit.integer("Plot")), (Some(0), Some(1)));
        assert!(!dynamic_if_flickering(&mut lit, &p.table), "already dynamic");
        let mut other = placed(0);
        assert!(!dynamic_if_flickering(&mut other, &p.table));
        assert_eq!(other.integer("Static"), Some(1));
        let mut past = placed(7);
        assert!(!dynamic_if_flickering(&mut past, &p.table), "a row past the table's end");
    }

    #[test]
    fn model_name_rounds_hundredths_without_float_truncation_collisions() {
        for (a, b) in [(0.52_f32, 0.53_f32), (0.28_f32, 0.29_f32), (0.57_f32, 0.58_f32)] {
            let r_a = LightSpec { radius: a, ..LightSpec::default() };
            let r_b = LightSpec { radius: b, ..LightSpec::default() };
            assert_ne!(r_a.model_name(), r_b.model_name(), "radius {a} vs {b}");

            let m_a = LightSpec { multiplier: a, ..LightSpec::default() };
            let m_b = LightSpec { multiplier: b, ..LightSpec::default() };
            assert_ne!(m_a.model_name(), m_b.model_name(), "multiplier {a} vs {b}");
        }
    }
}
