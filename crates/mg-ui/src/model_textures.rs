//! A model's textures, where a builder looks at a tile: the tileset
//! editor's tile, Tile Properties and a tile's view. Each texture the
//! model draws, with a small picture: what draws it (which node, as which
//! slot, through which material), where the load order finds it (the base
//! game, a hak by name, the override folder, the module) with its format
//! and size, and those found nowhere marked missing. The listing is
//! `mg_preview::replaceable`'s ([`uses`], [`texture_file`]), the walk the
//! Visuals page's texture replacements come from too.
//!
//! [`uses`]: mg_preview::replaceable::uses
//! [`texture_file`]: mg_preview::replaceable::texture_file

use std::sync::Arc;

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_mdl::Model;
use mg_preview::replaceable::{Role, Source, TextureFile, TextureUse, texture_file, uses};
use mg_resman::ResKey;

use crate::{Action, Moonglow, Tab};

/// The side of a texture's picture in a row, points.
const PICTURE: f32 = 32.0;
/// The size its picture is made at, pixels.
const THUMB: u32 = 64;
/// How many of a use's nodes are named before "and N more".
const NODES_NAMED: usize = 4;

/// What a model draws, listed once for the module's revision and the game
/// data (Tile Properties draws it each frame).
#[derive(Debug, Default)]
struct Listing {
    /// Why there is nothing to list: the model is found nowhere, or can't
    /// be read.
    problem: Option<String>,
    rows: Vec<Row>,
}

/// A texture of the model, and every use of it.
#[derive(Debug)]
struct Row {
    name: String,
    uses: Vec<TextureUse>,
    file: Option<TextureFile>,
    /// The layers of the materials it is drawn through, by name.
    materials: Vec<(String, Option<String>)>,
    /// It is the tileset's environment map (`EnvMap`), which the meshes
    /// reflect where nothing else is named.
    area_env: bool,
}

impl Row {
    /// Drawn by any of its uses.
    fn drawn(&self) -> bool {
        self.area_env || self.uses.iter().any(|u| u.drawn)
    }

    /// Drawn and found nowhere.
    fn missing(&self) -> bool {
        self.file.is_none() && self.drawn()
    }
}

/// What a listing was read for: the module's revision and the game data.
type Stamp = (Option<u64>, usize);

/// The model `model`'s textures: what [`section`] shows.
fn listing(app: &Moonglow, ui: &Ui, model: &str, area_env: Option<&str>) -> Arc<Listing> {
    let Some(game) = app.game.clone() else {
        return Arc::new(Listing { problem: Some("No game data".into()), rows: Vec::new() });
    };
    let model = model.trim().to_ascii_lowercase();
    let area_env = area_env.map(str::to_ascii_lowercase).filter(|e| !e.is_empty());
    let id = egui::Id::new(("model-textures", &model, &area_env));
    let stamp: Stamp = (app.ws.as_ref().map(|w| w.revision()), Arc::as_ptr(&game) as usize);
    if let Some((was, l)) = ui.data(|d| d.get_temp::<(Stamp, Arc<Listing>)>(id))
        && was == stamp
    {
        return l;
    }
    let l = Arc::new(read(&game, &model, area_env.as_deref()));
    ui.data_mut(|d| d.insert_temp(id, (stamp, l.clone())));
    l
}

