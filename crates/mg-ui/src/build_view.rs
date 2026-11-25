//! Build › Build Module (Aurora's `TdlgVerifyModule`): compile passes
//! (scripts, creature challenge ratings, encounters, palettes), then checks
//! for missing resources and, if asked, unused ones. Aurora's defaults:
//! Compile and Missing Resources on, Unused and Spell Check off.
//!
//! M4's U28, by the owner's Q4(a): the dialog keeps its options and Build,
//! Mark as Used, Export… and Done; its results go to Problems as a source
//! of their own ([`crate::problems::Source::Build`]), which Build opens (a
//! click on a result's place, or a double click on its row, opens what it is
//! about), and Counted as Used is the Build heading's button there. Mark as
//! Used counts the Unused result chosen in Problems as used.

use mg_uikit::Controls as _;
use std::collections::BTreeMap;

use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_module::project_file::UsedRules;
use mg_module::verify::Category;
use mg_resman::ResKey;

use crate::Moonglow;
use crate::dialogs::FileKind;

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
    /// The line as the log says it ("Error: …", "Unused: …").
    pub text: String,
    pub about: Option<ResKey>,
    /// Of an Unused line, what is unused (Mark as Used takes it).
    pub unused: Option<mg_module::verify::Unused>,
    /// Its kind in Problems: a script that does not compile and a missing
    /// resource that breaks the game are errors; an unused one, and one
    /// missing that the game does without, warnings.
    pub severity: mg_module::doctor::Severity,
    /// Its words in Problems, after its place (the line without "Error: "
    /// or "Unused: ").
    pub message: String,
}

impl Finding {
    fn error(text: String, about: Option<ResKey>) -> Finding {
        let message = text.strip_prefix("Error: ").unwrap_or(&text).to_string();
        Finding { text, about, unused: None, severity: Severity::Error, message }
    }

    /// As Problems lists it: its place the resource it is about.
    fn problem(&self) -> crate::problems::Problem {
        let at = self.about.map(|k| k.to_string()).unwrap_or_default();
        let go = self.about.map(crate::problems::Go::Resource);
        let mut p = crate::problems::Problem::new(self.severity, at, self.message.clone(), go);
        p.unused = self.unused;
        p
    }
}

use mg_module::doctor::Severity;

/// Counted as Used's field: the name or glob typed, and the key of
/// `[used]` it goes in (`scripts`, `conversations`, `blueprints`).
#[derive(Debug, Clone, PartialEq)]
pub struct UsedDraft {
    pub text: String,
    pub kind: &'static str,
}

impl Default for UsedDraft {
    fn default() -> UsedDraft {
        UsedDraft { text: String::new(), kind: "scripts" }
    }
}

/// The Build Module window: its options (its results are Problems').
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
            external: None,
            debug: false,
        }
    }
}

/// Where Mark as Used writes (the project file beside the module), where
/// the module has a folder.
pub(crate) fn used_place(app: &Moonglow) -> Option<mg_module::project_file::Place> {
    (app.ws.as_ref())
        .and_then(|ws| ws.module.location.as_ref())
        .and_then(mg_module::project_file::ProjectFile::place_to_write)
}

/// The Unused result of Build's chosen in Problems, if one is.
pub(crate) fn chosen_unused(app: &Moonglow) -> Option<mg_module::verify::Unused> {
    let p = app.problems.selected()?;
    let build = app.problems.of(&crate::problems::Source::Build)?;
    build.problems.iter().any(|x| std::ptr::eq(x, p)).then_some(p.unused).flatten()
}

