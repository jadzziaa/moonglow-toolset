//! File > Export and File > Import: resources to and from `.erf` archives,
//! with what they depend on.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use egui::Ui;
use mg_core::ResType;
use mg_module::transfer::{ImportPlan, plan_export, plan_import};
use mg_resman::{ResKey, ResMan};

use crate::{Action, FileKind, Moonglow};

/// The Export window's choices.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExportDraft {
    pub selected: BTreeSet<ResKey>,
    pub filter: String,
    /// The kind of resource listed; `None`: all of them, kind by kind.
    pub kind: Option<ResType>,
    /// Also export the module resources the chosen ones use.
    pub dependencies: bool,
    /// Move creatures in the module's own factions to the standard ones.
    pub reset_factions: bool,
    pub comments: String,
    /// Add to the archive chosen, if there is one, instead of replacing it.
    pub add: bool,
    /// The module's keys in the window's order, at a workspace revision,
    /// and those the kind and filter then listed: made again only when
    /// one of them changes (a module's 25,000 sorted and each named for
    /// the filter, every frame).
    pub(crate) listed: Option<Listed>,
}

/// [`ExportDraft::listed`]: the revision, kind and filter, the keys in
/// order, and those listed.
pub(crate) type Listed = (u64, Option<ResType>, String, Arc<Vec<ResKey>>, Arc<Vec<ResKey>>);

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
    /// What the archive was made from, as Moonglow's export stamps it
    /// (`None`: no stamp, as from Aurora).
    pub stamp: Option<String>,
    /// Why the module may have moved on since (another commit now).
    pub base_warning: Option<String>,
}

/// What To Scratch's hover says: where it copies `what`, or that it asks.
pub(crate) fn scratch_tip(scratch: Option<&std::path::Path>, what: &str) -> String {
    match scratch {
        Some(d) => format!(
            "Copy {what} into the scratch folder, {} (Tools › Options › Folders changes it)",
            d.display()
        ),
        None => format!(
            "Copy {what} into a scratch folder: asked for once, then kept (Tools › Options › \
             Folders)"
        ),
    }
}

/// Names for the log: all of a few, else the first and how many more.
pub(crate) fn listed(names: &[String]) -> String {
    const SHOWN: usize = 8;
    if names.len() <= SHOWN {
        names.join(", ")
    } else {
        format!("{} and {} more", names[..SHOWN].join(", "), names.len() - SHOWN)
    }
}

/// The rule for a script that doesn't compile, copied into the scratch
/// folder (a script editor's To Scratch, the module tree's Copy to Scratch
/// Folder): it goes as its source alone, and the compiled script (and its
/// debug file) of each of `scripts` that an earlier copy left in `dir` is
/// removed, for the game would still run it (El Grillo's request). What
/// was removed, by file name, and what could not be.
pub(crate) fn remove_stale_compiled(
    dir: &std::path::Path,
    scripts: &[mg_core::ResRef],
) -> (Vec<String>, Vec<String>) {
    let (mut removed, mut failed) = (Vec::new(), Vec::new());
    for resref in scripts {
        for kind in [ResType::NCS, ResType::NDB] {
            let name = ResKey::new(*resref, kind).to_string();
            let path = dir.join(&name);
            if !path.is_file() {
                continue;
            }
            match std::fs::remove_file(&path) {
                Ok(()) => removed.push(name),
                Err(e) => failed.push(format!("{}: {e}", path.display())),
            }
        }
    }
    (removed, failed)
}

/// What the log says of the compiled scripts [`remove_stale_compiled`]
/// removed from the scratch folder.
pub(crate) fn removed_said(removed: &[String]) -> String {
    format!("removed {}, left by an earlier copy (the game would still run it)", listed(removed))
}

