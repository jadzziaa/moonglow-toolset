//! The tileset editor (Tools › Tilesets): a `.set` file on disk, edited in
//! place line by line (`mg_set::edit`), with undo: its general settings,
//! terrains and crossers, tiles and groups. It also makes the tileset's
//! palette beside it and checks it with the content doctor.

use std::path::PathBuf;

use egui::Ui;
use mg_core::{Codepage, ResType};
use mg_resman::ResKey;
use mg_set::edit::{self, SetFile};
use mg_set::{CORNERS, EDGES, Tileset};

use crate::dialogs::FileKind;
use crate::{Action, Moonglow, Tab};

/// The editor's pages.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    General,
    Terrain,
    Tiles,
    Groups,
}

/// A tileset open in a tab.
#[derive(Debug)]
pub struct TilesetDoc {
    pub id: u32,
    pub path: PathBuf,
    pub file: SetFile,
    /// The file as read, after each change.
    set: Result<Tileset, String>,
    undo: Vec<SetFile>,
    redo: Vec<SetFile>,
    saved: Option<usize>,
    pub page: Page,
    pub tile: Option<usize>,
    pub group: Option<usize>,
    pub filter: String,
    /// A terrain's or crosser's name, while it's being added.
    pub new_type: String,
    pub findings: Option<Vec<mg_module::doctor::Finding>>,
}

impl TilesetDoc {
    fn new(id: u32, path: PathBuf, file: SetFile) -> TilesetDoc {
        let set = read(&file);
        TilesetDoc {
            id,
            path,
            file,
            set,
            undo: Vec::new(),
            redo: Vec::new(),
            saved: Some(0),
            page: Page::default(),
            tile: None,
            group: None,
            filter: String::new(),
            new_type: String::new(),
            findings: None,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.saved != Some(self.undo.len())
    }

    pub fn title(&self) -> String {
        let name =
            self.path.file_name().map_or("tileset".into(), |n| n.to_string_lossy().into_owned());
        format!("{name}{}", if self.is_dirty() { " *" } else { "" })
    }

    pub fn tileset(&self) -> Option<&Tileset> {
        self.set.as_ref().ok()
    }

    /// Changes the file, as one undo step.
    pub fn change(&mut self, f: impl FnOnce(&mut SetFile)) {
        let before = self.file.clone();
        f(&mut self.file);
        if self.file == before {
            return;
        }
        if self.saved.is_some_and(|s| s > self.undo.len()) {
            self.saved = None;
        }
        self.undo.push(before);
        self.redo.clear();
        self.set = read(&self.file);
        self.findings = None;
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.file, prev));
            self.set = read(&self.file);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.file, next));
            self.set = read(&self.file);
        }
    }

    pub fn save(&mut self) -> Result<(), String> {
        let part = self.path.with_extension("set.part");
        std::fs::write(&part, self.file.text())
            .and_then(|()| std::fs::rename(&part, &self.path))
            .map_err(|e| format!("{}: {e}", self.path.display()))?;
        self.saved = Some(self.undo.len());
        Ok(())
    }
}

fn read(file: &SetFile) -> Result<Tileset, String> {
    Tileset::parse(file.text().as_bytes(), Codepage::default()).map_err(|e| e.to_string())
}

/// Tools › Tilesets › New Tileset…: a new `.set` with its general
/// settings, to add terrains, tiles and groups to.
pub fn new_tileset(app: &mut Moonglow) {
    let suggested = PathBuf::from("zzz01.set");
    let Some(path) = app.dialogs.save_file(FileKind::Tileset, Some(&suggested)) else { return };
    let name = path.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    if name.is_empty() || name.len() > 16 {
        app.log.error("A tileset's name has 1 to 16 characters");
        return;
    }
    let file = edit::skeleton(&name);
    if let Err(e) = std::fs::write(&path, file.text()) {
        app.log.error(format!("{}: {e}", path.display()));
        return;
    }
    open_path(app, path);
}

/// Tools › Tilesets › Open Tileset….
pub fn open(app: &mut Moonglow) {
    let Some(path) = app.dialogs.open_file(FileKind::Tileset, None) else { return };
    open_path(app, path);
}

