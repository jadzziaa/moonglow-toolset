//! Build › Build Module (Aurora's `TdlgVerifyModule`): compile passes
//! (scripts, creature challenge ratings, encounters, palettes), then checks
//! for missing resources and, if asked, unused ones, listed as results; a double click opens what a result is about,
//! and the list can be exported as text. Aurora's defaults: Compile and
//! Missing Resources on, Unused and Spell Check off.

use std::collections::BTreeMap;

use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_module::project_file::UsedRules;
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
    /// Of an Unused line, what is unused (Mark as Used takes it).
    pub unused: Option<mg_module::verify::Unused>,
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
    /// Counted as Used: the name or glob typed, and the key of `[used]`
    /// it goes in (`scripts`, `conversations`, `blueprints`).
    pub mark_draft: String,
    pub mark_kind: &'static str,
    /// The external compiler a build is given (Options › Script Editor),
    /// set as it starts.
    pub(crate) external: Option<mg_module::external_compiler::ExternalCompiler>,
    /// Generate Debug Information (Options › Script Editor), set as a
    /// build starts: the scripts' debug files are made too.
    pub(crate) debug: bool,
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
            mark_draft: String::new(),
            mark_kind: "scripts",
            external: None,
            debug: false,
        }
    }
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut w) = app.build.take() else { return };
    let mut open = true;
    let (mut build, mut done, mut export, mut go) = (false, false, false, None);
    // (Mark as Used: `[used]`'s key, the name or glob, added or taken out.)
    let mut mark: Option<(&'static str, String, bool)> = None;
    let place = app
        .ws
        .as_ref()
        .and_then(|ws| ws.module.location.as_ref())
        .and_then(mg_module::project_file::ProjectFile::place_to_write);
    let used = app.ws.as_ref().map(|ws| ws.module.project_file.used.clone()).unwrap_or_default();
    crate::widgets::dialog(ctx, "Build Module", &mut open)
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
                    // (A gap, not a divider: a divider in a row takes all
                    // the height there is and the window grew to the
                    // screen's, its buttons out of reach.)
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.add_enabled_ui(w.missing, |ui| {
                            crate::widgets::field_label(ui, "Missing Resources");
                            for (c, on) in &mut w.missing_of {
                                ui.checkbox(on, format!("{c:?}"));
                            }
                        });
                    });
                    ui.add_space(16.0);
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
                        let Some(u) = f.unused else { continue };
                        r.context_menu(|ui| {
                            let r =
                                ui.add_enabled(place.is_some(), egui::Button::new("Mark as Used"));
                            if r.on_disabled_hover_text(NO_PLACE)
                                .on_hover_text(MARK_HOVER)
                                .clicked()
                                && let Some(kind) = UsedRules::key_for(u.key.restype)
                            {
                                mark = Some((kind, u.key.resref.to_string(), true));
                                ui.close();
                            }
                        });
                    }
                },
            );
            let chosen = w.selected.and_then(|i| w.results.get(i)).and_then(|f| f.unused);
            ui.horizontal(|ui| {
                build = ui.button("Build").clicked();
                let can = chosen.is_some() && place.is_some();
                let r = ui.add_enabled(can, egui::Button::new("Mark as Used"));
                let r = if place.is_none() {
                    r.on_disabled_hover_text(NO_PLACE)
                } else {
                    r.on_disabled_hover_text("Choose an Unused result first")
                };
                if r.on_hover_text(MARK_HOVER).clicked()
                    && let Some(u) = chosen
                    && let Some(kind) = UsedRules::key_for(u.key.restype)
                {
                    mark = Some((kind, u.key.resref.to_string(), true));
                }
                export =
                    ui.add_enabled(!w.results.is_empty(), egui::Button::new("Export…")).clicked();
                done = ui.button("Done").clicked();
            });
            if w.unused {
                egui::CollapsingHeader::new("Counted as Used").id_salt("build-used").show(
                    ui,
                    |ui| {
                        if let Some(change) = marks(ui, &mut w, &used, place.as_ref()) {
                            mark = Some(change);
                        }
                    },
                );
            }
        });
    if let Some((kind, pattern, add)) = mark
        && set_used(app, kind, &pattern, add)
    {
        // (What the project file counts as used now is no longer said.)
        let rules = app.ws.as_ref().map(|ws| ws.module.project_file.used.clone());
        let rules = rules.unwrap_or_default();
        let selected = w.selected.and_then(|i| w.results.get(i).cloned());
        w.results.retain(|f| {
            f.unused.is_none_or(|u| rules.marking(u.key.resref, u.key.restype).is_none())
        });
        w.selected = selected.and_then(|s| w.results.iter().position(|f| *f == s));
        if add {
            w.mark_draft.clear();
        }
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
    let options = build.then(|| {
        let mut options = w.clone();
        options.external = app.external_compiler();
        options.debug = app.settings.debug_info;
        options
    });
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
                    None => vec![Finding { text: NO_GAME.into(), about: None, unused: None }],
                };
                if let Some(w) = &mut app.build {
                    w.results = results;
                    w.selected = None;
                }
            },
        );
    }
}

