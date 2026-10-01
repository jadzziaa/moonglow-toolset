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
                    ui.label(label);
                    let r = ui
                        .add(egui::DragValue::new(&mut a.position[i]).speed(0.05).max_decimals(3));
                    a.changed[i] |= r.changed();
                    ui.end_row();
                }
                ui.label("Bearing (°)")
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
            ui.strong("Visual Transforms");
            egui::Grid::new("adjust-visual").num_columns(4).spacing([12.0, 6.0]).show(ui, |ui| {
                ui.label("Scale");
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
                if ui.button("OK").clicked() {
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
            ui.label("Search For");
            for kind in ObjectKind::ALL {
                ui.checkbox(&mut f.kinds[kind.index()], kind.plural());
            }
        });
        egui::Grid::new("find").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label("In Area");
            let shown = f.area.map_or_else(|| "(all areas)".to_string(), |a| a.to_string());
            egui::ComboBox::from_id_salt("find-area").selected_text(shown).show_ui(ui, |ui| {
                ui.selectable_value(&mut f.area, None, "(all areas)");
                for a in &areas {
                    ui.selectable_value(&mut f.area, Some(*a), a.to_string());
                }
            });
            ui.end_row();
            ui.label("From Blueprint");
            ui.add(egui::TextEdit::singleline(&mut f.template).hint_text("blueprint resref"));
            ui.end_row();
            ui.label("With Tag");
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
