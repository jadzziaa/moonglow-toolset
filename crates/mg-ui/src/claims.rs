//! Claimed areas and protected names (The Domains of Dread's builder
//! proposal, C6 and C7), in the window.
//!
//! A project whose `moonglow.toml` names a claims table (`[claims] file`)
//! says whose each area is: the module tree marks the claimed ones, an
//! area another builder holds says so above its view, and is either kept
//! from changes (`others = "read-only"`) or asked about before a save
//! writes a change to it (`"warn"`, the default). Claim This Area and
//! Release Claim, in an area's menus, edit the table (a file of the
//! checkout, which the builder commits).
//!
//! An area the table says is merged (in the game), or that `[claims]
//! protected` names, keeps its Tag and ResRef: the game keeps players'
//! positions by the one and their first visits by the other. A change to
//! either (Properties, Rename, Delete, or anything else that edits it) is
//! asked about first, with Make a Copy offered instead.
//!
//! Every change to the module goes through [`Moonglow::apply`], which asks
//! [`gate`] first: so no editor can pass either by.

use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, EditError, GffPath};
use mg_module::claims::{Access, Change, Standing};
use mg_module::project_file::ProjectFile;
use mg_resman::ResKey;

use crate::{Action, Moonglow};

/// What is asked about claims and protected names.
#[derive(Debug, Default)]
pub(crate) struct ClaimsState {
    /// The Save window: the areas not this builder's that a save would
    /// write, each with why, and what Save Anyway goes on with.
    pub(crate) save_ask: Option<(Vec<(ResRef, String)>, Action)>,
    /// Save Anyway was chosen: the next save is not asked about.
    pub(crate) cleared: bool,
    /// The Protected Area window.
    pub(crate) protected_ask: Option<ProtectedAsk>,
    /// An area the builder said to change anyway: let through once.
    let_through: Option<ResRef>,
    /// A command was held just now (for the window): Apply says no more.
    pub(crate) held: bool,
}

/// A change to a protected area, held until the builder answers.
#[derive(Debug, Clone)]
pub(crate) struct ProtectedAsk {
    pub area: ResRef,
    /// Why it is protected.
    pub why: String,
    pub what: Protected,
    /// What goes ahead if it is changed anyway.
    pub then: Action,
}

/// What would change of a protected area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Protected {
    Tag,
    ResRef,
    Deleted,
}

/// The area a resource is part of (its `.are`, `.git` or `.gic`).
pub(crate) fn area_of(key: &ResKey) -> Option<ResRef> {
    matches!(key.restype, ResType::ARE | ResType::GIT | ResType::GIC).then_some(key.resref)
}

pub(crate) fn project(app: &Moonglow) -> Option<&ProjectFile> {
    of(&app.ws)
}

/// The open module's project file (for a caller that holds other parts
/// of the application).
pub(crate) fn of(ws: &Option<mg_edit::Workspace>) -> Option<&ProjectFile> {
    ws.as_ref().map(|ws| &ws.module.project_file)
}

/// What this builder may do with an area.
pub(crate) fn access(app: &Moonglow, area: ResRef) -> Access {
    project(app).map_or(Access::Edit, |p| p.access(area))
}

/// Whether the project keeps claims (names a table).
pub(crate) fn kept(app: &Moonglow) -> bool {
    project(app).is_some_and(|p| p.claim_table.is_some())
}

/// The area's Tag as the module has it now.
fn tag_now(app: &mut Moonglow, area: ResRef) -> Option<Vec<u8>> {
    let ws = app.ws.as_mut()?;
    let doc = ws.doc(&ResKey::new(area, ResType::ARE)).ok()?;
    doc.root.string("Tag").map(<[u8]>::to_vec)
}

/// What a command changes of a protected area, if it changes any: its
/// Tag, its ResRef (a rename: the area goes, another comes), or the area
/// itself (deleted).
fn protected_change(app: &mut Moonglow, cmd: &Command) -> Option<(ResRef, String, Protected)> {
    // (The protected areas' edits, each with why.)
    let candidates: Vec<(&Edit, String)> = {
        let p = project(app)?;
        let protected = |e: &Edit| {
            let key = e.key();
            (key.restype == ResType::ARE).then(|| p.protected(key.resref)).flatten()
        };
        cmd.edits.iter().filter_map(|e| protected(e).map(|why| (e, why))).collect()
    };
    for (e, why) in candidates {
        let area = e.key().resref;
        let new_tag = match e {
            Edit::SetField { path, label, value, .. }
                if *path == GffPath::root() && label == "Tag" =>
            {
                match value {
                    Some(mg_gff::Value::String(s)) => Some(s.clone()),
                    _ => None,
                }
            }
            Edit::SetResource { data: None, .. } => {
                // (Another area made by the same command: a rename.)
                let renamed = cmd.edits.iter().any(|o| {
                    matches!(o, Edit::SetResource { key: k, data: Some(_) }
                        if k.restype == ResType::ARE && k.resref != area)
                });
                let what = if renamed { Protected::ResRef } else { Protected::Deleted };
                return Some((area, why, what));
            }
            Edit::SetResource { data: Some(bytes), .. } => {
                mg_gff::Gff::read(bytes).ok().and_then(|g| g.root.string("Tag").map(<[u8]>::to_vec))
            }
            _ => continue,
        };
        // (A Tag set to what it is: no change.)
        if new_tag != tag_now(app, area) {
            return Some((area, why, Protected::Tag));
        }
    }
    None
}