/// Why Mark as Used can't be: the module has no folder.
const NO_PLACE: &str = "Save the module first: the project file is kept beside it";

const MARK_HOVER: &str = "Count it as used from now on (for a script ExecuteScript runs by a name \
    made as it runs): its name goes in the project file's [used], which Aurora leaves alone";

/// The `[used]` keys, and what Counted as Used calls each.
const USED_KINDS: [(&str, &str); 3] =
    [("scripts", "Script"), ("conversations", "Conversation"), ("blueprints", "Blueprint")];

/// Counted as Used: the names and globs the project file's `[used]`
/// counts as used, each with Remove, and a field to add one. The change
/// asked for: the key, the name or glob, added or taken out.
fn marks(
    ui: &mut egui::Ui,
    w: &mut BuildWindow,
    used: &UsedRules,
    place: Option<&mg_module::project_file::Place>,
) -> Option<(&'static str, String, bool)> {
    let mut change = None;
    let Some(place) = place else {
        ui.weak(NO_PLACE);
        return None;
    };
    ui.weak(format!(
        "Kept in {}'s [used]. Without being named here, a resource counts as used where a \
         field names it, a script includes it, a string in a script spells its name \
         (ExecuteScript(\"x\"), CreateObject), a hak's conversation or blueprint the module \
         reaches names it, or, for a script, one of the game's 2DAs runs it (spells, areas \
         of effect, states, traps, diseases, poisons). A name made as a script runs is not \
         seen: name it here, with * for any letters (ai_*).",
        place.team_name()
    ))
    .on_hover_text(place.team().display().to_string());
    let mut any = false;
    for (key, label) in USED_KINDS {
        let Some(p) = used.of(key) else { continue };
        for glob in &p.globs {
            any = true;
            ui.horizontal(|ui| {
                ui.label(format!("{label}: {glob}"));
                if ui.small_button("Remove").on_hover_text("No longer count it as used").clicked() {
                    change = Some((key, glob.clone(), false));
                }
            });
        }
    }
    if !any {
        ui.weak("Nothing is named yet.");
    }
    ui.horizontal(|ui| {
        let label =
            |key: &&'static str| USED_KINDS.iter().find(|(k, _)| k == key).map_or("", |k| k.1);
        let keys: Vec<&'static str> = USED_KINDS.iter().map(|(k, _)| *k).collect();
        crate::widgets::dropdown("build-used-kind").selected_text(label(&w.mark_kind)).show(
            ui,
            &mut w.mark_kind,
            &keys,
            label,
        );
        let r = ui.add(
            egui::TextEdit::singleline(&mut w.mark_draft)
                .hint_text("name or glob (ai_*)")
                .desired_width(160.0),
        );
        let problem = (!w.mark_draft.trim().is_empty())
            .then(|| mg_module::project_file::Patterns::new(&[w.mark_draft.trim()]).err())
            .flatten();
        let can = !w.mark_draft.trim().is_empty() && problem.is_none();
        let add = ui.add_enabled(can, egui::Button::new("Add"));
        let add = match &problem {
            Some(p) => add.on_disabled_hover_text(p),
            None => add.on_hover_text("Count what it names as used"),
        };
        let typed = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if can && (add.clicked() || typed) {
            change = Some((w.mark_kind, w.mark_draft.trim().to_string(), true));
        }
    });
    change
}