pub fn open_path(app: &mut Moonglow, path: PathBuf) {
    if let Some(d) = app.tilesets.iter().find(|d| d.path == path) {
        app.actions.push(Action::OpenTab(Tab::Tileset(d.id)));
        return;
    }
    match std::fs::read(&path) {
        Ok(data) => {
            let text = Codepage::default().decode(&data).into_owned();
            app.next_tileset += 1;
            let id = app.next_tileset;
            app.tilesets.push(TilesetDoc::new(id, path, SetFile::parse(&text)));
            app.actions.push(Action::OpenTab(Tab::Tileset(id)));
        }
        Err(e) => app.log.error(format!("{}: {e}", path.display())),
    }
}

/// Saves every tileset with unsaved changes.
pub(crate) fn save_all(app: &mut Moonglow) {
    for d in app.tilesets.iter_mut().filter(|d| d.is_dirty()) {
        match d.save() {
            Ok(()) => app.log.info(format!("Saved {}", d.path.display())),
            Err(e) => app.log.error(e),
        }
    }
}

pub(crate) fn unsaved(app: &Moonglow) -> bool {
    app.tilesets.iter().any(TilesetDoc::is_dirty)
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, id: u32) {
    let Some(i) = app.tilesets.iter().position(|d| d.id == id) else {
        ui.label("This tileset was closed.");
        return;
    };
    // The doc is taken out while drawn (its fields edit through the app's
    // buffers), and put back.
    let mut doc = app.tilesets.remove(i);
    draw(app, ui, &mut doc);
    app.tilesets.insert(i, doc);
}

fn draw(app: &mut Moonglow, ui: &mut Ui, d: &mut TilesetDoc) {
    ui.horizontal(|ui| {
        ui.strong(d.path.display().to_string());
        if let Ok(t) = &d.set {
            ui.weak(format!(
                "{} tiles, {} groups, {} terrains, {} crossers",
                t.tiles.len(),
                t.groups.len(),
                t.terrains.len(),
                t.crossers.len()
            ));
        }
    });
    ui.horizontal_wrapped(|ui| {
        for (page, name) in [
            (Page::General, "General"),
            (Page::Terrain, "Terrains and Crossers"),
            (Page::Tiles, "Tiles"),
            (Page::Groups, "Groups"),
        ] {
            ui.selectable_value(&mut d.page, page, name);
        }
        ui.separator();
        if ui.add_enabled(!d.undo.is_empty(), egui::Button::new("Undo")).clicked() {
            d.undo();
        }
        if ui.add_enabled(!d.redo.is_empty(), egui::Button::new("Redo")).clicked() {
            d.redo();
        }
        if ui.add_enabled(d.is_dirty(), egui::Button::new("Save")).clicked() {
            match d.save() {
                Ok(()) => app.log.info(format!("Saved {}", d.path.display())),
                Err(e) => app.log.error(e),
            }
        }
        if ui
            .button("Make Palette")
            .on_hover_text(
                "Write <tileset>palstd.itp beside the .set: its groups, features and terrains",
            )
            .clicked()
        {
            make_palette(app, d);
        }
        if ui.button("Check").on_hover_text("The content doctor's checks of tilesets").clicked() {
            check(app, d);
        }
        if ui
            .add_enabled(app.viewport.is_some(), egui::Button::new("Render Minimap Pictures"))
            .on_hover_text(
                "Each tile seen from above, saved as its minimap picture (ImageMap2D) beside the \
                 .set: 32 pixels, from the models beside the .set or in the game data",
            )
            .on_disabled_hover_text("Rendering needs a GPU")
            .clicked()
        {
            render_minimaps(app, d, 32);
        }
    });
    if let Err(e) = &d.set {
        ui.colored_label(
            ui.visuals().error_fg_color,
            format!("The file can't be read as a tileset: {e}"),
        );
        return;
    }
    if let Some(findings) = &d.findings {
        if findings.is_empty() {
            ui.weak("No problems found.");
        }
        for f in findings {
            let color = match f.severity {
                mg_module::doctor::Severity::Error => ui.visuals().error_fg_color,
                mg_module::doctor::Severity::Warning => ui.visuals().warn_fg_color,
            };
            ui.colored_label(color, format!("{}: {}", f.at, f.message));
        }
    }
    ui.separator();
    match d.page {
        Page::General => general(app, ui, d),
        Page::Terrain => terrain(ui, d),
        Page::Tiles => tiles(app, ui, d),
        Page::Groups => groups(ui, d),
    }
}

