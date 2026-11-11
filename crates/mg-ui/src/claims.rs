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
use mg_module::claims::{Access, Change, Standing, Whose};
use mg_module::project_file::{MAX_CODE, ProjectFile, code_problem};
use mg_resman::ResKey;

use crate::{Action, Moonglow};

/// What is asked about claims and protected names.
#[derive(Debug, Default)]
pub(crate) struct ClaimsState {
    /// The Save window.
    pub(crate) save_ask: Option<SaveAsk>,
    /// Save Anyway was chosen: the next save is not asked about.
    pub(crate) cleared: bool,
    /// The Protected Area window.
    pub(crate) protected_ask: Option<ProtectedAsk>,
    /// The changes the builder said to make anyway (those the window
    /// named): let through once.
    let_through: Vec<Pass>,
    /// A command was held just now (for the window): Apply says no more.
    pub(crate) held: bool,
    /// The Claims Table window (Edit › Claims Table…).
    pub(crate) table: Option<TableWindow>,
    /// The Claims Table window's listing, kept while the claims and the
    /// module's areas are what it was made by ([`listing`]).
    listed: Option<(ClaimsNow, Vec<ResRef>, std::sync::Arc<Vec<mg_module::claims::Listed>>)>,
}

/// The Claims Table window: what is being answered in it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TableWindow {
    /// An area being handed on, and the code typed for whom.
    hand: Option<(ResRef, String)>,
    /// Another's area about to be taken over (asked first).
    take: Option<ResRef>,
}

/// What a builder does to an area's claim, in the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum How {
    Claim,
    Release,
    /// Another builder's claim, taken (`mg claims claim --force`).
    TakeOver,
    /// This builder's claim, handed to the builder of that code.
    Hand(String),
}

/// A save held: the areas not this builder's that it would write, each
/// with why, as the project's claims had it then, and what Save Anyway
/// goes on with.
#[derive(Debug, Clone)]
pub(crate) struct SaveAsk {
    areas: Vec<(ResRef, String)>,
    then: Action,
    /// The claims the list was made by (the table, the rules and the
    /// builder's code): read again (a pull, with reloading on), the list
    /// is made again.
    by: ClaimsNow,
}

/// What says whose each area is.
type ClaimsNow = (Option<mg_module::claims::Claims>, mg_module::claims::ClaimRules, Option<String>);

fn whose(p: &ProjectFile) -> ClaimsNow {
    (p.claim_table.clone(), p.claims.clone(), p.user.builder_code.clone())
}

/// A change to protected areas, held until the builder answers.
#[derive(Debug, Clone)]
pub(crate) struct ProtectedAsk {
    /// Every protected area it would change (one command may change
    /// several: areas chosen together and deleted, a field set on all an
    /// Advanced Search found), each with why it is protected and what of
    /// it would change. Never empty.
    pub changes: Vec<(ResRef, String, Protected)>,
    /// What goes ahead if they are changed anyway.
    pub then: Action,
}

/// A protected area's change the builder said to make anyway: that
/// change of that area, once, while what it went on with is under way
/// (its Rename or Delete window open, or its action waiting to run).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pass {
    area: ResRef,
    what: Protected,
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

/// Why an area is read-only here, where it is: another builder holds it,
/// in a project that keeps others' areas read-only. Its view's tools are
/// off and its forms gray ([`gate`] refuses what gets past them).
pub(crate) fn read_only(app: &Moonglow, area: ResRef) -> Option<String> {
    match access(app, area) {
        Access::ReadOnly(why) => Some(why),
        _ => None,
    }
}

