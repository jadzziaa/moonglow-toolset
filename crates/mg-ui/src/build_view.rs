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
    egui::Window::new("Build Module")
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .open(crate::widgets::open_unless_escape(ctx, "Build Module", &mut open))
        .collapsible(false)
        .default_width(560.0)
        .show(ctx, |ui| {
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
                            crate::widgets::field_label(ui, "Unused");
                            ui.checkbox(&mut w.unused_scripts, "Scripts");
                            ui.checkbox(&mut w.unused_conversations, "Conversations");
                            ui.checkbox(&mut w.unused_blueprints, "Blueprints");
                        });
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.add_enabled_ui(w.missing, |ui| {
                            crate::widgets::field_label(ui, "Missing Resources");
                            for (c, on) in &mut w.missing_of {
                                ui.checkbox(on, format!("{c:?}"));
                            }
                        });
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.add_enabled_ui(w.compile, |ui| {
                            crate::widgets::field_label(ui, "Compile");
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
            crate::widgets::field_label(ui, "Results");
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
        });
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
    let options = build.then(|| w.clone());
    if open && !done {
        app.build = Some(w);
    }
    // The build, in the background; its results come to the window when it
    // is done (the window's state is back in place by then).
    if let Some(options) = options {
        app.start_job(
            "Build Module",
            move |job| job.game.as_deref().map(|game| work(&job.module, game, &options)),
            |app, built| {
                let results = match built {
                    Some(built) => finish(app, built),
                    None => vec![Finding { text: "No module open".into(), about: None }],
                };
                if let Some(w) = &mut app.build {
                    w.results = results;
                    w.selected = None;
                }
            },
        );
    }
}

/// Options › General › Build module on save: the build with Aurora's
/// defaults before saving; the Build Module window opens with the results
/// if it found problems.
pub(crate) fn build_on_save(app: &mut Moonglow) {
    let was_open = app.build.is_some();
    let mut w = app.build.take().unwrap_or_default();
    let results = run(app, &BuildWindow::default());
    let clean =
        results.len() == 1 && results[0].about.is_none() && results[0].text == "No errors found";
    w.results = results;
    w.selected = None;
    if was_open || !clean {
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

/// Runs the build and waits for it (the build before saving): the compile
/// passes, as one undoable command, then the checks. The results are the
/// problems found ("No errors found" if none, as in Aurora); what each pass
/// did goes to the log.
fn run(app: &mut Moonglow, w: &BuildWindow) -> Vec<Finding> {
    app.refresh_module_layer();
    let none = || vec![Finding { text: "No module open".into(), about: None }];
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else { return none() };
    let built = match ws.snapshot() {
        Ok(module) => work(&module, game, w),
        Err(e) => return vec![Finding { text: e.to_string(), about: None }],
    };
    finish(app, built)
}

/// What a build made of a module.
#[derive(Debug)]
struct Built {
    /// What the compile passes changed.
    edits: Vec<mg_edit::Edit>,
    /// The problems found.
    out: Vec<Finding>,
    /// What each pass did.
    notes: Vec<String>,
}

/// The build's work, on the module as it is (a job's snapshot): the compile
/// passes, then the checks on what they made.
fn work(module: &mg_module::Module, game: &mg_rules::GameData, w: &BuildWindow) -> Built {
    let (mut out, mut notes) = (Vec::new(), Vec::new());
    let staged = compile(module, game, w, &mut out, &mut notes);
    check(&staged, game, w, &mut out, &mut notes);
    Built { edits: mg_edit::edits_between(module, &staged), out, notes }
}

/// Puts a build's work into the application: its edits as one command, its
/// passes and problems in the log (Aurora's: what it is doing, the results,
/// done); the problems.
fn finish(app: &mut Moonglow, built: Built) -> Vec<Finding> {
    let Built { edits, mut out, notes } = built;
    let line = |text: String, about: Option<ResKey>| Finding { text, about };
    if !edits.is_empty()
        && let Err(e) = app.apply(Command::new("Build Module", edits))
    {
        out.push(line(format!("Error: {e}"), None));
    }
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

/// The build's compile passes: the module with what they made, their
/// problems in `out` and what each did in `notes`.
fn compile(
    module: &mg_module::Module,
    game: &mg_rules::GameData,
    w: &BuildWindow,
    out: &mut Vec<Finding>,
    notes: &mut Vec<String>,
) -> mg_module::Module {
    let line = |text: String, about: Option<ResKey>| Finding { text, about };
    let mut staged = module.clone();
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
            let item = |r: ResRef| read_gff(module, game, ResKey::new(r, ResType::UTI));
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
    }
    staged
}

/// The build's checks (missing and unused resources), on the module as the
/// compile passes left it.
fn check(
    module: &mg_module::Module,
    game: &mg_rules::GameData,
    w: &BuildWindow,
    out: &mut Vec<Finding>,
    notes: &mut Vec<String>,
) {
    let line = |text: String, about: Option<ResKey>| Finding { text, about };
    if w.missing {
        let missing = mg_module::verify::missing(module, &game.resman);
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
        let unused: Vec<ResKey> = mg_module::verify::unused(module)
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