/// Whether a command may change the module, as the project's claims
/// have it: one that changes an area another builder holds, in a project
/// that keeps those read-only, is refused; one that changes a protected
/// area's Tag or ResRef is held, and the builder asked (the Protected
/// Area window).
pub(crate) fn gate(app: &mut Moonglow, cmd: &Command) -> Result<(), EditError> {
    if app.ws.is_none() {
        return Ok(());
    }
    for e in &cmd.edits {
        let Some(area) = area_of(e.key()) else { continue };
        if let Access::ReadOnly(why) = access(app, area) {
            return Err(EditError::Invalid {
                key: *e.key(),
                message: format!(
                    "not changed: {why}, and the project keeps others' areas read-only (Claim \
                     This Area, once it is released, to edit it)"
                ),
            });
        }
    }
    if let Some((area, why, what)) = protected_change(app, cmd) {
        if app.claims.let_through == Some(area) {
            app.claims.let_through = None;
            return Ok(());
        }
        app.claims.protected_ask =
            Some(ProtectedAsk { area, why, what, then: Action::Apply(cmd.clone()) });
        app.claims.held = true;
        return Err(EditError::Invalid {
            key: ResKey::new(area, ResType::ARE),
            message: format!("not changed yet: {}'s {} is kept: asked first", area, what.noun()),
        });
    }
    Ok(())
}

impl Protected {
    fn noun(self) -> &'static str {
        match self {
            Protected::Tag => "Tag",
            Protected::ResRef | Protected::Deleted => "ResRef",
        }
    }
}

/// Before an action that would change a protected area's ResRef (the
/// Rename window, deleting it): asks first. Whether it was held.
pub(crate) fn asks_first(app: &mut Moonglow, action: &Action) -> bool {
    let (key, what) = match action {
        Action::RenameDialog(k) => (*k, Protected::ResRef),
        Action::DeleteDialog(k) => (*k, Protected::Deleted),
        _ => return false,
    };
    let key = crate::references::main_key(key);
    if key.restype != ResType::ARE || app.claims.let_through == Some(key.resref) {
        return false;
    }
    let Some(why) = project(app).and_then(|p| p.protected(key.resref)) else { return false };
    app.claims.protected_ask =
        Some(ProtectedAsk { area: key.resref, why, what, then: action.clone() });
    true
}

/// Before a save: the areas not this builder's that it would write (in a
/// project that warns of them), asked about first. Whether the save is
/// held (`then` goes on once the builder says to save anyway).
pub(crate) fn hold_save(app: &mut Moonglow, then: Action) -> bool {
    if std::mem::take(&mut app.claims.cleared) || !kept(app) {
        return false;
    }
    let Some(ws) = app.ws.as_mut() else { return false };
    if ws.flush().is_err() {
        return false;
    }
    let m = &ws.module;
    let mut areas: Vec<ResRef> = m
        .keys()
        .filter_map(area_of)
        .chain(m.project_file.claim_table.iter().flat_map(|t| t.rows.iter().map(|c| c.area)))
        .collect();
    areas.sort();
    areas.dedup();
    // (The module's bytes and the file's as read are usually the same
    // shared bytes: compared at once, not byte by byte, for every area a
    // large world's builder hasn't claimed.)
    let same = |a: Option<&[u8]>, b: Option<&[u8]>| match (a, b) {
        (Some(a), Some(b)) => std::ptr::eq(a, b) || a == b,
        (None, None) => true,
        _ => false,
    };
    let changed = |area: ResRef| {
        [ResType::ARE, ResType::GIT, ResType::GIC].iter().any(|t| {
            let k = ResKey::new(area, *t);
            !same(m.get(&k), m.as_read(&k))
        })
    };
    let asked: Vec<(ResRef, String)> = areas
        .into_iter()
        .filter_map(|a| match m.project_file.access(a) {
            Access::Warn(why) if changed(a) => Some((a, why)),
            _ => None,
        })
        .collect();
    if asked.is_empty() {
        return false;
    }
    app.claims.save_ask = Some((asked, then));
    true
}