/// [`crate::widgets::variables_button`] of a form editing `key`: off
/// where it is a read-only area's (its variables aren't changed).
pub(crate) fn variables_button(
    app: &Moonglow,
    ui: &mut egui::Ui,
    key: &ResKey,
    count: usize,
) -> bool {
    let Some(why) = area_of(key).and_then(|a| read_only(app, a)) else {
        return crate::widgets::variables_button(ui, count);
    };
    let r = ui.add_enabled_ui(false, |ui| crate::widgets::variables_button(ui, count));
    r.response.on_disabled_hover_text(format!("{why}: read-only here"));
    false
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

/// What a command changes of protected areas, each one it changes: its
/// Tag, its ResRef (a rename: the area goes, another comes), or the area
/// itself (deleted). All of them: the first alone was asked about, and
/// its answer let the others through unnamed.
fn protected_changes(app: &mut Moonglow, cmd: &Command) -> Vec<(ResRef, String, Protected)> {
    // (The protected areas' edits, each with why.)
    let candidates: Vec<(&Edit, String)> = {
        let Some(p) = project(app) else { return Vec::new() };
        let protected = |e: &Edit| {
            let key = e.key();
            (key.restype == ResType::ARE).then(|| p.protected(key.resref)).flatten()
        };
        cmd.edits.iter().filter_map(|e| protected(e).map(|why| (e, why))).collect()
    };
    let mut out: Vec<(ResRef, String, Protected)> = Vec::new();
    for (e, why) in candidates {
        let area = e.key().resref;
        // (An area gone or renamed is said once, not its Tag too.)
        if out.iter().any(|(a, _, w)| *a == area && *w != Protected::Tag) {
            continue;
        }
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
                out.retain(|(a, ..)| *a != area);
                out.push((area, why, what));
                continue;
            }
            Edit::SetResource { data: Some(bytes), .. } => {
                mg_gff::Gff::read(bytes).ok().and_then(|g| g.root.string("Tag").map(<[u8]>::to_vec))
            }
            _ => continue,
        };
        // (A Tag set to what it is: no change.)
        if new_tag != tag_now(app, area) && !out.iter().any(|(a, ..)| *a == area) {
            out.push((area, why, Protected::Tag));
        }
    }
    out
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
    let changes = protected_changes(app, cmd);
    let Some(&(area, _, what)) = changes.first() else { return Ok(()) };
    // (Let through only where the builder was shown every one of them.)
    let passed = |app: &Moonglow, (area, _, what): &(ResRef, String, Protected)| {
        app.claims.let_through.contains(&Pass { area: *area, what: *what })
    };
    if changes.iter().all(|c| passed(app, c)) {
        app.claims.let_through.clear();
        return Ok(());
    }
    app.claims.protected_ask = Some(ProtectedAsk { changes, then: Action::Apply(cmd.clone()) });
    app.claims.held = true;
    Err(EditError::Invalid {
        key: ResKey::new(area, ResType::ARE),
        message: format!("not changed yet: {}'s {} is kept: asked first", area, what.noun()),
    })
}