/// Writes a change to the project file's `[used]` (making the file where
/// there is none) and reads the project's files again; whether it was
/// written.
fn set_used(app: &mut Moonglow, kind: &'static str, pattern: &str, add: bool) -> bool {
    use mg_module::project_file::ProjectFile;
    let Some(ws) = app.ws.as_mut() else { return false };
    let Some(place) = ws.module.location.as_ref().and_then(ProjectFile::place_to_write) else {
        app.log.warn(NO_PLACE);
        return false;
    };
    if let Err(e) = ProjectFile::set_used(&place, kind, pattern, add) {
        app.log.error(format!("{}: [used] was not written: {e}", place.team_name()));
        return false;
    }
    let had = ws.module.project_file.found;
    if ws.module.reread_project_file() {
        app.log_project_file(true, had);
    }
    app.references.look_again();
    let what = USED_KINDS.iter().find(|(k, _)| *k == kind).map_or(kind, |k| k.1);
    app.log.info(if add {
        format!("{what} {pattern} counts as used from now on ({}'s [used])", place.team_name())
    } else {
        format!("{what} {pattern} no longer counts as used ({}'s [used])", place.team_name())
    });
    true
}

/// Options › General › Build module on save: the build with Aurora's
/// defaults before saving; the Build Module window opens with the results
/// if it found problems.
pub(crate) fn build_on_save(app: &mut Moonglow) {
    let was_open = app.build.is_some();
    let mut w = app.build.take().unwrap_or_default();
    let external = app.external_compiler();
    let debug = app.settings.debug_info;
    let results = run(app, &BuildWindow { external, debug, ..Default::default() });
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
    mg_module::palette::BlueprintKind::from_restype(t).is_some()
}

/// What a build says without the game's data (as an editor's tab does).
const NO_GAME: &str =
    "No game data: choose your Neverwinter Nights installation in Tools > Options.";

/// Runs the build and waits for it (the build before saving): the compile
/// passes, as one undoable command, then the checks. The results are the
/// problems found ("No errors found" if none, as in Aurora); what each pass
/// did goes to the log.
fn run(app: &mut Moonglow, w: &BuildWindow) -> Vec<Finding> {
    app.refresh_module_layer();
    let none = |text: &str| vec![Finding { text: text.into(), about: None, unused: None }];
    let Some(game) = app.game.as_deref() else { return none(NO_GAME) };
    let Some(ws) = app.ws.as_mut() else { return none("No module open") };
    let built = match ws.snapshot() {
        Ok(module) => work(&module, game, w),
        Err(e) => return vec![Finding { text: e.to_string(), about: None, unused: None }],
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
    let line = |text: String, about: Option<ResKey>| Finding { text, about, unused: None };
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
    let line = |text: String, about: Option<ResKey>| Finding { text, about, unused: None };
    let mut staged = module.clone();
    if w.compile {
        if w.compile_scripts {
            let results = mg_module::build::compile_scripts_with(
                &mut staged,
                &game.resman,
                mg_module::build::ScriptSelection::All,
                w.external.as_ref(),
                w.debug,
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
            let item =
                |r: ResRef| mg_module::gff_root(Some(module), game, &ResKey::new(r, ResType::UTI));
            let n = mg_module::build::compile_creature_cr(&mut staged, game, &item);
            notes.push(format!("Build: {n} creature challenge ratings brought up to date"));
        }
        if w.compile_encounters {
            // The creatures as they are now, ratings included.
            let snapshot = staged.clone();
            let read = |r: ResRef| {
                mg_module::gff_root(Some(&snapshot), game, &ResKey::new(r, ResType::UTC))
            };
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
    let line = |text: String, about: Option<ResKey>| Finding { text, about, unused: None };
    // (Every GFF of the module read once for both checks.)
    let references = if w.missing || w.unused {
        mg_module::verify::module_references(module)
    } else {
        Vec::new()
    };
    // (The scripts the game's tables run, as the module sees them, count
    // as used, and what the module reaches in its haks and the game names.)
    let unused = w.unused.then(|| {
        let also = mg_module::verify::also_used(module, &references, &game.resman);
        mg_module::verify::unused_of(module, &references, &also)
    });
    if w.missing {
        let missing = mg_module::verify::missing_of(module, &game.resman, references);
        let mut n = 0;
        for m in missing.iter().filter(|m| w.missing_of.get(&m.category).copied().unwrap_or(true)) {
            n += 1;
            let what = format!("is {}", m.what());
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
    if let Some(unused) = unused {
        let unused: Vec<mg_module::verify::Unused> = unused
            .into_iter()
            .filter(|u| match u.key.restype {
                ResType::NSS | ResType::NCS => w.unused_scripts,
                ResType::DLG => w.unused_conversations,
                t => is_blueprint(t) && w.unused_blueprints,
            })
            .collect();
        for u in &unused {
            out.push(Finding {
                text: format!("Unused: {u}"),
                about: Some(u.key),
                unused: Some(*u),
            });
        }
        notes.push(format!("Build: {} unused resources", unused.len()));
    }
}