/// A text value of a section, committed when focus leaves the field.
fn text_key(
    app: &mut Moonglow,
    ui: &mut Ui,
    d: &mut TilesetDoc,
    section: &str,
    key: &str,
    width: f32,
) {
    let current = d.file.get(section, key).unwrap_or_default().to_string();
    let id = egui::Id::new(("set", d.id, section, key));
    if let Some(v) = crate::widgets::commit_text(app, ui, id, &current, false, width) {
        d.change(|f| f.set(section, key, v.trim()));
    }
}

fn flag_key(ui: &mut Ui, d: &mut TilesetDoc, section: &str, key: &str, label: &str) {
    let mut on =
        d.file.get(section, key).and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(0) != 0;
    if ui.checkbox(&mut on, label).changed() {
        d.change(|f| f.set(section, key, if on { "1" } else { "0" }));
    }
}

fn int_key(
    ui: &mut Ui,
    d: &mut TilesetDoc,
    section: &str,
    key: &str,
    range: std::ops::RangeInclusive<i32>,
) {
    let current = d.file.get(section, key).and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(0);
    if let Some(v) = crate::widgets::commit_number(ui, current, range) {
        d.change(|f| f.set(section, key, &v.to_string()));
    }
}

/// A terrain or crosser name chosen from the tileset's (empty: none).
fn name_key(
    ui: &mut Ui,
    d: &mut TilesetDoc,
    section: &str,
    key: &str,
    names: &[String],
    none: bool,
) {
    let current = d.file.get(section, key).unwrap_or_default().to_string();
    let mut chosen = None;
    egui::ComboBox::from_id_salt(("set-name", d.id, section, key))
        .selected_text(if current.is_empty() { "(none)".to_string() } else { current.clone() })
        .show_ui(ui, |ui| {
            if none && ui.selectable_label(current.is_empty(), "(none)").clicked() {
                chosen = Some(String::new());
            }
            for n in names {
                if ui.selectable_label(n.eq_ignore_ascii_case(&current), n).clicked() {
                    chosen = Some(n.clone());
                }
            }
        });
    if let Some(v) = chosen.filter(|v| *v != current) {
        d.change(|f| f.set(section, key, &v));
    }
}

fn general(app: &mut Moonglow, ui: &mut Ui, d: &mut TilesetDoc) {
    let terrains: Vec<String> = d
        .tileset()
        .map(|t| t.terrains.iter().map(|x| x.name.clone()).collect())
        .unwrap_or_default();
    egui::Grid::new(("set-general", d.id)).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        for (key, label) in [
            ("Name", "Name"),
            ("UnlocalizedName", "Name shown (no talk-table string)"),
            ("EnvMap", "Environment map"),
        ] {
            ui.label(label);
            text_key(app, ui, d, "GENERAL", key, 220.0);
            ui.end_row();
        }
        ui.label("Name's talk-table string");
        int_key(ui, d, "GENERAL", "DisplayName", -1..=i32::MAX);
        ui.end_row();
        crate::widgets::field_label(ui, "Height step (m)");
        text_key(app, ui, d, "GENERAL", "Transition", 80.0);
        ui.end_row();
        crate::widgets::field_label(ui, "Selector height (m)");
        text_key(app, ui, d, "GENERAL", "SelectorHeight", 80.0);
        ui.end_row();
        for (key, label) in [
            ("Border", "Border terrain"),
            ("Default", "New areas' terrain"),
            ("Floor", "Floor terrain"),
        ] {
            ui.label(label);
            name_key(ui, d, "GENERAL", key, &terrains, false);
            ui.end_row();
        }
    });
    flag_key(ui, d, "GENERAL", "Interior", "Interior");
    flag_key(ui, d, "GENERAL", "HasHeightTransition", "Height transitions (raise and lower)");
    ui.separator();
    ui.strong("Grass");
    flag_key(ui, d, "GRASS", "Grass", "Grass on tiles' grass faces");
    egui::Grid::new(("set-grass", d.id)).num_columns(2).show(ui, |ui| {
        for (key, label) in
            [("GrassTextureName", "Texture"), ("Density", "Density"), ("Height", "Height")]
        {
            ui.label(label);
            text_key(app, ui, d, "GRASS", key, 160.0);
            ui.end_row();
        }
    });
}

