//! The plugins' events that run at once, within what the user does (plugin
//! API 0.4): a resource's own events around a save, and what a plugin does
//! when objects are placed or pasted, or a wizard (or Add to Palette,
//! Setup Store, a copy) makes a resource.
//!
//! - **Before a save** ([`hold_save`]): each enabled plugin's
//!   `resource-saving` handlers read every resource the save would write,
//!   as it was and as it will be. What they say is shown before anything is
//!   written: a warning is asked about (Save Anyway, Cancel), a refusal
//!   stops the save, each with the fix the plugin offers. A handler never
//!   edits there: the user saves what they see. A plugin that fails or runs
//!   out of time is a warning ("could not check"), never a refusal, so a
//!   broken plugin can't keep a module from being saved.
//! - **What the user made** ([`with_plugins`]): the handlers run on a copy
//!   of the module with the creation made (core's naming and the project's
//!   rules first: the builder code, Setup Store's script), and their edits
//!   join the creation's own command, so that one Undo takes both back.
//!   What a plugin's own command makes fires nothing.

use std::cell::RefCell;
use std::rc::Rc;

use mg_edit::fix::Fix;
use mg_edit::{Command, Workspace};
use mg_module::ModuleLocation;
use mg_plugin::{
    EVENT_BUDGET, Event, EventKind, Host, Input, Plugin, PluginError, Question, Selected, run_event,
};
use mg_resman::ResKey;

use crate::{Action, Level, Moonglow};

/// What the user just made, for the plugins' handlers: the resources (an
/// area by its ARE alone, a script by its source) and the objects placed
/// or pasted, and what made them (`ctx.event.by`).
#[derive(Debug, Clone, PartialEq)]
pub struct Made {
    /// `"wizard"`, `"add-to-palette"`, `"setup-store"`, `"copy"`,
    /// `"palette"`, `"gallery"`, `"paste"`.
    pub by: &'static str,
    pub resources: Vec<ResKey>,
    pub objects: Vec<Selected>,
    /// The objects were pasted (else placed).
    pub pasted: bool,
}

impl Made {
    /// Resources made by `by`.
    pub fn resources(by: &'static str, resources: Vec<ResKey>) -> Made {
        Made { by, resources, objects: Vec::new(), pasted: false }
    }

    /// Objects placed (or `pasted`) by `by`.
    pub fn objects(by: &'static str, objects: Vec<Selected>, pasted: bool) -> Made {
        Made { by, resources: Vec::new(), objects, pasted }
    }

    /// The events it is, each with what happened.
    fn events(&self) -> Vec<Event> {
        let mut out = Vec::new();
        if !self.resources.is_empty() {
            let mut e = Event::new(EventKind::ResourceCreated, self.resources.clone());
            e.by = self.by.to_string();
            out.push(e);
        }
        if !self.objects.is_empty() {
            let kind = if self.pasted { EventKind::ObjectPasted } else { EventKind::ObjectPlaced };
            let mut e = Event::new(kind, Vec::new());
            e.objects = self.objects.clone();
            e.by = self.by.to_string();
            out.push(e);
        }
        out
    }
}

/// What a plugin said before a save: a warning or a refusal of a resource
/// (none: of the save, as a plugin that could not check), with its fix.
#[derive(Debug, Clone, PartialEq)]
pub struct Heard {
    pub plugin: String,
    pub resource: Option<ResKey>,
    pub text: String,
    pub refused: bool,
    pub fix: Option<Fix>,
}

/// A save held for what the plugins said: what they said, and the action
/// that goes on when the user saves anyway.
#[derive(Debug, Clone, PartialEq)]
pub struct Held {
    pub heard: Vec<Heard>,
    pub then: Action,
}

/// The state of the save's question.
#[derive(Debug, Default)]
pub struct SaveState {
    pub held: Option<Held>,
    /// Save Anyway: the next save is not held again.
    cleared: bool,
}