impl Moonglow {
    /// [`Moonglow::apply`] for a caller that goes on once its command is
    /// applied: `Ok(true)` when it was; `Ok(false)` when it was held for
    /// the Protected Area window (asked first: the caller stops without a
    /// word, and nothing is to be said of an error); an error otherwise.
    /// `then`: what goes ahead if the builder says to change it anyway,
    /// where the caller has an action of its own for it (its follow-ups
    /// happen then too); none: the command alone. A caller that asked
    /// plain `apply` and said the error left the mark of a held command
    /// set, and the next command that failed for another reason failed
    /// without a word.
    pub(crate) fn apply_or_ask(
        &mut self,
        cmd: Command,
        then: Option<Action>,
    ) -> Result<bool, EditError> {
        match self.apply(cmd) {
            Ok(()) => Ok(true),
            Err(_) if std::mem::take(&mut self.claims.held) => {
                if let (Some(ask), Some(then)) = (&mut self.claims.protected_ask, then) {
                    ask.then = then;
                }
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }
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
    if key.restype != ResType::ARE
        || app.claims.let_through.contains(&Pass { area: key.resref, what })
    {
        return false;
    }
    let Some(why) = project(app).and_then(|p| p.protected(key.resref)) else { return false };
    let changes = vec![(key.resref, why, what)];
    app.claims.protected_ask = Some(ProtectedAsk { changes, then: action.clone() });
    true
}

/// Before a save: the areas not this builder's that it would write (in a
/// project that warns of them), asked about first. Whether the save is
/// held (`then` goes on once the builder says to save anyway).
pub(crate) fn hold_save(app: &mut Moonglow, then: Action) -> bool {
    if std::mem::take(&mut app.claims.cleared) || !kept(app) {
        return false;
    }
    let asked = others_changed(app);
    if asked.is_empty() {
        return false;
    }
    let by = project(app).map(whose).unwrap_or_default();
    app.claims.save_ask = Some(SaveAsk { areas: asked, then, by });
    true
}

/// The areas not this builder's (in a project that warns of them) that a
/// save would write a change to, each with why.
fn others_changed(app: &mut Moonglow) -> Vec<(ResRef, String)> {
    let Some(ws) = app.ws.as_mut() else { return Vec::new() };
    if ws.flush().is_err() {
        return Vec::new();
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
    areas
        .into_iter()
        .filter_map(|a| match m.project_file.access(a) {
            Access::Warn(why) if changed(a) => Some((a, why)),
            _ => None,
        })
        .collect()
}

/// Claims an area (or releases this builder's claim of it) in the
/// project's table, and reads the project's files again.
pub(crate) fn claim(app: &mut Moonglow, area: ResRef, release: bool) {
    change(app, area, if release { How::Release } else { How::Claim });
}

/// Changes an area's row of the project's table as the builder said
/// (Claim, Release, Take Over, Hand To), and reads the project's files
/// again.
pub(crate) fn change(app: &mut Moonglow, area: ResRef, how: How) {
    let Some(p) = project(app) else { return };
    let Some(path) = p.claims_path() else {
        app.log.warn("The project keeps no claims: its moonglow.toml names no [claims] file");
        return;
    };
    let Some(code) = p.user.builder_code.clone() else {
        app.log.warn("A claim is made by your builder code: set it first (Edit › Builder Code…)");
        return;
    };
    let holder = p.standing(area).and_then(|s| s.claim()).and_then(|c| c.builder.clone());
    let file = p.claims.file.clone().unwrap_or_default();
    let (change, force, done, not) = match &how {
        How::Claim => (Change::Claim(&code), false, format!("claimed by {code}"), "claimed"),
        How::Release => (Change::Release(&code), false, format!("released by {code}"), "released"),
        How::TakeOver => {
            let from = holder.as_deref().unwrap_or("no one");
            (Change::Claim(&code), true, format!("taken over from {from} by {code}"), "taken over")
        }
        How::Hand(to) => (
            Change::Hand { from: &code, to },
            false,
            format!("handed to {to} by {code}"),
            "handed on",
        ),
    };
    match mg_module::claims::write_change(&path, area, change, force) {
        Ok(()) => {
            let tell = match how {
                How::TakeOver | How::Hand(_) => "commit it, and tell them",
                _ => "commit it, so the others see it",
            };
            app.log.info(format!("{area} {done}, in {file}: {tell}"));
            reread(app);
        }
        Err(e) => app.log.error(format!("{area} was not {not}: {e}")),
    }
}

/// Opens the Claims Table window (Edit › Claims Table…).
pub(crate) fn open_table(app: &mut Moonglow) {
    if kept(app) {
        app.claims.table = Some(TableWindow::default());
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
            Some((crate::icons::CLAIMED_MINE, c.builder.clone().unwrap_or_default())),
            format!("Yours: claimed by {}", c.holder().unwrap_or_default()),
        ),
        Standing::Others(c) => (
            Some((crate::icons::CLAIMED, c.builder.clone().unwrap_or_default())),
            format!("Claimed by {}", c.holder().unwrap_or_default()),
        ),
        Standing::Free(_) if merged.is_some() => (None, "Claimed by no one".into()),
        Standing::Free(_) => return,
    };
    let mut tip = tip;
    if let Some(why) = &merged {
        tip.push_str(&format!(
            "\n{why}: its Tag and ResRef are kept (players' positions and first visits)"
        ));
    }
    let color = match standing {
        Standing::Others(_) => ui.visuals().warn_fg_color,
        _ => ui.visuals().weak_text_color(),
    };
    // (The holder's icon and builder code, then a pin where it is merged:
    // the code names it for a screen reader.)
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = ui.spacing().icon_spacing;
        if let Some((icon, builder)) = mark {
            let text = egui::RichText::new(builder).color(color);
            mg_uikit::icon_label(ui, icon, Some(color), text).on_hover_text(&tip);
        }
        if merged.is_some() {
            let (rect, r) = ui.allocate_exact_size(egui::Vec2::splat(16.0), egui::Sense::hover());
            crate::icons::MERGED.paint(ui.painter(), rect, color);
            r.on_hover_text(&tip);
        }
    });
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
        (Access::ReadOnly(why), _) => Some((
            Some(crate::icons::CLAIMED),
            format!("{why}: read-only here. Ask them to release it, then claim it."),
        )),
        (Access::Warn(why), Some(Standing::Others(_))) => Some((
            Some(crate::icons::CLAIMED),
            format!("{why}: a save asks before it writes your changes to it."),
        )),
        (Access::Warn(why), _) => Some((
            None,
            format!(
                "{why}: a save asks before it writes your changes to it. Claim it to build it."
            ),
        )),
        (Access::Edit, _) => None,
    };
    if text.is_none() && protected.is_none() {
        return;
    }
    let free = matches!(standing, Some(Standing::Free(_)));
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            if let Some((icon, text)) = &text {
                let warn = ui.visuals().warn_fg_color;
                match icon {
                    Some(icon) => {
                        let text = egui::RichText::new(text).color(warn);
                        mg_uikit::icon_label(ui, *icon, Some(warn), text);
                    }
                    None => {
                        ui.colored_label(warn, text);
                    }
                }
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
                let weak = ui.visuals().weak_text_color();
                let text = format!(
                    "{why}: its Tag and ResRef are kept (players' positions and first visits go \
                     by them)."
                );
                mg_uikit::icon_label(
                    ui,
                    crate::icons::MERGED,
                    Some(weak),
                    egui::RichText::new(text).color(weak),
                );
            }
        });
    });
}

