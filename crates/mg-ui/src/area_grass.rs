//! Area Properties › Grass: the grass an area grows, by surface material
//! (surfacemat.2da row), as the game's `SetAreaGrassOverride` and
//! `SetAreaDefaultGrassDisabled` set it: the tileset's grass on grass
//! faces, or not, and grass of the area's own on any material. Stored
//! where the game reads it when the module loads, the GIT's
//! `AreaProperties` ([`mg_area::grass`]). Every change is an undoable
//! command; what a drag or a color picker is trying out is grown in the
//! area's view as it goes ([`tried`]), and set when it lets go.

use egui::Ui;
use mg_area::grass::{self, AreaGrass, GrassLook, GrassOverride};
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit};
use mg_gff::Struct;
use mg_resman::ResKey;
use mg_rules::{Choice, ChoiceColumns};

use crate::{Action, Moonglow};

/// Where what the page is trying out is kept for the area's view: the
/// frame it was tried at and what grows then.
type Tried = (u64, Vec<(u32, GrassLook)>);

fn tried_id(area: ResRef) -> egui::Id {
    egui::Id::new(("mg-grass-tried", area))
}

/// What the Grass page of `area`'s properties is trying out (a drag, a
/// color picker open), to be grown in its view instead of what the area
/// grows: what grows on each material then. `None` unless it was tried
/// this frame or the one before.
pub(crate) fn tried(ctx: &egui::Context, area: ResRef) -> Option<Vec<(u32, GrassLook)>> {
    let (at, growing) = ctx.data(|d| d.get_temp::<Tried>(tried_id(area)))?;
    (ctx.cumulative_pass_nr() <= at + 1).then_some(growing)
}

/// The surface materials, by row: surfacemat.2da's labels.
fn materials(app: &Moonglow) -> std::sync::Arc<Vec<Choice>> {
    let columns = ChoiceColumns { name: None, label: Some("Label") };
    app.game.as_ref().and_then(|g| g.choice_list("surfacemat", columns).ok()).unwrap_or_default()
}

fn material_name(materials: &[Choice], row: u32) -> String {
    match materials.iter().find(|c| c.row == row as usize) {
        Some(c) => format!("{} ({row})", c.text),
        None => format!("Material {row}"),
    }
}

/// The tileset's grass (`[GRASS]`), read once for each tileset; `None`
/// when the tileset grows none, or cannot be read.
fn tileset_grass(app: &Moonglow, ctx: &egui::Context, tileset: ResRef) -> Option<GrassLook> {
    let id = egui::Id::new(("mg-tileset-grass", tileset));
    if let Some(known) = ctx.data(|d| d.get_temp::<Option<GrassLook>>(id)) {
        return known;
    }
    let game = app.game.as_deref()?;
    let look = mg_area::tileset(game, tileset).ok().and_then(|t| GrassLook::of_tileset(&t));
    ctx.data_mut(|d| d.insert_temp(id, look.clone()));
    look
}

/// The GITs' `AreaProperties` alone (what the edits look at: not the
/// objects, copied every frame), by GIT, of those that have it.
fn properties(app: &mut Moonglow, areas: &[ResRef]) -> Vec<(ResKey, Struct)> {
    let Some(ws) = app.ws.as_mut() else { return Vec::new() };
    areas
        .iter()
        .filter_map(|a| {
            let key = ResKey::new(*a, ResType::GIT);
            let root = &ws.doc(&key).ok()?.root;
            let field =
                root.fields.iter().find(|f| f.label.as_bytes() == grass::PROPERTIES.as_bytes());
            Some((key, Struct { id: root.id, fields: vec![field?.clone()] }))
        })
        .collect()
}

/// Applies the edits `edit` makes of each GIT (its key and its
/// properties), as one command.
fn apply(
    app: &mut Moonglow,
    gits: &[(ResKey, Struct)],
    what: &str,
    edit: impl Fn(ResKey, &Struct) -> Vec<Edit>,
) {
    let edits: Vec<Edit> = gits.iter().flat_map(|(k, root)| edit(*k, root)).collect();
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new(what, edits)));
    }
}