/// A host for a handler that runs at once: nobody is asked (a question is
/// unanswered), and what it logs is kept for the window's log.
#[derive(Default)]
struct AtOnce {
    log: RefCell<Vec<(mg_plugin::Level, String)>>,
}

impl Host for AtOnce {
    fn log(&self, level: mg_plugin::Level, text: &str) {
        self.log.borrow_mut().push((level, text.to_string()));
    }

    fn ask(&self, _: &Question) -> Option<mg_plugin::Answer> {
        None
    }
}

/// The enabled plugins with a handler of an event of `kind`, each with its
/// settings: the plugin, the handler's id, its settings.
fn handlers(app: &Moonglow, kind: EventKind) -> Vec<(Plugin, String, mg_plugin::Settings)> {
    let mut out = Vec::new();
    for p in app.enabled_plugins() {
        for e in p.manifest.events.iter().filter(|e| e.on == kind) {
            out.push((p.clone(), e.id.clone(), app.plugin_settings(p)));
        }
    }
    out
}

/// Runs a handler at once, on `module`: what it made, with what it logged
/// put into the window's log.
fn run_at_once(
    app: &mut Moonglow,
    plugin: &Plugin,
    id: &str,
    event: &Event,
    module: mg_module::Module,
    settings: mg_plugin::Settings,
) -> Result<mg_plugin::Outcome, PluginError> {
    let host = Rc::new(AtOnce::default());
    let input = Input {
        settings: Some(settings),
        budget: Some(EVENT_BUDGET),
        ..Input::new(module, app.game.clone())
    };
    let made = run_event(plugin, id, event, input, host.clone());
    let name = &plugin.manifest.name;
    for (l, text) in host.log.take() {
        app.log.said(crate::plugins::level(l), format!("{name}: {text}"));
    }
    made
}

/// `cmd`, which makes what `made` says, with the edits the enabled
/// plugins' handlers make of it: run on a copy of the module with `cmd`
/// made, each plugin seeing the edits of those before it. One that fails
/// is said, and left out. Without handlers for it, `cmd` as it is.
pub(crate) fn with_plugins(app: &mut Moonglow, cmd: Command, made: &Made) -> Command {
    let events = made.events();
    let wanted: Vec<(Plugin, String, mg_plugin::Settings, Event)> = events
        .iter()
        .flat_map(|e| {
            handlers(app, e.kind).into_iter().map(move |(p, id, s)| (p, id, s, e.clone()))
        })
        // (Only those for what was made: else nothing need be worked out.)
        .filter(|(p, id, _, e)| {
            let Some(decl) = p.manifest.events.iter().find(|d| d.id == *id) else { return false };
            e.resources.iter().any(|k| decl.is_for(k))
                || e.objects.iter().any(|o| match o {
                    Selected::Object { kind, .. } => decl.is_for_kind(*kind),
                    _ => false,
                })
        })
        .collect();
    if wanted.is_empty() {
        return cmd;
    }
    let Some(ws) = app.ws.as_mut() else { return cmd };
    if ws.flush().is_err() {
        return cmd;
    }
    let mut scratch = Workspace::new(ws.module.clone());
    // (One that does not apply is said when it is applied.)
    if scratch.apply(cmd.clone()).is_err() {
        return cmd;
    }
    let mut edits = cmd.edits.clone();
    for (plugin, id, settings, event) in wanted {
        if scratch.flush().is_err() {
            break;
        }
        let module = scratch.module.clone();
        let name = plugin.manifest.name.clone();
        match run_at_once(app, &plugin, &id, &event, module, settings) {
            Ok(outcome) if outcome.edits.is_empty() => {}
            Ok(outcome) => {
                let n = touched(&outcome.edits);
                match scratch.apply(Command::new("plugin", outcome.edits.clone())) {
                    Ok(()) => {
                        edits.extend(outcome.edits);
                        app.log.info(format!(
                            "{name}: {} changed {n} resource{} with {} (one Undo takes both \
                             back)",
                            event.kind.name(),
                            if n == 1 { "" } else { "s" },
                            cmd.label
                        ));
                    }
                    Err(e) => app.log.error(format!(
                        "{name}: {}: its edits were not made ({e}); {} is made without them",
                        event.kind.name(),
                        cmd.label
                    )),
                }
            }
            Err(e) => app.log.error(format!("{e}; {} is made without it", cmd.label)),
        }
    }
    Command::new(cmd.label, edits)
}