/// Claims an area (or releases this builder's claim of it) in the
/// project's table, and reads the project's files again.
pub(crate) fn claim(app: &mut Moonglow, area: ResRef, release: bool) {
    let Some(p) = project(app) else { return };
    let Some(path) = p.claims_path() else {
        app.log.warn("The project keeps no claims: its moonglow.toml names no [claims] file");
        return;
    };
    let Some(code) = p.user.builder_code.clone() else {
        app.log.warn("A claim is made by your builder code: set it first (Edit › Builder Code…)");
        return;
    };
    let how = if release { Change::Release(&code) } else { Change::Claim(&code) };
    match mg_module::claims::write_change(&path, area, how, false) {
        Ok(()) => {
            let file = p.claims.file.clone().unwrap_or_default();
            let done = if release { "released" } else { "claimed" };
            app.log.info(format!(
                "{area} {done} by {code}, in {file}: commit it, so the others see it"
            ));
            reread(app);
        }
        Err(e) => app
            .log
            .error(format!("{area} was not {}: {e}", if release { "released" } else { "claimed" })),
    }
}

/// The project's files read again at once (the folder's watch is a few
/// seconds late for what is shown next).
fn reread(app: &mut Moonglow) {
    if let Some(ws) = app.ws.as_mut() {
        let had = ws.module.project_file.found;
        if ws.module.reread_project_file() {
            app.log_project_file(true, had);
        }
    }
}

/// Claim This Area or Release Claim, in an area's menus (the module
/// tree's, its tab's): whichever fits whose it is. Whether one was
/// chosen.
pub(crate) fn menu_items(
    p: Option<&ProjectFile>,
    actions: &mut Vec<Action>,
    ui: &mut egui::Ui,
    area: ResRef,
) -> bool {
    let Some(p) = p else { return false };
    let Some(standing) = p.standing(area) else { return false };
    let coded = p.user.builder_code.is_some();
    let mut chosen = false;
    match standing {
        Standing::Mine(_) => {
            if ui
                .button("Release Claim")
                .on_hover_text("Let another builder claim it (the claims table, which you commit)")
                .clicked()
            {
                actions.push(Action::ReleaseClaim(area));
                chosen = true;
            }
        }
        Standing::Others(c) => {
            let who = c.holder().unwrap_or_default();
            ui.add_enabled(false, egui::Button::new("Claim This Area"))
                .on_disabled_hover_text(format!("Claimed by {who}: they release it first"));
        }
        Standing::Free(_) => {
            let r = ui.add_enabled(coded, egui::Button::new("Claim This Area"));
            let r = r
                .on_hover_text("Yours to build: the claims table says so (you commit it)")
                .on_disabled_hover_text("Set your builder code first (Edit › Builder Code…)");
            if r.clicked() {
                actions.push(Action::Claim(area));
                chosen = true;
            }
        }
    }
    chosen
}

/// The mark beside an area in the module tree: who holds it, merged.
pub(crate) fn tree_mark(p: Option<&ProjectFile>, ui: &mut egui::Ui, area: ResRef) {
    let Some(p) = p else { return };
    let Some(standing) = p.standing(area) else { return };
    let merged = p.protected(area);
    let (mark, tip) = match standing {
        Standing::Mine(c) => (
            format!("{} {}", crate::icons::CLAIMED_MINE, c.builder.as_deref().unwrap_or("")),
            format!("Yours: claimed by {}", c.holder().unwrap_or_default()),
        ),
        Standing::Others(c) => (
            format!("{} {}", crate::icons::CLAIMED, c.builder.as_deref().unwrap_or("")),
            format!("Claimed by {}", c.holder().unwrap_or_default()),
        ),
        Standing::Free(_) if merged.is_some() => (String::new(), "Claimed by no one".into()),
        Standing::Free(_) => return,
    };
    let mut text = mark;
    let mut tip = tip;
    if let Some(why) = &merged {
        text.push(' ');
        text.push_str(crate::icons::MERGED);
        tip.push_str(&format!(
            "\n{why}: its Tag and ResRef are kept (players' positions and first visits)"
        ));
    }
    let color = match standing {
        Standing::Others(_) => ui.visuals().warn_fg_color,
        _ => ui.visuals().weak_text_color(),
    };
    ui.label(egui::RichText::new(text.trim()).color(color)).on_hover_text(tip);
}