/// The listing of `model`, read from the game data.
fn read(game: &mg_rules::GameData, model: &str, area_env: Option<&str>) -> Listing {
    let Some(key) = ResKey::parse(model, ResType::MDL) else {
        return Listing {
            problem: Some(format!("{model} is not a model's name")),
            rows: Vec::new(),
        };
    };
    let Ok(data) = game.resman.get(&key) else {
        let problem = format!(
            "The model {model} isn't in the game data (a hak the module uses, development, \
             override)"
        );
        return Listing { problem: Some(problem), rows: Vec::new() };
    };
    let m = match Model::read(&data) {
        Ok(m) => m,
        Err(e) => {
            let problem = format!("The model {model} can't be read: {e}");
            return Listing { problem: Some(problem), rows: Vec::new() };
        }
    };
    let mut rows: Vec<Row> = Vec::new();
    for u in uses(game, &m, area_env) {
        match rows.iter_mut().find(|r| r.name == u.name) {
            Some(r) => r.uses.push(u),
            None => rows.push(Row {
                name: u.name.clone(),
                uses: vec![u],
                file: None,
                materials: Vec::new(),
                area_env: false,
            }),
        }
    }
    // The tileset's environment map: what every lit mesh reflects in its
    // specular light where its texture names none (the renderer's
    // `texUnitEnv`), whether or not a TXI's `default` names it.
    if let Some(env) = area_env {
        match rows.iter_mut().find(|r| r.name == env) {
            Some(r) => r.area_env = true,
            None => rows.push(Row {
                name: env.to_string(),
                uses: Vec::new(),
                file: None,
                materials: Vec::new(),
                area_env: true,
            }),
        }
    }
    for r in &mut rows {
        r.file = texture_file(game, &r.name);
        let mut materials: Vec<String> = r.uses.iter().filter_map(|u| u.material.clone()).collect();
        materials.sort();
        materials.dedup();
        r.materials = materials
            .into_iter()
            .map(|m| {
                let layer = ResRef::cut(&m)
                    .and_then(|r| game.resman.origin(&ResKey::new(r, ResType::MTR)))
                    .map(str::to_string);
                (m, layer)
            })
            .collect();
    }
    // By name; the tileset's map alone (no use of the model's) last.
    rows.sort_by(|a, b| (a.uses.is_empty(), &a.name).cmp(&(b.uses.is_empty(), &b.name)));
    Listing { problem: None, rows }
}

/// What a texture's uses are, a line each kind: what it is drawn as, by
/// which nodes, through what.
fn use_lines(row: &Row) -> Vec<(String, String)> {
    // (Uses that differ only by node are one line.)
    let mut kinds: Vec<(&TextureUse, Vec<&str>)> = Vec::new();
    for u in &row.uses {
        let same = |k: &&mut (&TextureUse, Vec<&str>)| {
            k.0.role == u.role
                && k.0.source == u.source
                && k.0.material == u.material
                && k.0.drawn == u.drawn
        };
        match kinds.iter_mut().find(same) {
            Some(k) => k.1.push(&u.node),
            None => kinds.push((u, vec![&u.node])),
        }
    }
    let mut out: Vec<(String, String)> = kinds
        .iter()
        .map(|(u, nodes)| {
            let all = nodes.join(", ");
            let on = crate::text::some_of(nodes, NODES_NAMED);
            let line = match (&u.role, &u.source) {
                (Role::Slot(i), Source::Mesh) if !u.drawn => {
                    let slot = mg_image::mtr::slot_name(*i);
                    let m = u.material.as_deref().unwrap_or_default();
                    format!("{slot} of {on}, not drawn: material {m}'s {slot} is")
                }
                (Role::Slot(i), Source::Mesh) => {
                    let slot = mg_image::mtr::slot_name(*i);
                    match &u.material {
                        Some(m) => format!("{slot} of {on}, through material {m}"),
                        None => format!("{slot} of {on}"),
                    }
                }
                (Role::Slot(i), Source::Material(m)) => {
                    format!("{} of material {m}, on {on}", mg_image::mtr::slot_name(*i))
                }
                (_, Source::Txi { of, default: true }) => format!(
                    "environment map of {on}: {of}.txi says default, the tileset's EnvMap \
                     (chrome1 without one)"
                ),
                (_, Source::Txi { of, .. }) => format!("environment map of {on}, from {of}.txi"),
                (Role::Particles, _) | (_, Source::Emitter) => format!("particles of emitter {on}"),
                (Role::Flare, _) | (_, Source::Flare) => format!("lens flare of light {on}"),
                (role, source) => format!("{role:?} of {on} ({source:?})"),
            };
            (line, all)
        })
        .collect();
    if row.area_env {
        out.push((
            "the tileset's environment map (EnvMap): what its meshes reflect where their \
             textures name none"
                .into(),
            String::new(),
        ));
    }
    out
}