/// How many resources edits change.
fn touched(edits: &[mg_edit::Edit]) -> usize {
    let mut keys: Vec<&ResKey> = edits.iter().map(mg_edit::Edit::key).collect();
    keys.sort();
    keys.dedup();
    keys.len()
}

/// Applies `cmd`, which makes what `made` says, with the plugins' edits
/// of it, as one command.
pub(crate) fn apply_made(app: &mut Moonglow, cmd: Command, made: &Made) {
    let cmd = with_plugins(app, cmd, made);
    // (One held for an answer says so in its window.)
    if let Err(e) = app.apply(cmd)
        && !std::mem::take(&mut app.claims.held)
    {
        app.log.error(e.to_string());
    }
}

/// The module as a save would write it (the script editors' unsaved text
/// in it), and what the save writes: each resource that is not as its file
/// was (a module kept in one file: as it was when last saved or opened),
/// with what it was.
fn saving(app: &mut Moonglow, kind: EventKind) -> Option<(mg_module::Module, Event)> {
    let ws = app.ws.as_mut()?;
    ws.flush().ok()?;
    let mut module = ws.module.clone();
    for (key, buf) in app.scripts.iter().filter(|(_, b)| b.is_dirty()) {
        module.set(*key, buf.bytes());
    }
    let saved = app.plugins.saved.as_ref();
    let archive = matches!(module.location, Some(ModuleLocation::Archive(_)) | None);
    let disk = &ws.module;
    let was = |k: &ResKey| match archive {
        true => saved.and_then(|s| s.get(k)),
        false => disk.as_read(k),
    };
    let gone: Vec<ResKey> = saved.iter().flat_map(|s| s.keys().copied()).collect();
    let event = Event::saving(kind, &module, was, gone);
    Some((module, event))
}

/// Before a save: what the enabled plugins' `resource-saving` handlers say
/// of what it would write. Whether the save is held (`then` goes on once
/// the user saves anyway): for a refusal, or a warning.
pub(crate) fn hold_save(app: &mut Moonglow, then: Action) -> bool {
    if std::mem::take(&mut app.plugins.save.cleared) {
        return false;
    }
    let wanted = handlers(app, EventKind::ResourceSaving);
    if wanted.is_empty() {
        return false;
    }
    let Some((module, event)) = saving(app, EventKind::ResourceSaving) else { return false };
    if event.resources.is_empty() {
        return false;
    }
    let mut heard = Vec::new();
    for (plugin, id, settings) in wanted {
        let name = plugin.manifest.name.clone();
        match run_at_once(app, &plugin, &id, &event, module.clone(), settings) {
            Ok(outcome) => heard.extend(outcome.said.into_iter().map(|s| Heard {
                plugin: name.clone(),
                resource: Some(s.resource),
                text: s.text,
                refused: s.refused,
                fix: s.fix,
            })),
            // (A plugin that can't check refuses nothing.)
            Err(e) => heard.push(Heard {
                plugin: name.clone(),
                resource: None,
                text: format!("could not check the save: {}", why(&e)),
                refused: false,
                fix: None,
            }),
        }
    }
    if heard.is_empty() {
        return false;
    }
    // Refusals first.
    heard.sort_by_key(|h| !h.refused);
    app.plugins.save.held = Some(Held { heard, then });
    true
}

/// What went wrong with a plugin, without its name (the window says it).
fn why(e: &PluginError) -> String {
    match e {
        PluginError::Script { message, .. } => message.clone(),
        other => other.to_string(),
    }
}

