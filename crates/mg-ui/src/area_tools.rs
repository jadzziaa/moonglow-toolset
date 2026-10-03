//! The area viewer's dialogs: Adjust Location (`TdlgLocation`: an exact
//! position, bearing and visual transform for the selected objects) and Find Instance
//! (`TdlgFindInstance`: placed objects across the module by type, area,
//! blueprint and tag; a double click goes to one).

use egui::Ui;
use glam::Vec3;
use mg_area::ObjectKind;
use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_resman::ResKey;

use crate::{Action, Moonglow, Tab};

/// The Adjust Location window's values.
#[derive(Debug, Clone, PartialEq)]
pub struct AdjustLocation {
    pub area: ResRef,
    pub objects: Vec<(ObjectKind, usize)>,
    pub position: Vec3,
    /// Aurora's Bearing: degrees counter-clockwise, 0 facing north (the
    /// model's turn).
    pub bearing: f32,
    /// Which of X, Y, Z and the bearing were changed (only those are set
    /// on every object).
    pub changed: [bool; 4],
    /// The visual transform (EE), and whether it was changed.
    pub visual: mg_area::VisualTransform,
    pub visual_changed: bool,
}

/// Adjust Location for the selection of an area's view (showing the first
/// object's location).
pub(crate) fn adjust(view: &crate::area_view::AreaView) -> Option<AdjustLocation> {
    let model = view.model.as_ref()?;
    let first = view.selection.first().and_then(|&(k, i)| model.object(k, i))?;
    Some(AdjustLocation {
        area: view.area,
        objects: view.selection.clone(),
        position: first.position,
        bearing: first.rotation.to_degrees().rem_euclid(360.0),
        changed: [false; 4],
        visual: first.visual.unwrap_or_default(),
        visual_changed: false,
    })
}

/// Draws the open dialogs.
pub(crate) fn windows(app: &mut Moonglow, ui: &mut Ui) {
    adjust_window(app, ui);
    find_window(app, ui);
}

fn adjust_window(app: &mut Moonglow, ui: &mut Ui) {
    let Some(mut a) = app.adjust.take() else { return };
    let mut open = true;
    let mut cancel = false;
    let mut done = None;
    egui::Window::new("Adjust Location").open(&mut open).resizable(false).collapsible(false).show(
        ui.ctx(),
        |ui| {
            if a.objects.len() > 1 {
                ui.weak(format!("{} objects: what you change is set on each", a.objects.len()));
            }
            egui::Grid::new("adjust").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                for (i, label) in ["X", "Y", "Z"].into_iter().enumerate() {
                    crate::widgets::field_label(ui, label);
                    let r = ui
                        .add(egui::DragValue::new(&mut a.position[i]).speed(0.05).max_decimals(3));
                    a.changed[i] |= r.changed();
                    ui.end_row();
                }
                crate::widgets::field_label(ui, "Bearing (°)")
                    .on_hover_text("0 faces north, 90 west (as Aurora shows it)");
                let r = ui.add(
                    egui::DragValue::new(&mut a.bearing)
                        .speed(1.0)
                        .range(0.0..=360.0)
                        .max_decimals(2),
                );
                a.changed[3] |= r.changed();
                ui.end_row();
            });
            ui.separator();
            crate::widgets::section_heading(ui, "Visual Transforms");
            egui::Grid::new("adjust-visual").num_columns(4).spacing([12.0, 6.0]).show(ui, |ui| {
                crate::widgets::field_label(ui, "Scale");
                let mut scale = a.visual.scale.x;
                let r = ui.add(
                    egui::DragValue::new(&mut scale)
                        .speed(0.01)
                        .range(0.01..=100.0)
                        .max_decimals(2),
                );
                if r.changed() {
                    a.visual.scale = Vec3::splat(scale);
                    a.visual_changed = true;
                }
                ui.end_row();
                for (i, axis) in ["X", "Y", "Z"].into_iter().enumerate() {
                    ui.label(format!("{axis} Rotation"));
                    let r = ui.add(
                        egui::DragValue::new(&mut a.visual.rotate[i]).speed(1.0).max_decimals(1),
                    );
                    a.visual_changed |= r.changed();
                    ui.label(format!("{axis} Translation"));
                    let r = ui.add(
                        egui::DragValue::new(&mut a.visual.translate[i])
                            .speed(0.01)
                            .max_decimals(2),
                    );
                    a.visual_changed |= r.changed();
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() || crate::widgets::enter(ui) {
                    done = Some(true);
                }
                if ui.button("Apply").clicked() {
                    done = Some(false);
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        },
    );
    if cancel {
        return;
    }
    if let Some(close) = done {
        apply_location(app, &a);
        a.changed = [false; 4];
        a.visual_changed = false;
        if close {
            return;
        }
    }
    if open {
        app.adjust = Some(a);
    }
}

/// Sets the changed parts of the location on each object (one command).
fn apply_location(app: &mut Moonglow, a: &AdjustLocation) {
    let git = ResKey::new(a.area, ResType::GIT);
    let (Some(view), Some(ws)) = (app.area_views.get(&a.area), app.ws.as_mut()) else { return };
    let Some(model) = &view.model else { return };
    let Ok(doc) = ws.doc(&git) else { return };
    let mut edits = Vec::new();
    for &(kind, index) in &a.objects {
        let (Some(o), Some(s)) =
            (model.object(kind, index), doc.root.list(kind.list()).and_then(|l| l.get(index)))
        else {
            continue;
        };
        let mut p = o.position;
        for i in 0..3 {
            if a.changed[i] {
                p[i] = a.position[i];
            }
        }
        let rotation = if a.changed[3] { a.bearing.to_radians() } else { o.rotation };
        edits.extend(mg_area::edit::move_edits(git, o, s, p, rotation));
        let shaped = matches!(
            kind,
            ObjectKind::Creature | ObjectKind::Placeable | ObjectKind::Door | ObjectKind::Item
        );
        if a.visual_changed && shaped {
            edits.extend(mg_area::edit::visual_transform_edits(git, o, s, a.visual));
        }
    }
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new("Adjust location", edits)));
    }
}