/// Build Module's results said in its footer: where they are.
pub(crate) fn results_note(app: &Moonglow) -> Option<String> {
    let set = app.problems.of(&crate::problems::Source::Build)?;
    let (errors, warnings) = set.counts();
    let words = crate::problems::counted(errors, warnings)
        .unwrap_or_else(|| crate::problems::NOTHING.to_string());
    Some(format!("{words}, in Problems"))
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut w) = app.build.take() else { return };
    let mut open = true;
    let (mut build, mut done, mut export) = (false, false, false);
    let place = used_place(app);
    let chosen = chosen_unused(app);
    let note = results_note(app);
    let can_export =
        app.problems.of(&crate::problems::Source::Build).is_some_and(|s| !s.problems.is_empty());
    let footed = crate::widgets::dialog(ctx, "Build Module", &mut open)
        .collapsible(false)
        .default_width(560.0)
        .show_foot(ctx, |ui| {
            // (Wide enough for its footer's note, "5 errors · 9 warnings,
            // in Problems", beside the four buttons, where the window has
            // the room.)
            ui.set_min_width(540.0_f32.min(ctx.content_rect().width() - 48.0));
            // The four checks, always; Advanced Controls shows what each
            // covers.
            ui.horizontal(|ui| {
                ui.check_box(&mut w.unused, "Unused").on_hover_text("Check for use");
                ui.check_box(&mut w.missing, "Missing Resources")
                    .on_hover_text("Check that resources are available");
                ui.check_box(&mut w.compile, "Compile");
                ui.add_enabled(false, mg_uikit::Checkbox::new(&mut false, "Spell Check"))
                    .on_disabled_hover_text("Not yet: Moonglow has no dictionary");
            });
            ui.check_box(&mut w.advanced, "Advanced Controls");
            if w.advanced {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.add_enabled_ui(w.unused, |ui| {
                            crate::widgets::field_label(ui, "Unused");
                            ui.check_box(&mut w.unused_scripts, "Scripts");
                            ui.check_box(&mut w.unused_conversations, "Conversations");
                            ui.check_box(&mut w.unused_blueprints, "Blueprints");
                        });
                    });
                    // (A gap, not a divider: a divider in a row takes all
                    // the height there is and the window grew to the
                    // screen's, its buttons out of reach.)
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.add_enabled_ui(w.missing, |ui| {
                            crate::widgets::field_label(ui, "Missing Resources");
                            // (Two columns: eleven kinds in one would make
                            // the dialog taller than its other columns.)
                            let half = w.missing_of.len().div_ceil(2);
                            ui.horizontal_top(|ui| {
                                // (8 apart, the Dialogs board's `.two-c`
                                // column gap: U30's #10.)
                                ui.spacing_mut().item_spacing.x = MISSING_GAP;
                                let mut kinds = w.missing_of.iter_mut().enumerate();
                                for column in 0..2 {
                                    ui.vertical(|ui| {
                                        for (i, (c, on)) in kinds.by_ref() {
                                            ui.check_box(on, format!("{c:?}"));
                                            if column == 0 && i + 1 == half {
                                                break;
                                            }
                                        }
                                    });
                                }
                            });
                        });
                    });
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.add_enabled_ui(w.compile, |ui| {
                            crate::widgets::field_label(ui, "Compile");
                            ui.check_box(&mut w.compile_scripts, "Scripts");
                            ui.check_box(&mut w.compile_cr, "Creature CR")
                                .on_hover_text("Recalculate creature challenge ratings");
                            ui.check_box(&mut w.compile_encounters, "Encounters");
                            ui.check_box(&mut w.compile_palettes, "Palettes");
                        });
                    });
                });
            }
            let can = chosen.is_some() && place.is_some();
            let mark_tip = if place.is_none() {
                NO_PLACE
            } else if chosen.is_none() {
                CHOOSE_UNUSED
            } else {
                MARK_HOVER
            };
            let mut foot = crate::widgets::Foot::new();
            if let Some(note) = &note {
                foot = foot.note(note.clone());
            }
            let foot = foot
                .main("Build", true)
                .button(crate::problems::MARK_AS_USED, can)
                .tip(mark_tip)
                .button("Export…", can_export)
                .leave("Done");
            ((), foot)
        });
    if let Some(f) = footed {
        build = f.pressed.main();
        done = f.pressed.cancelled();
        export = f.pressed.clicked("Export…");
        if f.pressed.clicked(crate::problems::MARK_AS_USED)
            && let Some(u) = chosen
        {
            mark_used(app, u);
        }
    }
    if export
        && let Some(path) =
            app.dialogs.save_file(FileKind::Any, Some(std::path::Path::new("build.txt")))
    {
        let text = export_text(app);
        if let Err(e) = std::fs::write(&path, text) {
            app.log.error(format!("{}: {e}", path.display()));
        }
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
    // The build, in the background; its results go to Problems when it is
    // done, brought to the front with the Bottom Dock open.
    if let Some(options) = options {
        let unused = options.unused;
        app.start_job(
            "Build Module",
            move |job| job.game.as_deref().map(|game| work(&job.module, game, &options)),
            move |app, built| {
                let results = match built {
                    Some(built) => finish(app, built),
                    None => vec![Finding::error(NO_GAME.into(), None)],
                };
                show_results(app, &results, true, unused);
            },
        );
    }
}

/// A build's results into Problems in place of the last build's, of the
/// module as it is now (its own edits made): brought to the front, the
/// Bottom Dock opened where `open` or anything was found.
fn show_results(app: &mut Moonglow, results: &[Finding], open: bool, unused: bool) {
    let problems: Vec<_> = results.iter().map(Finding::problem).collect();
    let found = !problems.is_empty();
    let checked = app.ws.as_ref().map(|ws| ws.revision());
    app.problems.replace(crate::problems::Source::Build, problems, checked);
    app.problems.build_unused = unused;
    app.problems.ask_forward(open || found);
    // (Its command's: not a build on save, which leaves the keys be.)
    app.problems.take_keys = found && open;
}

