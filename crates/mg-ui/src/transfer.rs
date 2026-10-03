//! File > Export and File > Import: resources to and from `.erf` archives,
//! with what they depend on.

use std::collections::BTreeSet;
use std::path::PathBuf;

use egui::Ui;
use mg_module::transfer::{ImportPlan, plan_export, plan_import};
use mg_resman::{ResKey, ResMan};

use crate::{Action, FileKind, Moonglow};

/// The Export window's choices.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExportDraft {
    pub selected: BTreeSet<ResKey>,
    pub filter: String,
    /// Also export the module resources the chosen ones use.
    pub dependencies: bool,
    /// Move creatures in the module's own factions to the standard ones.
    pub reset_factions: bool,
    pub comments: String,
}

/// An archive being imported and what to overwrite.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDraft {
    pub path: PathBuf,
    pub data: Vec<u8>,
    pub resources: Vec<ResKey>,
    /// Resources the module already has, and whether to overwrite each.
    pub overwrites: Vec<(ResKey, bool)>,
    /// References nothing satisfies, as text.
    pub missing: Vec<String>,
}

fn window(title: &str) -> egui::Window<'_> {
    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
}

impl Moonglow {
    pub(crate) fn open_import(&mut self) {
        let Some(ws) = &mut self.ws else { return };
        let Some(path) = self.dialogs.open_file(FileKind::Erf, None) else { return };
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                self.log.error(format!("Could not read {}: {e}", path.display()));
                return;
            }
        };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let empty = ResMan::new();
        let resman = self.game.as_ref().map_or(&empty, |g| &g.resman);
        match plan_import(&ws.module, &data, resman) {
            Ok(ImportPlan { resources, overwrites, missing }) => {
                self.import = Some(ImportDraft {
                    path,
                    data,
                    resources,
                    overwrites: overwrites.into_iter().map(|k| (k, false)).collect(),
                    missing: missing
                        .iter()
                        .map(|r| format!("{} → {:?} {}", r.from, r.kind, r.target))
                        .collect(),
                })
            }
            Err(e) => self.log.error(format!("Could not read {}: {e}", path.display())),
        }
    }

    pub(crate) fn run_import(&mut self, draft: ImportDraft) {
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let mut staged = ws.module.clone();
        let overwrite: BTreeSet<ResKey> =
            draft.overwrites.iter().filter(|(_, o)| *o).map(|(k, _)| *k).collect();
        let summary = match mg_module::transfer::import_erf(&mut staged, &draft.data, |k| {
            overwrite.contains(k)
        }) {
            Ok(s) => s,
            Err(e) => {
                self.log.error(format!("Import failed: {e}"));
                return;
            }
        };
        let edits: Vec<mg_edit::Edit> = staged
            .keys()
            .filter(|k| staged.get(k) != ws.module.get(k))
            .map(|k| mg_edit::Edit::SetResource {
                key: *k,
                data: staged.get(k).map(<[u8]>::to_vec),
            })
            .collect();
        let name =
            draft.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !edits.is_empty()
            && let Err(e) = ws.apply(mg_edit::Command::new(format!("Import {name}"), edits))
        {
            self.log.error(e.to_string());
            return;
        }
        self.log.info(format!(
            "Imported {name}: {} added, {} replaced, {} kept",
            summary.added.len(),
            summary.replaced.len(),
            summary.skipped.len()
        ));
        self.refresh_module_layer();
    }

    pub(crate) fn run_export(&mut self, draft: ExportDraft) {
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let roots: Vec<ResKey> = draft.selected.iter().copied().collect();
        let resources = if draft.dependencies {
            let empty = ResMan::new();
            let resman = self.game.as_ref().map_or(&empty, |g| &g.resman);
            let plan = plan_export(&ws.module, &roots, resman);
            for r in &plan.missing {
                self.log.warn(format!(
                    "Not exported, not found: {} → {:?} {}",
                    r.from, r.kind, r.target
                ));
            }
            plan.resources
        } else {
            roots.clone()
        };
        let suggested = self
            .module_path()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .zip(roots.first())
            .map(|(dir, k)| dir.join(format!("{}.erf", k.resref)));
        let Some(path) = self.dialogs.save_file(FileKind::Erf, suggested.as_deref()) else {
            return;
        };
        let Some(ws) = &self.ws else { return };
        let result = mg_module::transfer::export_erf(
            &ws.module,
            &resources,
            &draft.comments,
            draft.reset_factions,
        )
        .map_err(|e| e.to_string())
        .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        match result {
            Ok(()) => self.log.info(format!(
                "Exported {} resources to {}",
                resources.len(),
                path.display()
            )),
            Err(e) => self.log.error(format!("Export failed: {e}")),
        }
    }
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    if let (Some(draft), Some(ws)) = (&mut app.export, &app.ws) {
        let mut close = false;
        let mut keys: Vec<ResKey> = ws.module.keys().copied().collect();
        keys.sort_by_key(|k| (k.restype.extension().unwrap_or_default(), k.resref));
        window("Export Resources").show(&ctx, |ui| {
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Filter");
                let field = ui.text_edit_singleline(&mut draft.filter);
                crate::widgets::autofocus(ui, &field);
            });
            let filter = draft.filter.to_ascii_lowercase();
            egui::Frame::group(ui.style()).show(ui, |ui| {
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    ui.set_width(320.0);
                    for k in
                        keys.iter().filter(|k| filter.is_empty() || k.to_string().contains(&filter))
                    {
                        let mut on = draft.selected.contains(k);
                        if ui.checkbox(&mut on, k.to_string()).changed() {
                            if on {
                                draft.selected.insert(*k);
                            } else {
                                draft.selected.remove(k);
                            }
                        }
                    }
                });
            });
            ui.checkbox(&mut draft.dependencies, "Include the resources they use");
            ui.checkbox(&mut draft.reset_factions, "Put creatures in the standard factions");
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Description");
                ui.text_edit_singleline(&mut draft.comments);
            });
            ui.horizontal(|ui| {
                let label = format!("Export {}…", draft.selected.len());
                if ui.add_enabled(!draft.selected.is_empty(), egui::Button::new(label)).clicked() {
                    app.actions.push(Action::Export(draft.clone()));
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        if close {
            app.export = None;
        }
    }

    if let Some(draft) = &mut app.import {
        let mut close = false;
        let name =
            draft.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        window("Import Resources").show(&ctx, |ui| {
            ui.label(format!("{name}: {} resources", draft.resources.len()));
            if !draft.overwrites.is_empty() {
                ui.add_space(6.0);
                ui.label("The module already has these; overwrite:");
                egui::ScrollArea::vertical().id_salt("overwrites").max_height(180.0).show(
                    ui,
                    |ui| {
                        for (k, on) in &mut draft.overwrites {
                            ui.checkbox(on, k.to_string());
                        }
                    },
                );
            }
            if !draft.missing.is_empty() {
                ui.add_space(6.0);
                ui.colored_label(ui.visuals().warn_fg_color, "Used but found nowhere:");
                egui::ScrollArea::vertical().id_salt("missing").max_height(120.0).show(ui, |ui| {
                    for m in &draft.missing {
                        ui.monospace(m);
                    }
                });
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Import").clicked() || crate::widgets::enter(ui) {
                    app.actions.push(Action::Import(draft.clone()));
                    close = true;
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        if close {
            app.import = None;
        }
    }
}