/// The Find Instance window's criteria and results.
#[derive(Debug, Clone, PartialEq)]
pub struct FindInstance {
    /// Which kinds to look for, by [`ObjectKind::index`].
    pub kinds: [bool; 9],
    /// `None`: every area.
    pub area: Option<ResRef>,
    pub template: String,
    pub tag: String,
    pub results: Vec<Found>,
}

impl Default for FindInstance {
    fn default() -> FindInstance {
        FindInstance {
            kinds: [true; 9],
            area: None,
            template: String::new(),
            tag: String::new(),
            results: Vec::new(),
        }
    }
}

/// A placed object found.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub kind: ObjectKind,
    pub index: usize,
    pub area: ResRef,
    pub tag: String,
    pub template: String,
}

/// The objects placed in the module's areas that match `f`'s criteria
/// (tag and blueprint: case-insensitive parts).
pub(crate) fn search(app: &mut Moonglow, f: &FindInstance) -> Vec<Found> {
    let Some(ws) = app.ws.as_mut() else { return Vec::new() };
    let mut areas: Vec<ResRef> = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
    areas.sort();
    areas.retain(|a| f.area.is_none_or(|only| only == *a));
    let (tag, template) = (f.tag.to_lowercase(), f.template.to_lowercase());
    let mut out = Vec::new();
    for area in areas {
        let Ok(git) = ws.doc(&ResKey::new(area, ResType::GIT)) else { continue };
        for kind in ObjectKind::ALL {
            if !f.kinds[kind.index()] {
                continue;
            }
            let field = if kind == ObjectKind::Store { "ResRef" } else { "TemplateResRef" };
            for (index, s) in git.root.list(kind.list()).unwrap_or(&[]).iter().enumerate() {
                let t = s.string("Tag").map(|t| String::from_utf8_lossy(t).into_owned());
                let t = t.unwrap_or_default();
                let bp = s.resref(field).map(|r| r.to_string()).unwrap_or_default();
                if t.to_lowercase().contains(&tag) && bp.contains(&template) {
                    out.push(Found { kind, index, area, tag: t, template: bp });
                }
            }
        }
    }
    out
}