/// Above an area's view (and its Properties): whose it is, where it
/// isn't this builder's, and that its names are kept, where they are.
pub(crate) fn banner(app: &mut Moonglow, ui: &mut egui::Ui, area: ResRef) {
    let Some(p) = project(app) else { return };
    let standing = p.standing(area);
    let access = p.access(area);
    let protected = p.protected(area);
    let coded = p.user.builder_code.is_some();
    let text = match (&access, standing) {
        (Access::ReadOnly(why), _) => Some(format!(
            "{} {why}: read-only here. Ask them to release it, then claim it.",
            crate::icons::CLAIMED
        )),
        (Access::Warn(why), Some(Standing::Others(_))) => Some(format!(
            "{} {why}: a save asks before it writes your changes to it.",
            crate::icons::CLAIMED
        )),
        (Access::Warn(why), _) => Some(format!(
            "{why}: a save asks before it writes your changes to it. Claim it to build it."
        )),
        (Access::Edit, _) => None,
    };
    if text.is_none() && protected.is_none() {
        return;
    }
    let free = matches!(standing, Some(Standing::Free(_)));
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            if let Some(text) = &text {
                ui.colored_label(ui.visuals().warn_fg_color, text);
                if free {
                    let r = ui.add_enabled(coded, egui::Button::new("Claim This Area"));
                    let r = r.on_disabled_hover_text(
                        "Set your builder code first (Edit › Builder Code…)",
                    );
                    if r.clicked() {
                        app.actions.push(Action::Claim(area));
                    }
                }
            }
            if let Some(why) = &protected {
                if text.is_some() {
                    ui.end_row();
                }
                ui.weak(format!(
                    "{} {why}: its Tag and ResRef are kept (players' positions and first visits \
                     go by them).",
                    crate::icons::MERGED
                ));
            }
        });
    });
}

/// The claims' windows: the save of others' areas, a protected area's
/// change.
pub(crate) fn windows(app: &mut Moonglow, ctx: &egui::Context) {
    save_window(app, ctx);
    protected_window(app, ctx);
}

fn save_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some((areas, then)) = app.claims.save_ask.clone() else { return };
    let mut open = true;
    let (mut save, mut cancel) = (false, false);
    crate::widgets::dialog(ctx, "Areas Not Yours", &mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_max_width(460.0);
            ui.label("The save would write changes to areas you have not claimed:");
            ui.add_space(4.0);
            for (_, why) in &areas {
                ui.label(format!("  •  {why}"));
            }
            ui.add_space(4.0);
            ui.weak(
                "Two builders' changes to one area are the conflict version control can't \
                 merge. Claim an area before building in it (its menu's Claim This Area), or \
                 save anyway.",
            );
            ui.separator();
            ui.horizontal(|ui| {
                save = ui.button("Save Anyway").clicked();
                cancel = crate::widgets::cancel(ui);
            });
        });
    if save {
        app.claims.save_ask = None;
        app.claims.cleared = true;
        app.actions.push(then);
    } else if cancel || !open {
        app.claims.save_ask = None;
        app.log.info("Not saved: the areas not yours are as they were edited, unsaved");
    }
}

fn protected_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(ask) = app.claims.protected_ask.clone() else { return };
    let mut open = true;
    let (mut anyway, mut copy, mut cancel) = (false, false, false);
    crate::widgets::dialog(ctx, "Protected Area", &mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_max_width(460.0);
            let change = match ask.what {
                Protected::Tag => "Changing its Tag",
                Protected::ResRef => "Renaming it",
                Protected::Deleted => "Deleting it",
            };
            ui.label(format!("{}.", ask.why));
            ui.add_space(4.0);
            ui.label(format!(
                "{change} loses what the game keeps of it: a character's position is kept by \
                 the area's Tag, and its first visits by the area's ResRef. Players who were \
                 there come back somewhere else, and see it as new."
            ));
            ui.add_space(4.0);
            ui.weak("A copy of it, under a new ResRef and Tag, is a new area and loses nothing.");
            ui.separator();
            ui.horizontal(|ui| {
                copy = ui.button("Make a Copy Instead…").clicked();
                let label = match ask.what {
                    Protected::Tag => "Change It Anyway",
                    Protected::ResRef => "Rename It Anyway",
                    Protected::Deleted => "Delete It Anyway",
                };
                anyway = ui.button(label).clicked();
                cancel = crate::widgets::cancel(ui);
            });
        });
    if anyway {
        app.claims.protected_ask = None;
        app.claims.let_through = Some(ask.area);
        app.actions.push(ask.then);
    } else if copy {
        app.claims.protected_ask = None;
        app.actions.push(Action::CopyDialog(ResKey::new(ask.area, ResType::ARE)));
    } else if cancel || !open {
        app.claims.protected_ask = None;
    }
}