fn terrain(ui: &mut Ui, d: &mut TilesetDoc) {
    let Some(t) = d.tileset().cloned() else { return };
    ui.columns(2, |cols| {
        for (col, crosser) in [(0, false), (1, true)] {
            let ui = &mut cols[col];
            ui.strong(if crosser { "Crossers" } else { "Terrains" });
            let list = if crosser { &t.crossers } else { &t.terrains };
            for (i, x) in list.iter().enumerate() {
                let label = if x.strref.is_none() {
                    x.unlocalized_name.clone().unwrap_or_default()
                } else {
                    format!("string {}", x.strref.0)
                };
                ui.horizontal(|ui| {
                    ui.monospace(format!("{i:>3}"));
                    ui.label(&x.name);
                    ui.weak(label);
                });
            }
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Name");
        ui.add(egui::TextEdit::singleline(&mut d.new_type).desired_width(160.0));
        let name = d.new_type.trim().to_string();
        let taken =
            t.terrains.iter().chain(&t.crossers).any(|x| x.name.eq_ignore_ascii_case(&name));
        let ok = !name.is_empty() && !taken && !name.contains(char::is_whitespace);
        if ui.add_enabled(ok, egui::Button::new("Add Terrain")).clicked() {
            d.change(|f| {
                edit::add_type(f, false, &name, None);
            });
            d.new_type.clear();
        }
        if ui.add_enabled(ok, egui::Button::new("Add Crosser")).clicked() {
            d.change(|f| {
                edit::add_type(f, true, &name, None);
            });
            d.new_type.clear();
        }
        if taken {
            ui.weak("The tileset has one of that name.");
        }
    });
}

fn tiles(app: &mut Moonglow, ui: &mut Ui, d: &mut TilesetDoc) {
    let Some(t) = d.tileset().cloned() else { return };
    ui.horizontal(|ui| {
        ui.label("Find");
        ui.add(egui::TextEdit::singleline(&mut d.filter).desired_width(160.0).hint_text("model or terrain"));
        let sel = d.tile;
        if ui
            .add_enabled(sel.is_some(), egui::Button::new("Duplicate Tile"))
            .on_hover_text("A copy at the end (tiles are numbered by place, which areas store)")
            .clicked()
            && let Some(s) = sel
        {
            let mut new = None;
            d.change(|f| new = edit::duplicate_tile(f, s));
            d.tile = new.or(d.tile);
        }
        if ui
            .button("Add Tile")
            .on_hover_text("A tile at the end, its model named after the tileset, every corner the first terrain")
            .clicked()
        {
            let model = format!("{}_a01_{:02}", t.general.name.to_lowercase(), t.tiles.len() + 1);
            let terrain = t.terrains.first().map(|x| x.name.clone()).unwrap_or_default();
            let mut new = 0;
            d.change(|f| new = edit::add_tile(f, &model, &terrain));
            d.tile = Some(new);
        }
        if ui.add_enabled(!t.tiles.is_empty(), egui::Button::new("Remove Last Tile")).clicked() {
            d.change(|f| {
                edit::remove_last_tile(f);
            });
        }
    });
    let filter = d.filter.trim().to_lowercase();
    let rows: Vec<usize> = (0..t.tiles.len())
        .filter(|&i| {
            let tile = &t.tiles[i];
            filter.is_empty()
                || tile.model.to_lowercase().contains(&filter)
                || tile.corners.iter().any(|(c, _)| c.to_lowercase().contains(&filter))
                || tile.edges.iter().any(|e| e.to_lowercase().contains(&filter))
        })
        .collect();
    let row_height = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
    egui::ScrollArea::vertical().id_salt(("set-tiles", d.id)).max_height(260.0).show_rows(
        ui,
        row_height,
        rows.len(),
        |ui, range| {
            for &i in &rows[range] {
                let tile = &t.tiles[i];
                let corners: Vec<&str> = tile.corners.iter().map(|(c, _)| c.as_str()).collect();
                let edges: Vec<&str> =
                    tile.edges.iter().map(|e| if e.is_empty() { "-" } else { e }).collect();
                let text = format!(
                    "{i:>4}  {:<16}  {}  [{}]",
                    tile.model,
                    corners.join("/"),
                    edges.join("/")
                );
                if ui
                    .selectable_label(d.tile == Some(i), egui::RichText::new(text).monospace())
                    .clicked()
                {
                    d.tile = Some(i);
                }
            }
        },
    );
    ui.separator();
    let Some(i) = d.tile.filter(|&i| i < t.tiles.len()) else {
        ui.weak("Choose a tile to edit it.");
        return;
    };
    let section = format!("TILE{i}");
    let terrains: Vec<String> = t.terrains.iter().map(|x| x.name.clone()).collect();
    let crossers: Vec<String> = t.crossers.iter().map(|x| x.name.clone()).collect();
    ui.horizontal(|ui| {
        ui.strong(format!("Tile {i}"));
        let model = t.tiles[i].model.clone();
        let key = ResKey::parse(&model, ResType::MDL);
        let found = key.is_some_and(|k| app.game.as_ref().is_some_and(|g| g.resman.contains(&k)));
        if ui
            .add_enabled(found, egui::Button::new("Preview"))
            .on_disabled_hover_text(
                "The model isn't in the game data (a hak the module uses, development, override)",
            )
            .clicked()
            && let Some(k) = key
        {
            app.actions.push(Action::OpenTab(Tab::Model(k)));
        }
    });
    egui::Grid::new(("set-tile", d.id)).num_columns(4).spacing([10.0, 4.0]).show(ui, |ui| {
        for (key, label) in [
            ("Model", "Model"),
            ("WalkMesh", "Walkmesh"),
            ("ImageMap2D", "Minimap picture"),
            ("PathNode", "Path node"),
        ] {
            ui.label(label);
            text_key(app, ui, d, &section, key, 160.0);
            ui.end_row();
        }
        ui.label("Orientation");
        // Degrees, a quarter turn at a time.
        let current = d
            .file
            .get(&section, "Orientation")
            .and_then(|v| v.trim().parse::<i32>().ok())
            .unwrap_or(0);
        let mut chosen = current;
        egui::ComboBox::from_id_salt(("set-orientation", d.id, i))
            .selected_text(format!("{current}°"))
            .show_ui(ui, |ui| {
                for a in [0, 90, 180, 270] {
                    ui.selectable_value(&mut chosen, a, format!("{a}°"));
                }
            });
        if chosen != current {
            d.change(|f| f.set(&section, "Orientation", &chosen.to_string()));
        }
        ui.end_row();
        for (n, corner) in CORNERS.iter().enumerate() {
            ui.label(*corner);
            name_key(ui, d, &section, corner, &terrains, false);
            ui.label("height");
            int_key(ui, d, &section, &format!("{corner}Height"), -1..=4);
            ui.end_row();
            let _ = n;
        }
        for edge in EDGES {
            ui.label(format!("{edge} edge"));
            name_key(ui, d, &section, edge, &crossers, true);
            ui.end_row();
        }
    });
    ui.horizontal_wrapped(|ui| {
        for key in [
            "MainLight1",
            "MainLight2",
            "SourceLight1",
            "SourceLight2",
            "AnimLoop1",
            "AnimLoop2",
            "AnimLoop3",
        ] {
            flag_key(ui, d, &section, key, key);
        }
    });
}

fn groups(ui: &mut Ui, d: &mut TilesetDoc) {
    let Some(t) = d.tileset().cloned() else { return };
    ui.horizontal(|ui| {
        let tile = d.tile;
        if ui
            .button("Add Group")
            .on_hover_text(
                "One tile (the tile chosen on the Tiles page, else tile 0); make it bigger below",
            )
            .clicked()
        {
            let mut n = 0;
            d.change(|f| {
                n = edit::add_group(f, "New group", 1, 1, &[Some(tile.unwrap_or(0) as u32)])
            });
            d.group = Some(n);
        }
        if ui.add_enabled(!t.groups.is_empty(), egui::Button::new("Remove Last Group")).clicked() {
            d.change(|f| {
                edit::remove_last_group(f);
            });
        }
    });
    ui.columns(2, |cols| {
        egui::ScrollArea::vertical().id_salt(("set-groups", d.id)).max_height(360.0).show(
            &mut cols[0],
            |ui| {
                for (i, g) in t.groups.iter().enumerate() {
                    let text = format!("{i:>3}  {} ({}×{})", g.name, g.columns, g.rows);
                    if ui.selectable_label(d.group == Some(i), text).clicked() {
                        d.group = Some(i);
                    }
                }
            },
        );
        let ui = &mut cols[1];
        let Some(i) = d.group.filter(|&i| i < t.groups.len()) else {
            ui.weak("Choose a group.");
            return;
        };
        let g = &t.groups[i];
        let section = format!("GROUP{i}");
        let mut name = g.name.clone();
        ui.horizontal(|ui| {
            ui.label("Name");
            if ui.text_edit_singleline(&mut name).lost_focus() && name != g.name {
                d.change(|f| f.set(&section, "Name", name.trim()));
            }
        });
        let (mut rows, mut columns) = (g.rows, g.columns);
        ui.horizontal(|ui| {
            ui.label("Rows");
            let r = ui.add(egui::DragValue::new(&mut rows).range(1..=16));
            ui.label("Columns");
            let c = ui.add(egui::DragValue::new(&mut columns).range(1..=16));
            if (r.changed() || c.changed()) && (rows, columns) != (g.rows, g.columns) {
                d.change(|f| {
                    f.set(&section, "Rows", &rows.to_string());
                    f.set(&section, "Columns", &columns.to_string());
                    for k in 0..(rows * columns) as usize {
                        if f.get(&section, &format!("Tile{k}")).is_none() {
                            f.set(&section, &format!("Tile{k}"), "-1");
                        }
                    }
                });
            }
        });
        ui.weak("Tile numbers, the bottom row first (-1: none)");
        // Rows from the top, as on the map.
        for r in (0..g.rows as usize).rev() {
            ui.horizontal(|ui| {
                for c in 0..g.columns as usize {
                    let k = r * g.columns as usize + c;
                    let current = g.tiles.get(k).copied().flatten().map_or(-1, |t| t as i64);
                    let mut v = current;
                    let resp =
                        ui.add(egui::DragValue::new(&mut v).range(-1..=t.tiles.len() as i64 - 1));
                    if resp.changed() && !resp.dragged() && v != current
                        || resp.drag_stopped() && v != current
                    {
                        d.change(|f| f.set(&section, &format!("Tile{k}"), &v.to_string()));
                    }
                }
            });
        }
    });
}

/// Writes the palette beside the `.set`.
fn make_palette(app: &mut Moonglow, d: &mut TilesetDoc) {
    let Some(t) = d.tileset() else { return };
    let (gff, warnings) = mg_module::tileset::palette(t);
    let stem = d.path.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    let path = d.path.with_file_name(format!("{stem}palstd.itp"));
    for w in warnings {
        app.log.warn(format!("Palette: {w}"));
    }
    match gff
        .to_bytes()
        .map_err(|e| e.to_string())
        .and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string()))
    {
        Ok(()) => app.log.info(format!("Wrote {}", path.display())),
        Err(e) => app.log.error(format!("{}: {e}", path.display())),
    }
}