/// The page: of `area`, and of `others` with it (each change is made to
/// each of them; an override, to the one each has for its material).
pub(crate) fn page(app: &mut Moonglow, ui: &mut Ui, area: ResRef, others: &[ResRef]) {
    let areas: Vec<ResRef> = std::iter::once(area).chain(others.iter().copied()).collect();
    let gits = properties(app, &areas);
    let Some((_, first)) = gits.first() else {
        ui.label("This area has no grass settings (its GIT lacks AreaProperties).");
        return;
    };
    let current = AreaGrass::read(first);
    let tileset = app
        .ws
        .as_mut()
        .and_then(|ws| ws.doc(&ResKey::new(area, ResType::ARE)).ok())
        .and_then(|are| are.root.resref("Tileset"))
        .unwrap_or(ResRef::EMPTY);
    let own = tileset_grass(app, ui.ctx(), tileset);
    let materials = materials(app);
    // What is being tried out, in place of the stored values.
    let mut trying = current.clone();
    let mut tried = false;

    ui.label(
        "Grass grows on the tiles' walkmesh faces, by their surface material. The tileset's \
         grows on Grass faces; the area can turn it off, and grow grass of its own on any \
         material (on Grass in place of the tileset's).",
    );
    ui.add_space(6.0);
    crate::widgets::section_heading(ui, "The Tileset's Grass");
    match &own {
        Some(look) => {
            let mut grows = !current.default_disabled;
            if ui
                .checkbox(&mut grows, "Grow the tileset's grass on Grass faces")
                .on_hover_text("Off: SetAreaDefaultGrassDisabled(TRUE) from the start")
                .changed()
            {
                let what = if grows { "Grow the tileset's grass" } else { "No tileset grass" };
                apply(app, &gits, what, |k, _| vec![grass::set_disabled(k, !grows)]);
            }
            ui.weak(format!(
                "{tileset}: texture {}, density {}, height {} m",
                look.texture, look.density, look.height
            ));
            if current.for_material(grass::GRASS_MATERIAL).is_some() {
                ui.weak("The area's own grass on Grass grows in its place.");
            }
        }
        None => {
            ui.weak(format!("The tileset ({tileset}) grows no grass of its own."));
        }
    }
    ui.add_space(8.0);
    crate::widgets::section_heading(ui, "The Area's Own Grass");
    let mut shown: Vec<&GrassOverride> = current.overrides.iter().collect();
    shown.sort_by_key(|o| o.material);
    shown.dedup_by_key(|o| o.material);
    if shown.is_empty() {
        ui.weak("None: add grass to a surface material below.");
    }
    let taken: Vec<u32> = shown.iter().map(|o| o.material).collect();
    for o in shown {
        let o = current.for_material(o.material).unwrap_or(o);
        let id = ui.id().with(("grass", o.material));
        egui::Frame::group(ui.style()).show(ui, |ui| {
            let mut changed = o.clone();
            let mut set = false;
            egui::Grid::new(id).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                crate::widgets::field_label(ui, "Surface Material");
                ui.horizontal(|ui| {
                    // (A list of the table's rows, as everywhere: the
                    // arrow keys step through it. Those other surfaces
                    // have are left out.)
                    let free: Vec<Choice> = (materials.iter())
                        .filter(|c| c.row as u32 == o.material || !taken.contains(&(c.row as u32)))
                        .map(|c| Choice {
                            row: c.row,
                            text: material_name(&materials, c.row as u32),
                        })
                        .collect();
                    let list = crate::widgets::row_choice(
                        id.with("material"),
                        &free,
                        i64::from(o.material),
                    )
                    .shown(material_name(&materials, o.material));
                    let to =
                        list.show(ui).and_then(|v| u32::try_from(v).ok()).unwrap_or(o.material);
                    if to != o.material {
                        let (from, what) = (o.material, "Grass's surface material");
                        apply(app, &gits, what, |k, root| grass::move_override(k, root, from, to));
                    }
                    if ui
                        .button("Remove")
                        .on_hover_text("This material grows no grass of the area's")
                        .clicked()
                    {
                        let m = o.material;
                        apply(app, &gits, "Remove grass", |k, root| {
                            grass::remove_override(k, root, m)
                        });
                    }
                });
                ui.end_row();
                crate::widgets::field_label(ui, "Texture");
                let texture = ResRef::from_str(&o.look.texture).unwrap_or(ResRef::EMPTY);
                let picked = crate::widgets::resref_field(
                    app,
                    ui,
                    id.with("texture"),
                    texture,
                    "Grass texture (four tufts, two by two)",
                    &[ResType::TGA, ResType::DDS],
                );
                if let Some(t) = picked.filter(|t| !t.is_empty()) {
                    changed.look.texture = t.to_string();
                    set = true;
                }
                ui.end_row();
                crate::widgets::field_label(ui, "Density");
                let (v, shown, _) = crate::widgets::drag_number_shown(ui, o.look.density, |d| {
                    d.range(0.0..=50.0).speed(0.05).max_decimals(2)
                });
                changed.look.density = v.unwrap_or(shown);
                set |= v.is_some();
                tried |= v.is_none() && shown != o.look.density;
                ui.end_row();
                crate::widgets::field_label(ui, "Height");
                let (v, shown, _) = crate::widgets::drag_number_shown(ui, o.look.height, |d| {
                    d.range(0.0..=10.0).speed(0.01).max_decimals(2).suffix(" m")
                });
                changed.look.height = v.unwrap_or(shown);
                set |= v.is_some();
                tried |= v.is_none() && shown != o.look.height;
                ui.end_row();
                for (text, which) in [("Ambient Color", 0), ("Diffuse Color", 1)] {
                    crate::widgets::field_label(ui, text);
                    let rgb = if which == 0 { o.look.ambient } else { o.look.diffuse };
                    let (now, open, done) = color(ui, id.with(text), rgb);
                    let slot = if which == 0 {
                        &mut changed.look.ambient
                    } else {
                        &mut changed.look.diffuse
                    };
                    *slot = now;
                    set |= done;
                    tried |= open && now != rgb;
                    ui.end_row();
                }
            });
            if set && changed != *o {
                apply(app, &gits, "Grass", |k, root| grass::set_override(k, root, &changed));
            } else if changed != *o
                && let Some(slot) =
                    trying.overrides.iter_mut().rev().find(|t| t.material == o.material)
            {
                *slot = changed;
            }
        });
    }
    ui.horizontal(|ui| {
        let free: Vec<&Choice> =
            materials.iter().filter(|c| !taken.contains(&(c.row as u32))).collect();
        let mut pick: Option<u32> = None;
        ui.menu_button("Add Grass to…", |ui| {
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                for c in free {
                    if ui.button(material_name(&materials, c.row as u32)).clicked() {
                        pick = Some(c.row as u32);
                    }
                }
            });
        })
        .response
        .on_hover_text("Grass of the area's own on a surface material: the tileset's, as a start");
        if let Some(m) = pick {
            let set = app.game.as_deref().and_then(|g| mg_area::tileset(g, tileset).ok());
            let o = GrassOverride::new(m, set.as_ref());
            apply(app, &gits, "Add grass", |k, root| grass::add_override(k, root, &o));
        }
    });
    ui.add_space(8.0);
    ui.weak(
        "Kept in the area's GIT (AreaProperties: GrassDefDisabled, GrassOverrides), where the \
         game reads it when the module loads; scripts change it while the game runs with \
         SetAreaGrassOverride, RemoveAreaGrassOverride and SetAreaDefaultGrassDisabled.",
    );
    if tried {
        let growing = trying.growing_over(own);
        let now = ui.ctx().cumulative_pass_nr();
        ui.ctx().data_mut(|d| d.insert_temp::<Tried>(tried_id(area), (now, growing)));
    }
}

/// A color (red, green and blue, 0 to 1) with a color picker: the color
/// shown (picked so far, while the picker is open), whether the picker is
/// open, and whether it closed on another color (to be set).
fn color(ui: &mut Ui, id: egui::Id, rgb: [f32; 3]) -> ([f32; 3], bool, bool) {
    let pending_id = id.with("pending");
    let pending: Option<[f32; 3]> = ui.data(|d| d.get_temp(pending_id));
    let shown = pending.unwrap_or(rgb);
    let mut bytes = shown.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
    let (changed, open) = ui
        .push_id(id, |ui| {
            let popup = ui.auto_id_with("popup");
            let changed = crate::color_button::edit_srgb(ui, &mut bytes).changed();
            (changed, egui::Popup::is_id_open(ui.ctx(), popup))
        })
        .inner;
    let picked = if changed { bytes.map(|b| f32::from(b) / 255.0) } else { shown };
    if changed {
        ui.data_mut(|d| d.insert_temp(pending_id, picked));
    }
    if !open && ui.data(|d| d.get_temp::<[f32; 3]>(pending_id)).is_some() {
        ui.data_mut(|d| d.remove::<[f32; 3]>(pending_id));
        return (picked, false, picked != rgb);
    }
    (picked, open, false)
}