/// After a save: the enabled plugins' `resource-saved` handlers, as jobs
/// (as `module-saved`'s), with what the save wrote and each as it was:
/// `was`, the module as it was last saved or opened.
pub(crate) fn queue_saved(app: &mut Moonglow, was: Option<&mg_module::Module>) {
    let wanted = handlers(app, EventKind::ResourceSaved);
    let Some(ws) = app.ws.as_ref() else { return };
    if wanted.is_empty() {
        return;
    }
    let module = &ws.module;
    let gone: Vec<ResKey> = was.iter().flat_map(|s| s.keys().copied()).collect();
    let event = Event::saving(EventKind::ResourceSaved, module, |k| was?.get(k), gone);
    if event.resources.is_empty() {
        return;
    }
    for (plugin, id, _) in wanted {
        app.queue_event_of(&plugin.manifest.id, &id, event.clone());
    }
}

/// The window of a held save: what the plugins said, each with its fix;
/// Save Anyway (not for a refusal) and Cancel.
pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(held) = app.plugins.save.held.clone() else { return };
    let refused = held.heard.iter().any(|h| h.refused);
    let (mut anyway, mut close, mut fixed) = (false, false, None);
    let modal = egui::Modal::new(egui::Id::new("plugin-save")).show(ctx, |ui| {
        ui.set_width(540.0);
        ui.heading(if refused { "Not saved" } else { "Before saving" });
        ui.add_space(4.0);
        ui.label(if refused {
            "A plugin refuses what the save would write. Nothing was written; change it (or \
             make the fix offered), then save again."
        } else {
            "The plugins have warnings about what the save would write. Nothing is written yet."
        });
        ui.add_space(6.0);
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            for (i, h) in held.heard.iter().enumerate() {
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    let (word, color) = match h.refused {
                        true => ("Refused", ui.visuals().error_fg_color),
                        false => ("Warning", ui.visuals().warn_fg_color),
                    };
                    ui.colored_label(color, egui::RichText::new(word).strong());
                    ui.label(egui::RichText::new(&h.plugin).strong());
                    if let Some(r) = &h.resource {
                        ui.weak(r.to_string());
                    }
                });
                ui.label(&h.text);
                if let Some(fix) = &h.fix
                    && ui
                        .button(&fix.label)
                        .on_hover_text(
                            "Make this change (Edit › Undo takes it back), and save again",
                        )
                        .clicked()
                {
                    fixed = Some(i);
                }
            }
        });
        ui.separator();
        ui.weak(
            "Plugins check a save as their manifests declare (Plugins › Manage Plugins lists \
             what each does, and disables one).",
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if !refused {
                anyway =
                    ui.button("Save Anyway").on_hover_text("Write the module as it is").clicked();
            }
            close = ui.button(if refused { "Close" } else { "Cancel" }).clicked();
        });
    });
    if let Some(i) = fixed {
        let fix = held.heard[i].fix.clone().expect("a fix was clicked");
        app.plugins.save.held = None;
        match app.apply(fix.command()) {
            Ok(()) => {
                app.log.info(format!("{}: fixed: {}", held.heard[i].plugin, fix.label));
                // (Asked again: the plugins read the module as it now is.)
                app.actions.push(held.then);
            }
            Err(e) => app.log.error(format!("{}: {}: {e}", held.heard[i].plugin, fix.label)),
        }
    } else if anyway {
        app.plugins.save.held = None;
        app.plugins.save.cleared = true;
        // (The claims were asked before the plugins, and the module is as
        // it was then: not asked again.)
        app.claims.cleared = true;
        app.actions.push(held.then);
    } else if close || modal.should_close() {
        app.plugins.save.held = None;
        app.log.said(
            if refused { Level::Warning } else { Level::Info },
            match refused {
                true => "Not saved: a plugin refused it, for what follows".to_string(),
                false => "Not saved: canceled, after the plugins' warnings that follow".to_string(),
            },
        );
        for h in &held.heard {
            let at = h.resource.map(|r| format!("{r}: ")).unwrap_or_default();
            app.log.said(
                if h.refused { Level::Error } else { Level::Warning },
                format!("{}: {at}{}", h.plugin, h.text),
            );
        }
    }
}