/// Renders every tile's minimap picture into the `.set`'s folder, its
/// models found there or in the game data. Tiles without a picture's name
/// get `mi_<model>` (one undoable change).
pub fn render_minimaps(app: &mut Moonglow, d: &mut TilesetDoc, size: u32) {
    let Some(t) = d.tileset().cloned() else { return };
    let dir = d.path.parent().map(PathBuf::from).unwrap_or_default();
    // Unnamed pictures named after their models.
    let unnamed: Vec<usize> = (0..t.tiles.len())
        .filter(|&i| {
            t.tiles[i]
                .image_map_2d
                .as_deref()
                .is_none_or(|n| n.is_empty() || n.eq_ignore_ascii_case("(null)"))
        })
        .collect();
    if !unnamed.is_empty() {
        d.change(|f| {
            for &i in &unnamed {
                f.set(
                    &format!("TILE{i}"),
                    "ImageMap2D",
                    &format!("mi_{}", t.tiles[i].model.to_lowercase()),
                );
            }
        });
    }
    let Some(t) = d.tileset().cloned() else { return };
    // The folder's models and textures, over the game data while rendering.
    const LAYER: &str = "tileset folder";
    if let Some(game) = &mut app.game {
        game.resman.add(
            mg_resman::priority::TEMP,
            LAYER,
            mg_resman::LayerClass::Directory,
            mg_resman::DirContainer::open(&dir),
        );
    }
    let (mut written, mut failed) = (0, Vec::new());
    let mut done = std::collections::HashSet::new();
    for tile in &t.tiles {
        let Some(name) = tile.image_map_2d.clone().filter(|n| !n.is_empty()) else { continue };
        if !done.insert(name.to_lowercase()) {
            continue;
        }
        match crate::model_view::render_tile_from_above(app, &tile.model, size) {
            Some(image) => {
                let path = dir.join(format!("{}.tga", name.to_lowercase()));
                match std::fs::write(&path, mg_image::write_tga(&image)) {
                    Ok(()) => written += 1,
                    Err(e) => failed.push(format!("{}: {e}", path.display())),
                }
            }
            None => failed.push(format!("{}: model not found", tile.model)),
        }
    }
    if let Some(game) = &mut app.game {
        game.resman.remove(LAYER);
    }
    app.log.info(format!("Rendered {written} minimap pictures into {}", dir.display()));
    for f in failed.iter().take(20) {
        app.log.warn(format!("Minimap picture: {f}"));
    }
    if failed.len() > 20 {
        app.log.warn(format!("… and {} more", failed.len() - 20));
    }
}

