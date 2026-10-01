//! Build › Build Module (Aurora's `TdlgVerifyModule`): compile passes
//! (scripts, creature challenge ratings, encounters, palettes), then checks
//! for missing resources and, if asked, unused ones, listed as results; a double click opens what a result is about,
//! and the list can be exported as text. Aurora's defaults: Compile and
//! Missing Resources on, Unused and Spell Check off.

use std::collections::BTreeMap;

use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_module::verify::Category;
use mg_resman::ResKey;

use crate::dialogs::FileKind;
use crate::{Action, Moonglow, Tab};

/// The categories Missing Resources checks, in Aurora's order.
pub const MISSING: [Category; 11] = [
    Category::Creatures,
    Category::Doors,
    Category::Placeables,
    Category::Items,
    Category::Sounds,
    Category::Triggers,
    Category::Waypoints,
    Category::Stores,
    Category::Conversations,
    Category::Encounters,
    Category::Areas,
];

/// One line of the results, and the resource it is about.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub text: String,
    pub about: Option<ResKey>,
}

/// The Build Module window.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildWindow {
    /// Advanced Controls: the options are shown.
    pub advanced: bool,
    pub compile: bool,
    pub compile_scripts: bool,
    pub compile_cr: bool,
    pub compile_encounters: bool,
    pub compile_palettes: bool,
    pub missing: bool,
    pub missing_of: BTreeMap<Category, bool>,
    pub unused: bool,
    pub unused_scripts: bool,
    pub unused_conversations: bool,
    pub unused_blueprints: bool,
    pub results: Vec<Finding>,
    pub selected: Option<usize>,
}

impl Default for BuildWindow {
    fn default() -> BuildWindow {
        BuildWindow {
            advanced: false,
            compile: true,
            compile_scripts: true,
            compile_cr: true,
            compile_encounters: true,
            compile_palettes: true,
            missing: true,
            missing_of: MISSING.iter().map(|c| (*c, true)).collect(),
            unused: false,
            unused_scripts: true,
            unused_conversations: true,
            unused_blueprints: true,
            results: Vec::new(),
            selected: None,
        }
    }
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut w) = app.build.take() else { return };
    let mut open = true;
    let (mut build, mut done, mut export, mut go) = (false, false, false, None);
    egui::Window::new("Build Module").open(&mut open).collapsible(false).default_width(560.0).show(
        ctx,
        |ui| {
            // The four checks, always; Advanced Controls shows what each
            // covers.
            ui.horizontal(|ui| {
                ui.checkbox(&mut w.unused, "Unused").on_hover_text("Check for use");
                ui.checkbox(&mut w.missing, "Missing Resources")
                    .on_hover_text("Check that resources are available");
                ui.checkbox(&mut w.compile, "Compile");
                ui.add_enabled(false, egui::Checkbox::new(&mut false, "Spell Check"))
                    .on_disabled_hover_text("Not yet: Moonglow has no dictionary");
            });
            ui.checkbox(&mut w.advanced, "Advanced Controls");
            if w.advanced {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.add_enabled_ui(w.unused, |ui| {
                            ui.label("Unused");
                            ui.checkbox(&mut w.unused_scripts, "Scripts");
                            ui.checkbox(&mut w.unused_conversations, "Conversations");
                            ui.checkbox(&mut w.unused_blueprints, "Blueprints");
                        });
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.add_enabled_ui(w.missing, |ui| {
                            ui.label("Missing Resources");
                            for (c, on) in &mut w.missing_of {
                                ui.checkbox(on, format!("{c:?}"));
                            }
                        });
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.add_enabled_ui(w.compile, |ui| {
                            ui.label("Compile");
                            ui.checkbox(&mut w.compile_scripts, "Scripts");
                            ui.checkbox(&mut w.compile_cr, "Creature CR")
                                .on_hover_text("Recalculate creature challenge ratings");
                            ui.checkbox(&mut w.compile_encounters, "Encounters");
                            ui.checkbox(&mut w.compile_palettes, "Palettes");
                        });
                    });
                });
            }
            ui.separator();
            ui.label("Results");
            egui::ScrollArea::vertical().max_height(260.0).auto_shrink([false, true]).show(
                ui,
                |ui| {
                    for (i, f) in w.results.iter().enumerate() {
                        let r = ui.selectable_label(w.selected == Some(i), &f.text);
                        if r.clicked() {
                            w.selected = Some(i);
                        }
                        if r.double_clicked() {
                            go = f.about;
                        }
                    }
                },
            );
            ui.horizontal(|ui| {
                build = ui.button("Build").clicked();
                export =
                    ui.add_enabled(!w.results.is_empty(), egui::Button::new("Export…")).clicked();
                done = ui.button("Done").clicked();
            });
        },
    );
    if build {
        w.results = run(app, &w);
        w.selected = None;
    }
    if export
        && let Some(path) =
            app.dialogs.save_file(FileKind::Any, Some(std::path::Path::new("build.txt")))
    {
        let text: String = w.results.iter().map(|f| format!("{}\n", f.text)).collect();
        if let Err(e) = std::fs::write(&path, text) {
            app.log.error(format!("{}: {e}", path.display()));
        }
    }
    if let Some(t) = go.and_then(Tab::for_resource) {
        app.actions.push(Action::OpenTab(t));
    }
    if open && !done {
        app.build = Some(w);
    }
}