/// The claims' windows: the save of others' areas, a protected area's
/// change.
pub(crate) fn windows(app: &mut Moonglow, ctx: &egui::Context) {
    end_pass(app);
    table_window(app, ctx);
    save_window(app, ctx);
    protected_window(app, ctx);
}

/// The Claims Table window: every area of the table and the module, whose
/// each is, and what this builder may do with each (Claim, Release, Hand
/// To…, Take Over…), as `mg claims` lists and changes them.
/// Every area of the claims table and the module, whose each is
/// (`ProjectFile::claims_listing`): made when the claims (the table, the
/// rules, the builder's code) or the module's areas change, and kept
/// between. It was made anew every frame the window showed, each area
/// looked for in the table and the module's list: eight million
/// comparisons a frame for a world of 1,400 areas.
fn listing(app: &mut Moonglow) -> Option<std::sync::Arc<Vec<mg_module::claims::Listed>>> {
    let p = project(app).filter(|p| p.claim_table.is_some() || p.claims.file.is_some())?;
    let areas: Vec<ResRef> = (app.ws.as_ref())
        .map(|ws| ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect())
        .unwrap_or_default();
    let by = whose(p);
    if let Some((was, of, listed)) = &app.claims.listed
        && *was == by
        && *of == areas
    {
        return Some(listed.clone());
    }
    let listed = std::sync::Arc::new(p.claims_listing(Some(&areas)));
    app.claims.listed = Some((by, areas, listed.clone()));
    Some(listed)
}