/// What a texture's file is, in a line: its type, size, format and bytes.
fn file_line(f: &TextureFile) -> String {
    let kib = f.bytes.div_ceil(1024);
    let cube = if f.cube { ", a cube map of six faces" } else { "" };
    let kind = f.restype.to_string().to_uppercase();
    match &f.problem {
        Some(p) if f.width == 0 => format!("{kind}, {kib} KB, can't be read: {p}"),
        _ => format!("{kind} {}×{} {}{cube}, {kib} KB", f.width, f.height, f.format),
    }
}

/// Where and how [`section`] shows a model's textures.
pub(crate) struct Place<'a> {
    /// The tileset's `EnvMap`, what a TXI's `default` draws.
    pub area_env: Option<&'a str>,
    /// The rows kept to a scrolled height (a window with more below them).
    pub max_height: Option<f32>,
    /// Open until the builder folds it (else folded until opened).
    pub open: bool,
    /// A line said first (which of several tiles it is).
    pub note: Option<String>,
}

/// The section of `model`'s textures, headed "Textures of `model`" with
/// how many and how many are missing, folded as the builder last left it.
pub(crate) fn section(
    app: &mut Moonglow,
    ui: &mut Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    model: &str,
    place: Place<'_>,
) {
    let l = listing(app, ui, model, place.area_env);
    let missing = l.rows.iter().filter(|r| r.missing()).count();
    let title = match (l.rows.len(), missing) {
        (0, _) => format!("Textures of {model}"),
        (n, 0) => format!("Textures of {model} ({n})"),
        (n, m) => format!("Textures of {model} ({n}, {m} missing)"),
    };
    let id = egui::Id::new(("model-textures-section", salt));
    let header = egui::CollapsingHeader::new(title).id_salt(id).default_open(place.open);
    // (Within the width in sight: a window's content may be wider than
    // the window, in its scroll area, and the rows wrap to what is seen.)
    let seen = ui.clip_rect().right() - ui.cursor().left();
    let width = ui.available_width().min(seen).max(160.0);
    header.show(ui, |ui| {
        ui.set_max_width(width - ui.spacing().indent);
        if let Some(note) = &place.note {
            ui.weak(note);
        }
        if let Some(p) = &l.problem {
            ui.weak(p);
            return;
        }
        if l.rows.is_empty() {
            ui.weak(format!("{model} draws no textures."));
            return;
        }
        let mut rows = |ui: &mut Ui| {
            for (i, row) in l.rows.iter().enumerate() {
                if i > 0 {
                    ui.add_space(4.0);
                }
                row_ui(app, ui, row);
            }
        };
        match place.max_height {
            Some(h) => {
                egui::ScrollArea::vertical()
                    .id_salt((id, "rows"))
                    .max_height(h)
                    .auto_shrink([false, true])
                    .show(ui, rows);
            }
            None => rows(ui),
        }
    });
}

