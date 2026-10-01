//! The Module and Area wizards (Aurora's File > New and Wizards > Area
//! Wizard), and the unsaved-changes question.

use egui::Ui;
use mg_module::new::{AREA_SIZE_RANGE, AREA_SIZES, AreaSpec, TilesetChoice};

use crate::{Action, Moonglow};

/// An open wizard window.
#[derive(Debug, Clone, PartialEq)]
pub enum Wizard {
    NewModule { name: String },
    NewArea(AreaWizard),
}

/// The Area Wizard's choices.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaWizard {
    pub name: String,
    pub tilesets: Vec<TilesetChoice>,
    pub selected: Option<usize>,
    pub width: u32,
    pub height: u32,
    /// Launch Area Properties Dialog (Aurora's default: off).
    pub launch_properties: bool,
    /// Open Area in the Area Viewer (Aurora's default: on).
    pub open_viewer: bool,
}

impl AreaWizard {
    /// Aurora's defaults: "Area NNN" after the module's areas, no tileset
    /// chosen yet, Medium (8×8).
    pub fn new(tilesets: Vec<TilesetChoice>, areas: usize) -> AreaWizard {
        AreaWizard {
            name: format!("Area {:03}", areas + 1),
            tilesets,
            selected: None,
            width: 8,
            height: 8,
            launch_properties: false,
            open_viewer: true,
        }
    }

    fn spec(&self) -> Option<AreaSpec> {
        let t = self.tilesets.get(self.selected?)?;
        Some(AreaSpec {
            name: self.name.trim().to_string(),
            tileset: t.resref,
            width: self.width,
            height: self.height,
        })
    }
}

fn window(title: &str) -> egui::Window<'_> {
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let mut close = false;
    match &mut app.wizard {
        None => {}
        Some(Wizard::NewModule { name }) => {
            window("New Module").show(&ctx, |ui| {
                ui.label("Module name");
                ui.text_edit_singleline(name);
                ui.horizontal(|ui| {
                    let ok = !name.trim().is_empty();
                    if ui.add_enabled(ok, egui::Button::new("Create")).clicked()
                        || (ok && crate::widgets::enter(ui))
                    {
                        app.actions.push(Action::NewModule(name.trim().to_string()));
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
        }
        Some(Wizard::NewArea(w)) => {
            window("Area Wizard").show(&ctx, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut w.name);
                ui.add_space(6.0);
                ui.label("Tileset");
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        ui.set_width(280.0);
                        for (i, t) in w.tilesets.iter().enumerate() {
                            if ui.selectable_label(w.selected == Some(i), &t.name).clicked() {
                                w.selected = Some(i);
                            }
                        }
                    });
                });
                ui.add_space(6.0);
                ui.label("Size");
                ui.horizontal(|ui| {
                    for (label, n) in AREA_SIZES {
                        if ui.selectable_label((w.width, w.height) == (n, n), label).clicked() {
                            (w.width, w.height) = (n, n);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut w.width).range(AREA_SIZE_RANGE));
                    ui.label("×");
                    ui.add(egui::DragValue::new(&mut w.height).range(AREA_SIZE_RANGE));
                    ui.weak("tiles, 10 m each");
                });
                ui.add_space(6.0);
                ui.checkbox(&mut w.launch_properties, "Launch Area Properties Dialog");
                ui.checkbox(&mut w.open_viewer, "Open Area in the Area Viewer");
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let spec = w.spec().filter(|s| !s.name.is_empty());
                    if (ui.add_enabled(spec.is_some(), egui::Button::new("Create")).clicked()
                        || (spec.is_some() && crate::widgets::enter(ui)))
                        && let Some(spec) = spec
                    {
                        app.actions.push(Action::NewArea(spec));
                        app.after_new_area = (w.open_viewer, w.launch_properties);
                        close = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
        }
    }
    if close {
        app.wizard = None;
    }

    if let Some(pending) = app.confirm_discard.clone() {
        let what = app
            .module_path()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "the new module".into());
        window("Unsaved Changes").show(&ctx, |ui| {
            ui.label(format!("Save changes to {what}?"));
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    app.actions.push(Action::SaveThen(Box::new(pending.clone())));
                    app.confirm_discard = None;
                }
                if ui.button("Don't Save").clicked() {
                    app.actions.push(Action::Proceed(Box::new(pending.clone())));
                    app.confirm_discard = None;
                }
                if ui.button("Cancel").clicked() {
                    app.confirm_discard = None;
                }
            });
        });
    }
}