/// A blueprint type (the Unused › Blueprints check).
fn is_blueprint(t: ResType) -> bool {
    matches!(
        t,
        ResType::UTC
            | ResType::UTD
            | ResType::UTE
            | ResType::UTI
            | ResType::UTP
            | ResType::UTS
            | ResType::UTM
            | ResType::UTT
            | ResType::UTW
    )
}

/// Runs the build: the compile passes, as one undoable command, then the
/// checks. The results are the problems found ("No errors found" if none,
/// as in Aurora); what each pass did goes to the log.
fn run(app: &mut Moonglow, w: &BuildWindow) -> Vec<Finding> {
    app.refresh_module_layer();
    let line = |text: String, about: Option<ResKey>| Finding { text, about };
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_ref()) else {
        return vec![line("No module open".into(), None)];
    };
    let (mut out, notes) = build(ws, game, w);
    // Aurora's log: what it is doing, the results, done.
    app.log.info("Building Module...");
    for n in notes {
        app.log.info(n);
    }
    if out.is_empty() {
        out.push(line("No errors found".into(), None));
    }
    for f in &out {
        app.log.info(f.text.clone());
    }
    app.log.info("Finished Building Module");
    out
}

/// The build's problems, and what each pass did.
fn build(
    ws: &mut mg_edit::Workspace,
    game: &mg_rules::GameData,
    w: &BuildWindow,
) -> (Vec<Finding>, Vec<String>) {
    let (mut out, mut notes) = (Vec::new(), Vec::new());
    let line = |text: String, about: Option<ResKey>| Finding { text, about };
    if let Err(e) = ws.flush() {
        return (vec![line(e.to_string(), None)], notes);
    }
    let mut staged = ws.module.clone();
    if w.compile {
        if w.compile_scripts {
            let results = mg_module::build::compile_scripts(
                &mut staged,
                &game.resman,
                mg_module::build::ScriptSelection::All,
            );
            let failed = results.iter().filter(|r| r.result.is_err()).count();
            for r in &results {
                if let Err(e) = &r.result {
                    out.push(line(
                        format!("Error: {}", e.message),
                        Some(ResKey::new(r.script.resref, ResType::NSS)),
                    ));
                }
            }
            notes.push(format!("Build: compiled {} scripts, {failed} with errors", results.len()));
        }
        if w.compile_cr {
            let item = |r: ResRef| read_gff(&ws.module, game, ResKey::new(r, ResType::UTI));
            let n = mg_module::build::compile_creature_cr(&mut staged, game, &item);
            notes.push(format!("Build: {n} creature challenge ratings brought up to date"));
        }
        if w.compile_encounters {
            // The creatures as they are now, ratings included.
            let snapshot = staged.clone();
            let read = |r: ResRef| read_gff(&snapshot, game, ResKey::new(r, ResType::UTC));
            let n = mg_module::build::compile_encounters(&mut staged, &read);
            notes.push(format!("Build: {n} encounter creature entries brought up to date"));
        }
        if w.compile_palettes {
            match mg_module::palette::rebuild_custom_palettes(&mut staged, game) {
                Ok(n) => notes.push(format!("Build: {n} custom palettes rebuilt")),
                Err(e) => out.push(line(format!("Error: palettes: {e}"), None)),
            }
        }
        let edits: Vec<mg_edit::Edit> = staged
            .keys()
            .chain(ws.module.keys())
            .copied()
            .collect::<std::collections::BTreeSet<ResKey>>()
            .into_iter()
            .filter(|k| staged.get(k) != ws.module.get(k))
            .map(|k| mg_edit::Edit::SetResource {
                key: k,
                data: staged.get(&k).map(<[u8]>::to_vec),
            })
            .collect();
        if !edits.is_empty()
            && let Err(e) = ws.apply(Command::new("Build Module", edits))
        {
            out.push(line(format!("Error: {e}"), None));
        }
    }
    if w.missing {
        let missing = mg_module::verify::missing(&ws.module, &game.resman);
        let mut n = 0;
        for m in missing.iter().filter(|m| w.missing_of.get(&m.category).copied().unwrap_or(true)) {
            n += 1;
            let what = if m.uncompiled { "is not compiled" } else { "is missing" };
            out.push(line(
                format!(
                    "{:?}: {:?} {} ({} {}) {what}",
                    m.category,
                    m.reference.kind,
                    m.reference.target,
                    m.reference.from,
                    m.reference.path
                ),
                Some(m.reference.from),
            ));
        }
        notes.push(format!("Build: {n} missing resources"));
    }
    if w.unused {
        let unused: Vec<ResKey> = mg_module::verify::unused(&ws.module)
            .into_iter()
            .filter(|k| match k.restype {
                ResType::NSS | ResType::NCS => w.unused_scripts,
                ResType::DLG => w.unused_conversations,
                t => is_blueprint(t) && w.unused_blueprints,
            })
            .collect();
        for k in &unused {
            out.push(line(format!("Unused: {k}"), Some(*k)));
        }
        notes.push(format!("Build: {} unused resources", unused.len()));
    }
    (out, notes)
}

/// A GFF resource from the module, else the game.
fn read_gff(
    module: &mg_module::Module,
    game: &mg_rules::GameData,
    k: ResKey,
) -> Option<mg_gff::Struct> {
    let data = module
        .get(&k)
        .map(<[u8]>::to_vec)
        .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()))?;
    mg_gff::Gff::read(&data).ok().map(|g| g.root)
}