/// The content doctor's tileset checks, models looked for in the game data.
fn check(app: &mut Moonglow, d: &mut TilesetDoc) {
    let Some(game) = &app.game else {
        app.log.error("Checking a tileset needs the game data");
        return;
    };
    let stem = d.path.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    let Some(key) = ResKey::parse(&stem, ResType::SET) else { return };
    d.findings =
        Some(mg_module::doctor::examine_tileset(&game.resman, key, d.file.text().as_bytes()));
}

/// Asks what to do with a tileset's unsaved changes when its tab is closed.
pub(crate) fn closing_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(id) = app.tileset_closing else { return };
    let Some(i) = app.tilesets.iter().position(|d| d.id == id) else {
        app.tileset_closing = None;
        return;
    };
    let title = app.tilesets[i].title();
    let mut answer = None;
    egui::Window::new("Unsaved Tileset")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("{} has unsaved changes.", title.trim_end_matches(" *")));
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    answer = Some(true);
                }
                if ui.button("Discard").clicked() {
                    answer = Some(false);
                }
                if ui.button("Cancel").clicked() {
                    app.tileset_closing = None;
                }
            });
        });
    let Some(keep) = answer else { return };
    app.tileset_closing = None;
    if keep && let Err(e) = app.tilesets[i].save() {
        app.log.error(e);
        return;
    }
    app.tilesets.remove(i);
    if let Some(t) = app.dock.find_tab(&Tab::Tileset(id)) {
        app.dock.remove_tab(t);
    }
}