fn table_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut w) = app.claims.table.take() else {
        app.claims.listed = None;
        return;
    };
    let Some(listed) = listing(app) else { return };
    let Some(p) = project(app) else { return };
    let file = p.claims.file.clone().unwrap_or_default();
    let code = p.user.builder_code.clone();
    let mut open = true;
    let mut close = false;
    let mut chosen: Option<(ResRef, How)> = None;
    crate::widgets::dialog(ctx, "Claims Table", &mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_max_width(640.0);
            match &code {
                Some(c) => ui.label(format!("{file}: whose each area is. Your code is {c}.")),
                None => ui.label(format!(
                    "{file}: whose each area is. Set your builder code to claim one (Edit › \
                     Builder Code…)."
                )),
            };
            ui.add_space(4.0);
            // (The rows in sight alone are laid out: a world has 1,400
            // areas. Columns of set widths, a long holder's name cut and
            // whole on hover.)
            let height = ui.spacing().interact_size.y;
            let rows = egui::ScrollArea::vertical().max_height(360.0).auto_shrink([false, true]);
            mg_uikit::show_rows(rows, ui, listed.len(), |ui, range| {
                ui.set_min_width(600.0);
                let cell = |ui: &mut egui::Ui, width: f32, text: egui::RichText| {
                    let layout = egui::Layout::left_to_right(egui::Align::Center);
                    ui.allocate_ui_with_layout(egui::vec2(width, height), layout, |ui| {
                        ui.set_min_width(width);
                        let said = text.text().to_string();
                        ui.add(egui::Label::new(text).truncate()).on_hover_text(said);
                    });
                };
                for at in range {
                    let l = &listed[at];
                    ui.horizontal(|ui| {
                        if at % 2 == 1 {
                            let mut stripe = ui.available_rect_before_wrap();
                            stripe.set_height(height);
                            ui.painter().rect_filled(stripe, 0.0, ui.visuals().faint_bg_color);
                        }
                        let c = l.claim.as_ref();
                        cell(ui, 130.0, egui::RichText::new(l.area.to_string()));
                        let whose = match (l.whose, c.and_then(|c| c.holder())) {
                            (Whose::Mine, Some(h)) => format!("yours: {h}"),
                            (_, Some(h)) => h,
                            (_, None) => "free".to_string(),
                        };
                        let whose = match l.whose {
                            Whose::Others => {
                                egui::RichText::new(whose).color(ui.visuals().warn_fg_color)
                            }
                            _ => egui::RichText::new(whose),
                        };
                        cell(ui, 190.0, whose);
                        let mut notes = Vec::new();
                        if c.is_some_and(|c| c.merged()) {
                            notes.push("merged");
                        } else if l.protected {
                            notes.push("protected");
                        }
                        if !l.in_module {
                            notes.push("not in the module");
                        }
                        cell(ui, 140.0, egui::RichText::new(notes.join(", ")).weak());
                        match l.whose {
                            Whose::Mine => {
                                if ui.button("Release").clicked() {
                                    chosen = Some((l.area, How::Release));
                                }
                                if ui
                                    .button("Hand To…")
                                    .on_hover_text("Give your claim to another builder")
                                    .clicked()
                                {
                                    w.hand = Some((l.area, String::new()));
                                    w.take = None;
                                }
                            }
                            Whose::Others => {
                                let r =
                                    ui.add_enabled(code.is_some(), egui::Button::new("Take Over…"));
                                let r = r
                                    .on_hover_text("Claim it in their place (asked first)")
                                    .on_disabled_hover_text(
                                        "Set your builder code first (Edit › Builder Code…)",
                                    );
                                if r.clicked() {
                                    w.take = Some(l.area);
                                    w.hand = None;
                                }
                            }
                            Whose::Free => {
                                let r = ui.add_enabled(code.is_some(), egui::Button::new("Claim"));
                                let r = r.on_disabled_hover_text(
                                    "Set your builder code first (Edit › Builder Code…)",
                                );
                                if r.clicked() {
                                    chosen = Some((l.area, How::Claim));
                                }
                            }
                        }
                    });
                }
            });
            if let Some(area) = w.take {
                let who = listed
                    .iter()
                    .find(|l| l.area == area)
                    .and_then(|l| l.claim.as_ref()?.holder())
                    .unwrap_or_default();
                ui.separator();
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    format!(
                        "{area} is claimed by {who}. Taken over, their changes to it not yet \
                         merged are the conflict version control can't merge: ask them first."
                    ),
                );
                ui.horizontal(|ui| {
                    if ui.button("Take It Over").clicked() {
                        chosen = Some((area, How::TakeOver));
                        w.take = None;
                    }
                    if ui.button("Don't Take It").clicked() {
                        w.take = None;
                    }
                });
            }
            if let Some((area, to)) = &mut w.hand {
                ui.separator();
                let area = *area;
                let mut give = false;
                ui.horizontal(|ui| {
                    crate::widgets::field_label(ui, format!("Hand {area} to"));
                    let field = ui.add(
                        mg_uikit::text_field(to)
                            .char_limit(MAX_CODE + 4)
                            .desired_width(100.0)
                            .hint_text("their code"),
                    );
                    crate::widgets::autofocus(ui, &field);
                });
                let typed = to.trim().to_ascii_lowercase();
                let problem = if typed.is_empty() {
                    Some(String::new())
                } else if Some(&typed) == code.as_ref() {
                    Some("That is your own code.".to_string())
                } else {
                    code_problem(&typed)
                };
                if let Some(p) = problem.as_ref().filter(|p| !p.is_empty()) {
                    ui.colored_label(ui.visuals().warn_fg_color, p);
                }
                let mut back = false;
                ui.horizontal(|ui| {
                    give =
                        ui.add_enabled(problem.is_none(), egui::Button::new("Hand Over")).clicked();
                    back = ui.button("Keep It").clicked();
                });
                if give {
                    chosen = Some((area, How::Hand(typed)));
                }
                if give || back {
                    w.hand = None;
                }
            }
            ui.separator();
            ui.weak("The table is a file of the checkout: commit it, so the others see it.");
            close = ui.button("Close").clicked();
        });
    if let Some((area, how)) = chosen {
        change(app, area, how);
    }
    if open && !close {
        app.claims.table = Some(w);
    }
}