fn find_window(app: &mut Moonglow, ui: &mut Ui) {
    let Some(mut f) = app.find_instance.take() else { return };
    let mut open = true;
    let mut go = None;
    let areas: Vec<ResRef> = app.ws.as_ref().map_or_else(Vec::new, |ws| {
        let mut a: Vec<ResRef> = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
        a.sort();
        a
    });
    egui::Window::new("Find Instance").open(&mut open).default_width(460.0).show(ui.ctx(), |ui| {
        ui.horizontal_wrapped(|ui| {
            crate::widgets::field_label(ui, "Search For");
            for kind in ObjectKind::ALL {
                ui.checkbox(&mut f.kinds[kind.index()], kind.plural());
            }
        });
        egui::Grid::new("find").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            crate::widgets::field_label(ui, "In Area");
            let shown = f.area.map_or_else(|| "(all areas)".to_string(), |a| a.to_string());
            egui::ComboBox::from_id_salt("find-area").selected_text(shown).show_ui(ui, |ui| {
                ui.selectable_value(&mut f.area, None, "(all areas)");
                for a in &areas {
                    ui.selectable_value(&mut f.area, Some(*a), a.to_string());
                }
            });
            ui.end_row();
            crate::widgets::field_label(ui, "From Blueprint");
            let field =
                ui.add(egui::TextEdit::singleline(&mut f.template).hint_text("blueprint resref"));
            crate::widgets::autofocus(ui, &field);
            ui.end_row();
            crate::widgets::field_label(ui, "With Tag");
            ui.add(egui::TextEdit::singleline(&mut f.tag).hint_text("tag"));
            ui.end_row();
        });
        ui.horizontal(|ui| {
            if ui.button("Search").clicked() {
                f.results = search(app, &f);
            }
            if ui.button("Clear").clicked() {
                f = FindInstance::default();
            }
        });
        ui.separator();
        ui.weak(format!("{} found; double click one to go to it", f.results.len()));
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            egui::Grid::new("found").num_columns(4).striped(true).show(ui, |ui| {
                for h in ["Type", "Tag", "Area", "Blueprint"] {
                    ui.strong(h);
                }
                ui.end_row();
                for r in &f.results {
                    let row = [format!("{:?}", r.kind), r.tag.clone(), r.area.to_string()];
                    let mut double = false;
                    for text in row {
                        double |= ui.selectable_label(false, text).double_clicked();
                    }
                    double |= ui.selectable_label(false, &r.template).double_clicked();
                    if double {
                        go = Some(r.clone());
                    }
                    ui.end_row();
                }
            });
        });
    });
    if let Some(r) = go {
        app.area_focus = Some((r.area, r.kind, r.index));
        app.actions.push(Action::OpenTab(Tab::Area(r.area)));
    }
    if open {
        app.find_instance = Some(f);
    }
}

/// Aurora's Preview window (`TfrmPreview`): the blueprint chosen in the
/// palette, in 3D for creatures, items, placeables and doors, with its
/// name, tag, resref and comments and a few fields of its type.
pub(crate) fn preview_window(app: &mut Moonglow, ui: &mut Ui) {
    if !app.preview_window {
        return;
    }
    let mut open = true;
    let key = app.palette.selected;
    egui::Window::new("Preview").open(&mut open).default_size([360.0, 460.0]).show(
        ui.ctx(),
        |ui| {
            let Some(key) = key else {
                ui.weak("Choose a blueprint in the palette.");
                return;
            };
            let gff = blueprint(app, key);
            let Some(gff) = gff else {
                ui.colored_label(ui.visuals().error_fg_color, format!("{key}: not found"));
                return;
            };
            ui.horizontal_top(|ui| {
                // Items also in 2D: the inventory icon.
                if key.restype == ResType::UTI {
                    let layers = app.item_icon(ui.ctx(), &gff.root);
                    crate::images::stacked(ui, &layers, 1.0, "Item icon");
                }
                egui::Grid::new("preview-fields").num_columns(2).spacing([12.0, 4.0]).show(
                    ui,
                    |ui| {
                        for (label, value) in summary(app, key.restype, &gff.root) {
                            crate::widgets::field_label(ui, label);
                            ui.add(egui::Label::new(value).truncate());
                            ui.end_row();
                        }
                    },
                );
            });
            if crate::model_view::previewable(key.restype) {
                ui.separator();
                ui.allocate_ui(egui::vec2(ui.available_width(), 300.0), |ui| {
                    crate::model_view::ui(app, ui, crate::model_view::Source::Resource(key));
                });
            }
            if key.restype == ResType::UTM {
                egui::ScrollArea::vertical().show(ui, |ui| store_stock(app, ui, &gff.root));
            }
        },
    );
    app.preview_window = open;
}

/// A blueprint without a model (a store, a sound, a trigger, a waypoint,
/// an encounter) where the model viewer would show one: its fields, as the
/// Preview window lists them, and a store's stock by page.
pub(crate) fn summary_view(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let Some(gff) = blueprint(app, key) else {
        ui.colored_label(ui.visuals().error_fg_color, format!("{key}: not found"));
        return;
    };
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Grid::new(("summary", key)).num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            for (label, value) in summary(app, key.restype, &gff.root) {
                crate::widgets::field_label(ui, label);
                ui.label(value);
                ui.end_row();
            }
        });
        if key.restype == ResType::UTM {
            store_stock(app, ui, &gff.root);
        }
    });
}