/// A texture's row: its picture beside its name and what it is.
fn row_ui(app: &mut Moonglow, ui: &mut Ui, row: &Row) {
    ui.horizontal_top(|ui| {
        picture(app, ui, row);
        ui.vertical(|ui| {
            ui.horizontal_wrapped(|ui| {
                let missing = row.missing();
                let text = egui::RichText::new(&row.name).strong();
                let text = if missing { text.color(ui.visuals().error_fg_color) } else { text };
                let name = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                name.context_menu(|ui| menu(app, ui, row));
                if ui.small_button("Copy").on_hover_text("Copy the texture's name").clicked() {
                    ui.ctx().copy_text(row.name.clone());
                }
                // (Cut by characters, as the game cuts it.)
                let cut = ResRef::cut(&row.name).map(|r| r.to_string().to_ascii_lowercase());
                if let Some(cut) = cut.filter(|c| *c != row.name) {
                    ui.weak(format!("(read as {cut})")).on_hover_text(
                        "A name longer than 16 characters: the game cuts it to 16 and draws \
                         the texture of that name",
                    );
                }
            });
            match &row.file {
                Some(f) => {
                    ui.horizontal_wrapped(|ui| {
                        ui.weak(file_line(f));
                        ui.weak("·");
                        ui.label(crate::images::origin_name(&f.layer))
                            .on_hover_text(format!("Read from the layer {}", f.layer));
                        if let Some(txi) = &f.txi {
                            ui.weak("·");
                            ui.weak("TXI").on_hover_text(format!(
                                "{}.txi, from {}",
                                row.name,
                                crate::images::origin_name(txi)
                            ));
                        }
                    });
                }
                None if row.drawn() => {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        "Missing: no DDS, TGA or PLT of the name in the load order",
                    );
                }
                None => {}
            }
            // (On hover: every node, and where the materials come from.)
            let materials: Vec<String> = row
                .materials
                .iter()
                .map(|(m, layer)| match layer {
                    Some(l) => format!("{m}.mtr from {}", crate::images::origin_name(l)),
                    None => format!("{m}.mtr found nowhere"),
                })
                .collect();
            for (line, all) in use_lines(row) {
                let r = ui.weak(line);
                let hover: Vec<&str> = std::iter::once(all.as_str())
                    .chain(materials.iter().map(String::as_str))
                    .filter(|s| !s.is_empty())
                    .collect();
                if !hover.is_empty() {
                    r.on_hover_text(hover.join("\n"));
                }
            }
        });
    });
}

/// A texture's picture (a PLT's in its first colors, a cube map's first
/// face); a mark where it is missing.
fn picture(app: &mut Moonglow, ui: &mut Ui, row: &Row) {
    let size = egui::vec2(PICTURE, PICTURE);
    let name = ResRef::cut(&row.name).map_or_else(|| row.name.clone(), |r| r.to_string());
    let picture = row.file.as_ref().and_then(|f| {
        let mut l = app.loader()?;
        match (f.restype, f.cube) {
            (ResType::PLT, _) => l.plt_picture(ui.ctx(), &name, [0; 10]),
            (_, true) => l.thumbnail(ui.ctx(), &format!("{name}0"), THUMB),
            _ => l.thumbnail(ui.ctx(), &name, THUMB),
        }
    });
    match picture {
        Some(p) => {
            ui.add(egui::Image::new(&p.texture).fit_to_exact_size(size).alt_text(&row.name));
        }
        None => {
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
            ui.painter().rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Inside);
            if row.missing() {
                let color = ui.visuals().error_fg_color;
                let font = egui::FontId::proportional(18.0);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "?", font, color);
            }
        }
    }
}

/// A texture's menu: its name to the clipboard, its materials and TXI to
/// read.
fn menu(app: &mut Moonglow, ui: &mut Ui, row: &Row) {
    if ui.button("Copy Name").clicked() {
        ui.ctx().copy_text(row.name.clone());
        ui.close();
    }
    for (m, layer) in &row.materials {
        let Some(key) = ResRef::cut(m).map(|r| ResKey::new(r, ResType::MTR)) else { continue };
        if layer.is_some() && ui.button(format!("View {m}.mtr")).clicked() {
            app.actions.push(Action::OpenTab(Tab::Resource(key)));
            ui.close();
        }
    }
    if row.file.as_ref().is_some_and(|f| f.txi.is_some())
        && let Some(r) = ResRef::cut(&row.name)
        && ui.button(format!("View {}.txi", row.name)).clicked()
    {
        app.actions.push(Action::OpenTab(Tab::Resource(ResKey::new(r, ResType::TXI))));
        ui.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