/// Export…: Build's results in Problems as text, a line each ("Error:
/// m1q1.nss: …").
pub(crate) fn export_text(app: &Moonglow) -> String {
    let Some(set) = app.problems.of(&crate::problems::Source::Build) else { return String::new() };
    (set.problems.iter())
        .map(|p| {
            let word = if p.severity == Severity::Error { "Error" } else { "Warning" };
            match p.at.is_empty() {
                true => format!("{word}: {}\n", p.message),
                false => format!("{word}: {}: {}\n", p.at, p.message),
            }
        })
        .collect()
}

/// Mark as Used (the Build dialog's button, an Unused result's menu in
/// Problems): `u` counted as used from now on in the project file's
/// `[used]`, and the Unused results it covers now taken from Problems.
pub(crate) fn mark_used(app: &mut Moonglow, u: mg_module::verify::Unused) {
    let Some(kind) = UsedRules::key_for(u.key.restype) else { return };
    change_used(app, kind, &u.key.resref.to_string(), true);
}

/// A change to `[used]` written; Build's Unused results it now covers
/// taken from Problems.
fn change_used(app: &mut Moonglow, kind: &'static str, pattern: &str, add: bool) -> bool {
    if !set_used(app, kind, pattern, add) {
        return false;
    }
    // (What the project file counts as used now is no longer said.)
    let rules = app.ws.as_ref().map(|ws| ws.module.project_file.used.clone());
    let rules = rules.unwrap_or_default();
    app.problems.retain(&crate::problems::Source::Build, |p| {
        p.unused.is_none_or(|u| rules.marking(u.key.resref, u.key.restype).is_none())
    });
    true
}

/// Build's Counted as Used in Problems: under its button, what the
/// project file counts as used, each with Remove, and a field to add a
/// name or glob.
pub(crate) fn counted_as_used(app: &mut Moonglow, button: &egui::Response) {
    let place = used_place(app);
    let used = app.ws.as_ref().map(|ws| ws.module.project_file.used.clone()).unwrap_or_default();
    let mut draft = std::mem::take(&mut app.build_used);
    let mut change = None;
    egui::Popup::menu(button)
        .id(egui::Id::new("build-counted-as-used"))
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .width(380.0)
        .show(|ui| {
            ui.set_max_width(380.0);
            change = marks(ui, &mut draft, &used, place.as_ref());
        });
    app.build_used = draft;
    if let Some((kind, pattern, add)) = change
        && change_used(app, kind, &pattern, add)
        && add
    {
        app.build_used.text.clear();
    }
}

/// The room between Missing Resources' two columns of kinds (the
/// Dialogs board's `.two-c`: `column-gap: 8px`).
const MISSING_GAP: f32 = 8.0;

/// Why Mark as Used can't be: no Unused result is chosen.
const CHOOSE_UNUSED: &str = "Choose an Unused result in Problems first";

/// Why Mark as Used can't be: the module has no folder.
pub(crate) const NO_PLACE: &str = "Save the module first: the project file is kept beside it";

pub(crate) const MARK_HOVER: &str =
    "Count it as used from now on (kept in the project file's [used])";

/// The `[used]` keys, and what Counted as Used calls each.
const USED_KINDS: [(&str, &str); 3] =
    [("scripts", "Script"), ("conversations", "Conversation"), ("blueprints", "Blueprint")];