/// A store's items, by page: icon, name, and whether it never runs out.
fn store_stock(app: &mut Moonglow, ui: &mut Ui, store: &mg_gff::Struct) {
    let pages = store.list("StoreList").unwrap_or(&[]);
    if pages.iter().all(|p| p.list("ItemList").is_none_or(<[_]>::is_empty)) {
        ui.add_space(8.0);
        ui.weak("It sells nothing.");
        return;
    }
    for (id, page) in crate::blueprint::store::STORE_PAGES {
        let items = pages.iter().find(|p| p.id == id).and_then(|p| p.list("ItemList"));
        let Some(items) = items.filter(|l| !l.is_empty()) else { continue };
        ui.add_space(8.0);
        ui.strong(format!("{page} ({})", items.len()));
        for entry in items {
            let resref = entry.resref("InventoryRes").unwrap_or(ResRef::EMPTY);
            // A whole item (placed stores hold them) or its blueprint.
            let item = match entry.integer("BaseItem") {
                Some(_) => Some(entry.clone()),
                None => blueprint(app, ResKey::new(resref, ResType::UTI)).map(|g| g.root),
            };
            let icon = item.as_ref().map(|i| app.item_icon(ui.ctx(), i)).unwrap_or_default();
            let name = item
                .as_ref()
                .and_then(|i| i.locstring("LocalizedName"))
                .and_then(|ls| app.game.as_ref()?.locstring(ls))
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| resref.to_string());
            ui.horizontal(|ui| {
                crate::images::icon_box(ui, &icon, crate::images::ICON_MAX, &name);
                ui.label(&name).on_hover_text(resref.to_string());
                if entry.integer("Infinite").unwrap_or(0) != 0 {
                    ui.weak("infinite");
                }
            });
        }
    }
}

/// A blueprint from the module, else the game.
fn blueprint(app: &Moonglow, key: ResKey) -> Option<mg_gff::Gff> {
    let data = app
        .ws
        .as_ref()
        .and_then(|w| w.module.get(&key).map(<[u8]>::to_vec))
        .or_else(|| app.game.as_ref()?.resman.get(&key).ok().map(|d| d.into_owned()))?;
    mg_gff::Gff::read(&data).ok()
}

/// The Preview window's fields for a blueprint (as Aurora's).
fn summary(app: &Moonglow, t: ResType, s: &mg_gff::Struct) -> Vec<(&'static str, String)> {
    let game = app.game.as_ref();
    let text = |label: &str| s.string(label).map(|v| String::from_utf8_lossy(v).into_owned());
    let name = |label: &str| {
        let ls = s.locstring(label)?;
        game.and_then(|g| g.locstring(ls))
    };
    let int = |label: &str| s.integer(label).map(|v| v.to_string()).unwrap_or_default();
    let yes =
        |label: &str| if s.integer(label).unwrap_or(0) != 0 { "yes" } else { "no" }.to_string();
    let resref_field = if t == ResType::UTM { "ResRef" } else { "TemplateResRef" };
    let mut out = vec![
        (
            "Name",
            name(match t {
                ResType::UTC => "FirstName",
                ResType::UTI | ResType::UTW | ResType::UTT | ResType::UTE => "LocalizedName",
                _ => "LocName",
            })
            .unwrap_or_default(),
        ),
        ("Tag", text("Tag").unwrap_or_default()),
        ("Blueprint ResRef", s.resref(resref_field).map(|r| r.to_string()).unwrap_or_default()),
    ];
    match t {
        ResType::UTC => {
            out.push((
                "Challenge Rating",
                s.float("ChallengeRating").map(|v| v.to_string()).unwrap_or_default(),
            ));
            out.push(("Faction", int("FactionID")));
        }
        ResType::UTD => {
            out.push(("Trap Type", int("TrapType")));
            out.push(("Faction", int("Faction")));
            out.push(("Destination Tag", text("LinkedTo").unwrap_or_default()));
            out.push(("Locked", yes("Locked")));
        }
        ResType::UTE => {
            out.push(("Difficulty", int("DifficultyIndex")));
            out.push(("Spawn Option", int("SpawnOption")));
            out.push(("Faction", int("Faction")));
        }
        ResType::UTI => {
            let cost = game.map(|g| g.item_cost(&mg_rules::ItemValue::from_gff(s)));
            out.push(("Total Cost", cost.map(|c| c.to_string()).unwrap_or_default()));
        }
        ResType::UTP => {
            out.push(("Faction", int("Faction")));
            out.push(("Trap Type", int("TrapType")));
            out.push(("Locked", yes("Locked")));
        }
        ResType::UTS => {
            out.push(("Volume", int("Volume")));
            out.push(("Active", yes("Active")));
        }
        ResType::UTT => {
            out.push(("Trigger Type", int("Type")));
            out.push(("Destination Tag", text("LinkedTo").unwrap_or_default()));
            out.push(("Faction", int("Faction")));
            out.push(("Trap Type", int("TrapType")));
        }
        _ => {}
    }
    out.push(("Comments", text("Comment").unwrap_or_default()));
    out
}