impl Moonglow {
    /// The scratch folder To Scratch copies into: chosen once, then kept
    /// in the settings (Options › Folders); None if none is chosen when
    /// asked.
    pub(crate) fn scratch_folder(&mut self) -> Option<PathBuf> {
        self.settings.scratch_dir.clone().or_else(|| {
            let start = self.module_path().and_then(|p| p.parent().map(|d| d.to_path_buf()));
            let picked = self.dialogs.pick_folder("Scratch folder", start.as_deref());
            self.settings.scratch_dir.clone_from(&picked);
            picked
        })
    }

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
        let resman = self.game.as_deref().map_or(&empty, |g| &g.resman);
        match plan_import(&ws.module, &data, resman) {
            Ok(ImportPlan { resources, overwrites, missing }) => {
                let stamp = mg_module::transfer::archive_stamp(&data);
                let base_warning =
                    stamp.as_ref().and_then(|s| mg_module::stamp::compare(s, &ws.module));
                self.import = Some(ImportDraft {
                    path,
                    data,
                    resources,
                    overwrites: overwrites.into_iter().map(|k| (k, false)).collect(),
                    missing: missing
                        .iter()
                        .map(|r| format!("{} → {:?} {}", r.from, r.kind, r.target))
                        .collect(),
                    stamp: stamp.map(|s| s.summary()),
                    base_warning,
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
        let edits = ws.edits_to(&staged);
        let name =
            draft.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if let Some(w) = &draft.base_warning {
            self.log.warn(format!("{name}: {w}"));
        }
        if !edits.is_empty()
            && let Err(e) = self.apply(mg_edit::Command::new(format!("Import {name}"), edits))
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

    /// Writes `keys` as loose files, each with what goes with it (a
    /// script's compiled script and debug file, an area's `.git` and
    /// `.gic`), as they are now (saved or not).
    pub(crate) fn run_export_files(
        &mut self,
        keys: Vec<ResKey>,
        dependencies: bool,
        scratch: bool,
    ) {
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let roots = if dependencies {
            let empty = ResMan::new();
            let resman = self.game.as_deref().map_or(&empty, |g| &g.resman);
            plan_export(&ws.module, &keys, resman).resources
        } else {
            keys
        };
        // A script changed since it was compiled is compiled first.
        // (A script going out is compiled whatever it includes: stale
        // compiles are found by compiling, not by what changed.)
        let scripts: Vec<ResKey> =
            roots.iter().filter(|k| k.restype == mg_core::ResType::NSS).copied().collect();
        let (compiled, broken) = crate::script_view::compile_stale(self, &scripts);
        if self.game.is_none() && !scripts.is_empty() {
            self.log.warn(
                "No game data to compile with: the scripts go out with the compiled scripts \
                 they have, which may be older than their source",
            );
        }
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let mut resources: Vec<ResKey> = Vec::new();
        for k in &roots {
            for f in mg_module::transfer::file_set(&ws.module, k) {
                if !resources.contains(&f) {
                    resources.push(f);
                }
            }
        }
        // A script without its compiled script is of no use to the game.
        let uncompiled: Vec<String> = resources
            .iter()
            .filter(|k| k.restype == mg_core::ResType::NSS)
            .filter(|k| !resources.contains(&ResKey::new(k.resref, mg_core::ResType::NCS)))
            .map(|k| k.resref.to_string())
            .collect();
        // To the scratch folder, one that no longer compiles goes as its
        // source alone, as To Scratch sends it: not with the older
        // compiled script, which the game would run as this script's.
        if scratch {
            resources.retain(|k| {
                !matches!(k.restype, ResType::NCS | ResType::NDB)
                    || !broken.contains(&k.resref.to_string())
            });
        }
        let dir = if scratch {
            self.scratch_folder()
        } else {
            let start = self
                .export_dir
                .clone()
                .or_else(|| self.module_path().and_then(|p| p.parent().map(|d| d.to_path_buf())));
            self.dialogs.pick_folder("Export as files into", start.as_deref())
        };
        let Some(dir) = dir else { return };
        let Some(ws) = &self.ws else { return };
        match mg_module::transfer::export_files(&ws.module, &resources, &dir) {
            Ok(written) => {
                let names: Vec<String> = written
                    .iter()
                    .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .collect();
                self.log.info(format!(
                    "Wrote {} to {}",
                    crate::transfer::listed(&names),
                    dir.display()
                ));
                if !compiled.is_empty() {
                    self.log.info(format!(
                        "Compiled first (changed since last compiled): {}",
                        crate::transfer::listed(&compiled)
                    ));
                }
                if !broken.is_empty() && scratch {
                    self.log.warn(format!(
                        "No longer compiles (its source copied alone; Compile in the script's \
                         editor says why): {}",
                        crate::transfer::listed(&broken)
                    ));
                } else if !broken.is_empty() {
                    self.log.warn(format!(
                        "No longer compiles (the .ncs written is older than the script): {}",
                        crate::transfer::listed(&broken)
                    ));
                }
                if !uncompiled.is_empty() {
                    self.log.warn(format!(
                        "Not compiled (no .ncs written; compile first): {}",
                        crate::transfer::listed(&uncompiled)
                    ));
                }
                if scratch {
                    let alone: Vec<mg_core::ResRef> =
                        (broken.iter().chain(&uncompiled)).filter_map(|n| n.parse().ok()).collect();
                    let (removed, failed) = remove_stale_compiled(&dir, &alone);
                    if !removed.is_empty() {
                        self.log.info(format!("Scratch folder: {}", removed_said(&removed)));
                    }
                    failed.into_iter().for_each(|f| self.log.error(f));
                } else {
                    self.export_dir = Some(dir);
                }
            }
            Err(e) => self.log.error(format!("Could not write to {}: {e}", dir.display())),
        }
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
            let resman = self.game.as_deref().map_or(&empty, |g| &g.resman);
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
        // Add to Existing File: what the archive holds stays, but for
        // resources of the same names.
        let existing = if draft.add { std::fs::read(&path).ok() } else { None };
        // What it was made from follows the description (Import reads it).
        let stamp = mg_module::stamp::Stamp::of(&ws.module);
        let comments = mg_module::stamp::describe(&draft.comments, &stamp);
        let (comments, reset) = (comments.as_str(), draft.reset_factions);
        let result = match &existing {
            Some(old) => {
                mg_module::transfer::export_into_erf(old, &ws.module, &resources, comments, reset)
            }
            None => mg_module::transfer::export_erf(&ws.module, &resources, comments, reset),
        }
        .map_err(|e| e.to_string())
        .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        match result {
            Ok(()) => self.log.info(format!(
                "{} {} resources to {}, stamped with {}",
                if existing.is_some() { "Added" } else { "Exported" },
                resources.len(),
                path.display(),
                stamp.summary()
            )),
            Err(e) => self.log.error(format!("Export failed: {e}")),
        }
    }
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    if let (Some(draft), Some(ws)) = (&mut app.export, &app.ws) {
        let mut close = false;
        // Kind by kind, those of the module's tree first and in its order.
        let rank = |t: ResType| {
            let group = crate::tree::GROUPS.iter().position(|(_, types)| types.contains(&t));
            (group.unwrap_or(usize::MAX), t.extension().unwrap_or_default())
        };
        let kind_name = |t: ResType| {
            let ext = t.extension().unwrap_or_default();
            match crate::tree::GROUPS.iter().find(|(_, types)| *types == [t]) {
                Some((name, _)) => format!("{name} (.{ext})"),
                None => format!(".{ext}"),
            }
        };
        let revision = ws.revision();
        let kept = (draft.listed.as_ref()).filter(|(r, ..)| *r == revision).map(|l| l.3.clone());
        let keys = kept.unwrap_or_else(|| {
            let mut keys: Vec<ResKey> = ws.module.keys().copied().collect();
            keys.sort_by_cached_key(|k| (rank(k.restype), k.resref));
            Arc::new(keys)
        });
        let mut kinds: Vec<(ResType, usize)> = Vec::new();
        for k in keys.iter() {
            match kinds.last_mut() {
                Some((t, n)) if *t == k.restype => *n += 1,
                _ => kinds.push((k.restype, 1)),
            }
        }
        if draft.kind.is_some_and(|t| kinds.iter().all(|(k, _)| *k != t)) {
            draft.kind = None;
        }
        crate::widgets::window("Export Resources").show(&ctx, |ui| {
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Kind");
                let shown = match draft.kind {
                    Some(t) => kind_name(t),
                    None => "All".to_string(),
                };
                let options: Vec<_> =
                    std::iter::once(None).chain(kinds.iter().map(|(t, _)| Some(*t))).collect();
                crate::widgets::dropdown("export-kind").selected_text(shown).width(200.0).show(
                    ui,
                    &mut draft.kind,
                    &options,
                    |t| match t.and_then(|t| kinds.iter().find(|k| k.0 == t)) {
                        Some((t, n)) => format!("{}: {n}", kind_name(*t)),
                        None => format!("All: {}", keys.len()),
                    },
                );
            });
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Filter");
                let width = ui.spacing().text_edit_width;
                let field = crate::widgets::SearchField::new(
                    ui.id().with("transfer-filter-field"),
                    &mut draft.filter,
                )
                .width(width)
                .find_over()
                .show(ui)
                .response;
                crate::widgets::autofocus(ui, &field);
            });
            let same = |(r, kind, filter, ..): &&Listed| {
                *r == revision && *kind == draft.kind && *filter == draft.filter
            };
            let listed = match draft.listed.as_ref().filter(same) {
                Some(l) => l.4.clone(),
                None => {
                    let filter = mg_core::text::Needle::new(&draft.filter);
                    let listed: Vec<ResKey> = keys
                        .iter()
                        .filter(|k| draft.kind.is_none_or(|t| k.restype == t))
                        .filter(|k| filter.finds(&k.to_string()))
                        .copied()
                        .collect();
                    let listed = Arc::new(listed);
                    let (kind, filter) = (draft.kind, draft.filter.clone());
                    draft.listed = Some((revision, kind, filter, keys.clone(), listed.clone()));
                    listed
                }
            };
            // The rows: with every kind listed, each under its heading.
            let mut rows: Vec<Result<ResKey, ResType>> = Vec::with_capacity(listed.len());
            for k in listed.iter() {
                let first = !matches!(rows.last(), Some(Ok(l)) if l.restype == k.restype);
                if draft.kind.is_none() && first {
                    rows.push(Err(k.restype));
                }
                rows.push(Ok(*k));
            }
            egui::Frame::group(ui.style()).show(ui, |ui| {
                let height = ui.spacing().interact_size.y;
                egui::ScrollArea::vertical().max_height(260.0).show_rows(
                    ui,
                    height,
                    rows.len(),
                    |ui, range| {
                        ui.set_width(320.0);
                        for row in &rows[range] {
                            let k = match row {
                                Ok(k) => k,
                                Err(t) => {
                                    let size = egui::vec2(ui.available_width(), height);
                                    let across = egui::Layout::left_to_right(egui::Align::Center);
                                    ui.allocate_ui_with_layout(size, across, |ui| {
                                        ui.strong(kind_name(*t));
                                    });
                                    continue;
                                }
                            };
                            let mut on = draft.selected.contains(k);
                            if ui.checkbox(&mut on, k.to_string()).changed() {
                                if on {
                                    draft.selected.insert(*k);
                                } else {
                                    draft.selected.remove(k);
                                }
                            }
                        }
                    },
                );
            });
            ui.horizontal(|ui| {
                let all = listed.iter().all(|k| draft.selected.contains(k));
                if ui
                    .add_enabled(
                        !all,
                        egui::Button::new(format!("Select the {} Listed", listed.len())),
                    )
                    .clicked()
                {
                    draft.selected.extend(listed.iter().copied());
                }
                let any = listed.iter().any(|k| draft.selected.contains(k));
                if ui.add_enabled(any, egui::Button::new("Clear Them")).clicked() {
                    for k in listed.iter() {
                        draft.selected.remove(k);
                    }
                }
            });
            ui.checkbox(&mut draft.dependencies, "Include the resources they use");
            ui.checkbox(&mut draft.reset_factions, "Put creatures in the standard factions");
            ui.checkbox(&mut draft.add, "Add to the file if it exists").on_hover_text(
                "Choosing an archive that is already there adds these resources to it \
                 (replacing those of the same names) instead of replacing the whole file. \
                 The file dialog may still ask about overwriting it.",
            );
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Description");
                ui.text_edit_singleline(&mut draft.comments).on_hover_text(
                    "What the export is made from follows it in the archive: the module's git \
                     commit, Moonglow's version and the module's name (Import shows it)",
                );
            });
            ui.horizontal(|ui| {
                let label = format!("Export {}…", draft.selected.len());
                if ui.add_enabled(!draft.selected.is_empty(), egui::Button::new(label)).clicked() {
                    app.actions.push(Action::Export(draft.clone()));
                    close = true;
                }
                let files = format!("Export {} as Files…", draft.selected.len());
                if ui
                    .add_enabled(!draft.selected.is_empty(), egui::Button::new(files))
                    .on_hover_text(
                        "Loose files in a folder (name.ext), as they are in the module: a \
                         script with its compiled .ncs, an area with its .git and .gic",
                    )
                    .clicked()
                {
                    app.actions.push(Action::ExportFiles {
                        keys: draft.selected.iter().copied().collect(),
                        dependencies: draft.dependencies,
                        scratch: false,
                    });
                    close = true;
                }
                if crate::widgets::cancel(ui) {
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
        crate::widgets::window("Import Resources").show(&ctx, |ui| {
            ui.label(format!("{name}: {} resources", draft.resources.len()));
            match &draft.stamp {
                Some(s) => ui.label(format!("Made from {s}")),
                None => ui.weak("It doesn't say what it was made from."),
            };
            if let Some(w) = &draft.base_warning {
                ui.colored_label(ui.visuals().warn_fg_color, format!("Careful: {w}."));
            }
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
                if crate::widgets::cancel(ui) {
                    close = true;
                }
            });
        });
        if close {
            app.import = None;
        }
    }
}