/// Counted as Used: the names and globs the project file's `[used]`
/// counts as used, each with Remove, and a field to add one. The change
/// asked for: the key, the name or glob, added or taken out.
fn marks(
    ui: &mut egui::Ui,
    w: &mut UsedDraft,
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
    mg_uikit::with_rows(ui, |ui| {
        for (key, label) in USED_KINDS {
            let Some(p) = used.of(key) else { continue };
            for glob in &p.globs {
                any = true;
                ui.horizontal(|ui| {
                    ui.label(format!("{label}: {glob}"));
                    let tip = "No longer count it as used";
                    if ui.small_button("Remove").on_hover_text(tip).clicked() {
                        change = Some((key, glob.clone(), false));
                    }
                });
            }
        }
    });
    if !any {
        ui.weak("Nothing is named yet.");
    }
    ui.horizontal(|ui| {
        let label =
            |key: &&'static str| USED_KINDS.iter().find(|(k, _)| k == key).map_or("", |k| k.1);
        let keys: Vec<&'static str> = USED_KINDS.iter().map(|(k, _)| *k).collect();
        crate::widgets::dropdown("build-used-kind").selected_text(label(&w.kind)).show(
            ui,
            &mut w.kind,
            &keys,
            label,
        );
        let r = ui.add(
            mg_uikit::text_field(&mut w.text).hint_text("name or glob (ai_*)").desired_width(160.0),
        );
        let problem = (!w.text.trim().is_empty())
            .then(|| mg_module::project_file::Patterns::new(&[w.text.trim()]).err())
            .flatten();
        let can = !w.text.trim().is_empty() && problem.is_none();
        let add = ui.add_enabled(can, egui::Button::new("Add"));
        let add = match &problem {
            Some(p) => add.on_disabled_hover_text(p),
            None => add.on_hover_text("Count what it names as used"),
        };
        let typed = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if can && (add.clicked() || typed) {
            change = Some((w.kind, w.text.trim().to_string(), true));
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
/// defaults before saving; its results go to Problems, brought to the
/// front with the Bottom Dock open if it found problems.
pub(crate) fn build_on_save(app: &mut Moonglow) {
    let external = app.external_compiler();
    let debug = app.settings.debug_info;
    let results = run(app, &BuildWindow { external, debug, ..Default::default() });
    show_results(app, &results, false, false);
}

/// A blueprint type (the Unused › Blueprints check).
fn is_blueprint(t: ResType) -> bool {
    mg_module::palette::BlueprintKind::from_restype(t).is_some()
}

/// What a build says without the game's data (as an editor's tab does).
const NO_GAME: &str =
    "No game data: choose your Neverwinter Nights installation in Tools › Options.";

/// Runs the build and waits for it (the build before saving): the compile
/// passes, as one undoable command, then the checks. The results are the
/// problems found (none: the log says "No errors found", as Aurora's
/// results did); what each pass did goes to the log.
fn run(app: &mut Moonglow, w: &BuildWindow) -> Vec<Finding> {
    app.refresh_module_layer();
    let none = |text: &str| vec![Finding::error(text.into(), None)];
    let Some(game) = app.game.as_deref() else { return none(NO_GAME) };
    let Some(ws) = app.ws.as_mut() else { return none("No module open") };
    let built = match ws.snapshot() {
        Ok(module) => work(&module, game, w),
        Err(e) => return none(&e.to_string()),
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
    if !edits.is_empty()
        && let Err(e) = app.apply(Command::new("Build Module", edits))
    {
        out.push(Finding::error(format!("Error: {e}"), None));
    }
    app.log.info("Building Module…");
    for n in notes {
        app.log.info(n);
    }
    if out.is_empty() {
        app.log.info("No errors found");
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
    let line = Finding::error;
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
            let utcs: std::collections::HashMap<ResRef, std::sync::Arc<[u8]>> = staged
                .keys_of(ResType::UTC)
                .filter_map(|k| staged.shared(k).map(|b| (k.resref, b.clone())))
                .collect();
            let read = |r: ResRef| {
                if let Some(bytes) = utcs.get(&r) {
                    return mg_gff::Gff::read(bytes).ok().map(|g| g.root);
                }
                mg_module::gff_root(None, game, &ResKey::new(r, ResType::UTC))
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
    // (Every GFF of the module read once for both checks.)
    let references = if w.missing || w.unused {
        mg_module::verify::module_references(module)
    } else {
        Vec::new()
    };
    // (The scripts the game's tables run, as the module sees them, count
    // as used, and what the module reaches in its haks and the game names.)
    let unused = w.unused.then(|| mg_module::verify::unused_in(module, &references, &game.resman));
    if w.missing {
        let missing = mg_module::verify::missing_of(module, &game.resman, references);
        let mut n = 0;
        for m in missing.iter().filter(|m| w.missing_of.get(&m.category).copied().unwrap_or(true)) {
            n += 1;
            let what = format!("is {}", m.what());
            let text = format!(
                "{:?}: {:?} {} ({} {}) {what}",
                m.category,
                m.reference.kind,
                m.reference.target,
                m.reference.from,
                m.reference.path
            );
            let severity = if m.is_error() { Severity::Error } else { Severity::Warning };
            let message = text.clone();
            out.push(Finding {
                text,
                about: Some(m.reference.from),
                unused: None,
                severity,
                message,
            });
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
                severity: Severity::Warning,
                // (Its place is its name: what is unused of it after it.)
                message: match u.script {
                    Some(s) => format!("Unused ({})", s.words()),
                    None => "Unused".to_string(),
                },
            });
        }
        notes.push(format!("Build: {} unused resources", unused.len()));
    }
}