/// A pass the builder gave lasts while what it went on with is under
/// way: a Rename or Delete window of the area closed without the change
/// (or never opened), or an action that ran without making it, takes it
/// away, so that the area's next change is asked about again.
fn end_pass(app: &mut Moonglow) {
    if app.claims.let_through.is_empty() {
        return;
    }
    let open = app.claims.let_through.iter().any(|pass| match pass.what {
        Protected::ResRef => app
            .rename
            .as_ref()
            .is_some_and(|d| d.from.restype == ResType::ARE && d.from.resref == pass.area),
        Protected::Deleted => {
            let asked = |k: &ResKey| k.restype == ResType::ARE && k.resref == pass.area;
            app.confirm_delete.as_ref().is_some_and(|keys| keys.iter().any(asked))
        }
        Protected::Tag => false,
    });
    if !open && app.actions.is_empty() {
        app.claims.let_through.clear();
    }
}

fn save_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(ask) = app.claims.save_ask.take() else { return };
    // The claims read again while it waits (a pull): the list made again,
    // and the save gone on with where none is left.
    let now = project(app).map(whose).unwrap_or_default();
    let ask = if now == ask.by {
        ask
    } else {
        let areas = if kept(app) { others_changed(app) } else { Vec::new() };
        if areas.is_empty() {
            app.log.info("The claims table changed: the save writes no area not yours now");
            app.actions.push(ask.then);
            return;
        }
        SaveAsk { areas, by: now, ..ask }
    };
    app.claims.save_ask = Some(ask.clone());
    let SaveAsk { areas, then, .. } = ask;
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
            let doing = |what: Protected| match what {
                Protected::Tag => "Changing its Tag",
                Protected::ResRef => "Renaming it",
                Protected::Deleted => "Deleting it",
            };
            match ask.changes.as_slice() {
                [(_, why, what)] => {
                    ui.label(format!("{why}."));
                    ui.add_space(4.0);
                    ui.label(format!(
                        "{} loses what the game keeps of it: a character's position is kept by \
                         the area's Tag, and its first visits by the area's ResRef. Players who \
                         were there come back somewhere else, and see it as new.",
                        doing(*what)
                    ));
                }
                // (Every one named: the answer is for them all.)
                several => {
                    ui.label(format!("This changes {} protected areas:", several.len()));
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        for (_, why, what) in several {
                            let done = match what {
                                Protected::Tag => "its Tag would change",
                                Protected::ResRef => "it would be renamed",
                                Protected::Deleted => "it would be deleted",
                            };
                            ui.label(format!("• {why}: {done}"));
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(
                        "That loses what the game keeps of them: a character's position is \
                         kept by the area's Tag, and its first visits by the area's ResRef. \
                         Players who were there come back somewhere else, and see them as new.",
                    );
                }
            }
            ui.add_space(4.0);
            let one = ask.changes.len() == 1;
            ui.weak(match one {
                true => "A copy of it, under a new ResRef and Tag, is a new area and loses nothing.",
                false => "A copy of an area, under a new ResRef and Tag, is a new area and loses nothing.",
            });
            ui.separator();
            ui.horizontal(|ui| {
                if one {
                    copy = ui.button("Make a Copy Instead…").clicked();
                }
                let first = ask.changes[0].2;
                let same = ask.changes.iter().all(|c| c.2 == first);
                let label = match (first, one) {
                    (Protected::Tag, true) => "Change It Anyway",
                    (Protected::ResRef, true) => "Rename It Anyway",
                    (Protected::Deleted, true) => "Delete It Anyway",
                    (Protected::ResRef, false) if same => "Rename Them Anyway",
                    (Protected::Deleted, false) if same => "Delete Them Anyway",
                    (_, false) => "Change Them Anyway",
                };
                anyway = ui.button(label).clicked();
                cancel = crate::widgets::cancel(ui);
            });
        });
    if anyway {
        app.claims.protected_ask = None;
        app.claims.let_through =
            ask.changes.iter().map(|(area, _, what)| Pass { area: *area, what: *what }).collect();
        app.actions.push(ask.then);
    } else if copy {
        app.claims.protected_ask = None;
        app.actions.push(Action::CopyDialog(ResKey::new(ask.changes[0].0, ResType::ARE)));
    } else if cancel || !open {
        app.claims.protected_ask = None;
    }
}
