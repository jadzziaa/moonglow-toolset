//! Bake Walkmesh to Transform… and Unbake Walkmesh (the area view's
//! Position menu), and the question a save asks where baked models are
//! no longer used ([`mg_module::bake`]; `docs/walkmesh-bake-proposal.md`).
//!
//! The window says what a bake will make before anything is written (the
//! name, the row, the hak, the rows file, an earlier bake taken again)
//! and shows the selection's walkmeshes in the area view as they are
//! (orange) and as baked (green). Baking writes the models into the
//! project's hak and the rows into its rows file, and sets each object's
//! `Appearance`: one step of Undo, which takes the hak and the rows file
//! back with it.

use std::path::PathBuf;

use egui::Ui;
use glam::Vec3;
use mg_area::ObjectKind;
use mg_core::ResRef;
use mg_edit::{Command, Edit, GffPath};
use mg_gff::Value;
use mg_module::bake::{self, Plan};

use crate::{Action, Moonglow};

/// The bake window's state, its steps' rows files kept for Undo, and the
/// save's question.
#[derive(Debug, Default)]
pub struct BakeState {
    /// The window, while it is open.
    pub draft: Option<BakeDraft>,
    /// What the Position menu says of the placeables selected, what for,
    /// and the pass it was last asked in: worked out as the menu opens
    /// (and when what it is of changes), not every frame it is open.
    menu: Option<(MenuFor, MenuSays, u64)>,
    /// How many times that was worked out (for tests).
    pub menus_worked: u64,
    /// The rows file as each step of Undo found it and left it.
    pub(crate) steps: Vec<(u64, RowsWritten)>,
    /// The save's question, while it waits.
    pub(crate) ask: Option<TidyAsk>,
    /// What the builder last chose to keep: not asked of again.
    kept: Option<bake::Unused>,
    /// Asked and answered: the save held goes on without asking again.
    cleared: bool,
}

/// The rows file before and after a bake.
#[derive(Debug, Clone)]
pub(crate) struct RowsWritten {
    path: PathBuf,
    before: Option<Vec<u8>>,
    after: Vec<u8>,
}

/// The save's question: what no placeable uses, and what to do after.
#[derive(Debug, Clone)]
pub(crate) struct TidyAsk {
    unused: bake::Unused,
    then: Action,
}

/// The Bake Walkmesh window's state.
#[derive(Debug, Clone)]
pub struct BakeDraft {
    pub area: ResRef,
    /// The placeables baked (their places in the area's list).
    pub objects: Vec<usize>,
    /// What baking them will do, worked out as the window opened, and
    /// again when the area's objects changed since.
    pub plan: Result<Plan, String>,
    /// When the area's GIT last changed as the plan was made
    /// (`Workspace::changed_at`).
    planned_at: u64,
}

/// When `area`'s GIT last changed.
fn changed_at(app: &Moonglow, area: ResRef) -> u64 {
    let key = mg_resman::ResKey::new(area, mg_core::ResType::GIT);
    app.ws.as_ref().map_or(0, |ws| ws.changed_at(&key))
}

/// The project's rules and rows file, or why nothing can be baked.
fn rules(app: &Moonglow) -> Result<(bake::Ready, bake::RowsFile), String> {
    let pf = &app.ws.as_ref().ok_or("no module")?.module.project_file;
    let ready = pf.bake.ready()?;
    let rows = pf.baked_rows()?;
    Ok((ready, rows))
}

/// Placed object `index` of the area's placeables, as the module has it
/// now (unsaved edits too).
fn placed(app: &mut Moonglow, area: ResRef, index: usize) -> Option<mg_gff::Struct> {
    let key = mg_resman::ResKey::new(area, mg_core::ResType::GIT);
    let ws = app.ws.as_mut()?;
    let git = ws.doc(&key).ok()?;
    git.root.list(ObjectKind::Placeable.list())?.get(index).cloned()
}

/// Works out baking the placeables `objects` of `area`.
fn plan_for(app: &mut Moonglow, area: ResRef, objects: &[usize]) -> Result<Plan, String> {
    let (ready, rows) = rules(app)?;
    let mut requests = Vec::new();
    for &i in objects {
        let s = placed(app, area, i).ok_or("the placeable is gone")?;
        let t = bake::transform_of(&s).map_err(|e| format!("Not baked: {e}"))?;
        let v = mg_area::VisualTransform {
            scale: Vec3::from(t.scale),
            rotate: Vec3::from(t.rotate),
            translate: Vec3::from(t.translate),
        };
        let appearance = s.integer("Appearance").unwrap_or(0) as u32;
        requests.push(bake::Request {
            appearance,
            transform: t,
            matrix: v.matrix().to_cols_array_2d(),
        });
    }
    let game = app.game.as_deref().ok_or("no game data")?;
    bake::plan(&ready, &rows, &game.resman, game.codepage(), &requests)
}

/// What the Position menu's items are worked out for: the area, the
/// module's revision and the placeables selected.
type MenuFor = (ResRef, u64, Vec<usize>);

/// What the Position menu says of them: why Bake Walkmesh is not offered
/// (if it is not), and those baked, each with the row it goes back to.
type MenuSays = (Result<(), String>, Vec<(usize, u32)>);

/// [`MenuSays`] of the placeables `selected` of `area`. (Each one's
/// struct read from the GIT, and, with the project's `[bake]`, its
/// model's walkmesh read and parsed: two thousand trees selected were as
/// many of both every frame the menu was open.)
fn menu_says(app: &mut Moonglow, area: ResRef, selected: &[usize]) -> MenuSays {
    let why_not = (|| -> Result<(), String> {
        let (_, rows) = rules(app)?;
        let what = if selected.len() == 1 { "It isn't baked" } else { "Not every one" };
        for &i in selected {
            let s = placed(app, area, i).ok_or("the placeable is gone")?;
            bake::transform_of(&s).map_err(|e| format!("{what}: {e}"))?;
            // (A model with no walkmesh has nothing to bake: the row a baked
            // one copies, its model.)
            let a = s.integer("Appearance").unwrap_or(0) as u32;
            let row = rows.row(a).map_or(a, |r| r.copy_of);
            let game = app.game.as_deref().ok_or("no game data")?;
            let table = game.table("placeables").map_err(|e| e.to_string())?;
            let model = table
                .get(row as usize, "ModelName")
                .ok_or_else(|| format!("{what}: row {row} of placeables.2da has no model"))?;
            bake::walkmesh_of(&game.resman, model).map_err(|e| format!("{what}: {e}"))?;
        }
        Ok(())
    })();
    let rows = rules(app).map(|(_, r)| r).unwrap_or_default();
    let baked: Vec<(usize, u32)> = selected
        .iter()
        .filter_map(|&i| {
            let a = placed(app, area, i)?.integer("Appearance")? as u32;
            Some((i, bake::unbaked_row(&rows, a).ok()?))
        })
        .collect();
    (why_not, baked)
}

/// The Position menu's items for selected placeables: Bake Walkmesh to
/// Transform… (offered with the project's `[bake]` and a visual
/// transform, a tip saying why not otherwise) and Unbake Walkmesh (on
/// those baked).
pub(crate) fn menu_items(app: &mut Moonglow, view: &crate::area_view::AreaView, ui: &mut Ui) {
    let selected: Vec<usize> = view
        .selection
        .iter()
        .filter(|(k, _)| *k == ObjectKind::Placeable)
        .map(|(_, i)| *i)
        .collect();
    if selected.is_empty() || selected.len() != view.selection.len() {
        return;
    }
    ui.separator();
    // (Worked out once for the selection and the module as they are.)
    let revision = app.ws.as_ref().map_or(0, mg_edit::Workspace::revision);
    let wanted: MenuFor = (view.area, revision, selected.clone());
    let now = ui.ctx().cumulative_pass_nr();
    let kept = |(of, _, at): &(MenuFor, MenuSays, u64)| *of == wanted && now <= at + 1;
    if !app.bake.menu.as_ref().is_some_and(kept) {
        let says = menu_says(app, view.area, &selected);
        app.bake.menu = Some((wanted, says, now));
        app.bake.menus_worked += 1;
    }
    let Some((_, (why_not, baked), at)) = &mut app.bake.menu else { return };
    *at = now;
    let (why_not, baked) = (why_not.clone(), baked.clone());
    let tip =
        "Give each a model copy whose walkmesh blocks as it looks. Shows what it makes first.";
    let r = ui
        .add_enabled(why_not.is_ok(), egui::Button::new("Bake Walkmesh to Transform…"))
        .on_hover_text(tip)
        .on_disabled_hover_text(why_not.as_ref().err().map_or(tip, String::as_str));
    if r.clicked() {
        open(app, view.area, selected.clone());
        ui.close();
    }
    if !baked.is_empty()
        && ui
            .button("Unbake Walkmesh")
            .on_hover_text(
                "Set each baked one back to its original row and the model's own walkmesh",
            )
            .clicked()
    {
        unbake(app, view.area, &baked);
        ui.close();
    }
}

/// Opens the window on placeables `objects` of `area`, with its plan, and
/// the area view's Object Walkmeshes on (to see before and after).
pub(crate) fn open(app: &mut Moonglow, area: ResRef, objects: Vec<usize>) {
    let plan = plan_for(app, area, &objects);
    if let Some(v) = app.area_views.get_mut(&area) {
        v.object_walkmesh = true;
    }
    let planned_at = changed_at(app, area);
    app.bake.draft = Some(BakeDraft { area, objects, plan, planned_at });
}

/// Sets `objects` (each with the row it goes back to) back: one step.
fn unbake(app: &mut Moonglow, area: ResRef, objects: &[(usize, u32)]) {
    let key = mg_resman::ResKey::new(area, mg_core::ResType::GIT);
    let edits = objects
        .iter()
        .map(|&(i, row)| Edit::SetField {
            key,
            path: GffPath::root().item(ObjectKind::Placeable.list(), i),
            label: "Appearance".into(),
            value: Some(Value::Dword(row)),
        })
        .collect();
    app.actions.push(Action::Apply(Command::new("Unbake Walkmesh", edits)));
}

/// Draws the window, when open.
pub(crate) fn window(app: &mut Moonglow, ui: &mut Ui) {
    let Some(mut d) = app.bake.draft.take() else { return };
    // (Its placeables scaled or turned again, or given another
    // appearance, with the window open: what it would bake is worked out
    // from them as they are now.)
    let now = changed_at(app, d.area);
    if now != d.planned_at {
        d.plan = plan_for(app, d.area, &d.objects);
        d.planned_at = now;
    }
    let mut open = true;
    let mut done = None;
    // (Beside the view, not over the objects whose walkmeshes it shows.)
    let corner = ui.ctx().content_rect().right_top() + egui::vec2(-24.0, 96.0);
    crate::widgets::dialog(ui.ctx(), "Bake Walkmesh to Transform", &mut open)
        .pivot(egui::Align2::RIGHT_TOP)
        .default_pos(corner)
        .resizable(false)
        .collapsible(false)
        .show(ui.ctx(), |ui| {
            ui.set_max_width(460.0);
            match &d.plan {
                Ok(plan) => {
                    for line in plan.describe().lines() {
                        ui.add(egui::Label::new(line).wrap());
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.colored_label(BEFORE, "■");
                        ui.label("as the game has it now");
                        ui.colored_label(AFTER, "■");
                        ui.label("as baked");
                    });
                    ui.weak(
                        "The model is drawn as before; only what it blocks changes. Players \
                         need the hak, and the project's build adds the rows to placeables.2da.",
                    );
                }
                Err(e) => {
                    ui.colored_label(ui.visuals().warn_fg_color, e);
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                let ok = d.plan.as_ref().is_ok_and(|p| p.bakes().next().is_some());
                if ui.add_enabled(ok, egui::Button::new("Bake")).clicked() {
                    done = Some(true);
                }
                if crate::widgets::cancel(ui) {
                    done = Some(false);
                }
            });
        });
    match done {
        Some(true) => {
            if let Ok(plan) = &d.plan {
                made(app, &d, plan);
            }
        }
        Some(false) => {}
        None if open => app.bake.draft = Some(d),
        None => {}
    }
}

/// Before: the walkmeshes as the game has them.
const BEFORE: egui::Color32 = egui::Color32::from_rgb(255, 150, 40);
/// After: as baked.
const AFTER: egui::Color32 = egui::Color32::from_rgb(80, 220, 110);

/// The baked walkmeshes the window would make in `area`, in the area's
/// space, each with its object's place among the area's objects: drawn
/// green over the area view's own.
pub(crate) fn after_faces(app: &Moonglow, area: &mg_area::AreaModel, at: ResRef) -> Vec<[Vec3; 3]> {
    let Some(d) = app.bake.draft.as_ref().filter(|d| d.area == at) else { return Vec::new() };
    let Ok(plan) = &d.plan else { return Vec::new() };
    let mut out = Vec::new();
    for (&i, b) in d.objects.iter().zip(&plan.each) {
        let Ok(b) = b else { continue };
        let o = area.objects.iter().find(|o| o.kind == ObjectKind::Placeable && o.index == i);
        let Some(o) = o else { continue };
        let to = o.transform();
        out.extend(b.after.iter().map(|f| f.map(|c| to.transform_point3(Vec3::from(c)))));
    }
    out
}

/// The colors the area view draws them in.
pub(crate) fn after_color() -> egui::Color32 {
    AFTER
}

/// Writes the plan: the models into the hak (kept as it was, for Undo),
/// the rows file (kept so too), then the objects' `Appearance` and the hak
/// in the module's list, as one step.
fn made(app: &mut Moonglow, d: &BakeDraft, plan: &Plan) {
    let say = |app: &mut Moonglow, e: String| app.log.error(format!("Bake Walkmesh: {e}"));
    let Some(dir) = app.install.as_ref().and_then(mg_resman::GameInstall::user_hak_dir) else {
        return say(app, "there is no user folder to keep the hak in".into());
    };
    let Some(rows_path) = app.ws.as_ref().and_then(|w| w.module.project_file.bake_rows_path())
    else {
        return say(app, "the project file has no folder".into());
    };
    // (Open in the hak editor: its own edits would be lost, or ours.)
    let hak_path = dir.join(format!("{}.hak", plan.hak));
    if app.haks.iter().any(|h| h.hak.path.as_deref() == Some(hak_path.as_path())) {
        return say(
            app,
            format!("{}.hak is open in the hak editor: close it there first", plan.hak),
        );
    }
    let mut written = Vec::new();
    if !plan.files.is_empty() {
        let files = (plan.files.iter())
            .map(|(k, data)| (*k, mg_plugin::HakData::Bytes(data.as_slice().into())))
            .collect();
        let write = mg_plugin::HakWrite { name: plan.hak.clone(), files, held: None };
        match write.write_kept(&dir) {
            Ok(done) => written.push(done),
            Err(e) => return say(app, format!("{}.hak was not written: {e}", plan.hak)),
        }
    }
    let mut rows = None;
    if plan.rows_changed {
        let before = std::fs::read(&rows_path).ok();
        if let Err(e) = plan.rows.write(&rows_path) {
            take_back(app, &mut written, None);
            return say(app, e);
        }
        let after = std::fs::read(&rows_path).unwrap_or_default();
        rows = Some(RowsWritten { path: rows_path, before, after });
    }
    // The module reads its rows file again before the haks are layered.
    if let Some(ws) = app.ws.as_mut() {
        ws.module.reread_project_file();
    }
    let git = mg_resman::ResKey::new(d.area, mg_core::ResType::GIT);
    let mut edits = Vec::new();
    for (&i, b) in d.objects.iter().zip(&plan.each) {
        let Ok(b) = b else { continue };
        let now = placed(app, d.area, i).and_then(|s| s.integer("Appearance"));
        if now != Some(i64::from(b.row)) {
            edits.push(Edit::SetField {
                key: git,
                path: GffPath::root().item(ObjectKind::Placeable.list(), i),
                label: "Appearance".into(),
                value: Some(Value::Dword(b.row)),
            });
        }
    }
    let listed = app.listed_haks();
    if !listed.iter().any(|h| h.eq_ignore_ascii_case(&plan.hak)) {
        let info_key = crate::module_props::info_key();
        if let Some(info) = app.ws.as_mut().and_then(|ws| ws.doc(&info_key).ok()) {
            let list = mg_module::attach::hak_list(&info.root, std::slice::from_ref(&plan.hak));
            edits.push(Edit::SetField {
                key: info_key,
                path: GffPath::root(),
                label: mg_schema::ifo::MOD_HAK_LIST.label.to_string(),
                value: Some(list),
            });
        }
    }
    if edits.is_empty() {
        app.log.info("Bake Walkmesh: each is baked so already: nothing changed");
        after_files(app);
        return;
    }
    match app.apply(Command::new("Bake Walkmesh to Transform", edits)) {
        Ok(()) => {
            if let Some(step) = app.ws.as_ref().and_then(|w| w.last_step()) {
                if !written.is_empty() {
                    app.plugins.hak_steps.push((step, written));
                }
                if let Some(rows) = rows {
                    app.bake.steps.push((step, rows));
                }
            }
            for b in plan.bakes() {
                let how = if b.reused { "taken again" } else { "made" };
                app.log.info(format!(
                    "Bake Walkmesh: {} (row {}, a copy of {}) {how}",
                    b.model, b.row, b.original
                ));
            }
        }
        Err(e) => {
            take_back(app, &mut written, rows.as_ref());
            say(app, e.to_string());
        }
    }
    after_files(app);
}

/// The hak and the rows file put back as they were, a bake's edits not
/// made.
fn take_back(
    app: &mut Moonglow,
    written: &mut Vec<mg_plugin::HakWritten>,
    rows: Option<&RowsWritten>,
) {
    for mut done in written.drain(..).rev() {
        if let Err(e) = done.undo() {
            app.log.warn(format!("Bake Walkmesh: {e}"));
        }
    }
    if let Some(r) = rows {
        put(app, &r.path, r.before.as_deref());
    }
    if let Some(ws) = app.ws.as_mut() {
        ws.module.reread_project_file();
    }
}

/// Writes `data` at `path` (none: takes the file away).
fn put(app: &mut Moonglow, path: &std::path::Path, data: Option<&[u8]>) {
    let done = match data {
        // (Whole or not at all: the project's build reads this file.)
        Some(d) => mg_module::write_replacing(path, d),
        None => std::fs::remove_file(path).or_else(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Ok(()),
            _ => Err(e),
        }),
    };
    if let Err(e) = done {
        app.log.warn(format!("{}: {e}", path.display()));
    }
}

/// The game data after the hak and the rows file changed: the hak read
/// again, the rows the table lacks laid over it.
fn after_files(app: &mut Moonglow) {
    app.reload_resources(false);
    app.sync_bake_rows();
}

/// After Undo (`redo`: Redo) of step `step`: the rows file as the bake
/// found it (left it).
pub(crate) fn follow(app: &mut Moonglow, step: Option<u64>, redo: bool) {
    match &app.ws {
        Some(ws) => app.bake.steps.retain(|(s, _)| ws.has_step(*s)),
        None => app.bake.steps.clear(),
    }
    let Some(step) = step else { return };
    let Some((_, r)) = app.bake.steps.iter().find(|(s, _)| *s == step).cloned() else { return };
    let data = if redo { Some(r.after.as_slice()) } else { r.before.as_deref() };
    put(app, &r.path, data);
    let what = if redo { "written again" } else { "as it was" };
    app.log.info(format!("{}: {what}", r.path.display()));
    if let Some(ws) = app.ws.as_mut() {
        ws.module.reread_project_file();
    }
    app.sync_bake_rows();
}

/// What no placeable of the module uses any longer of the bake's (none
/// without `[bake]`).
fn unused_now(app: &mut Moonglow) -> bake::Unused {
    let Ok((ready, rows)) = rules(app) else { return Default::default() };
    let models = app.game.as_deref().map(|g| bake::hak_models(&g.resman, &ready.hak));
    let Some(ws) = app.ws.as_mut() else { return Default::default() };
    if ws.flush().is_err() {
        return Default::default();
    }
    bake::unused(&ws.module, &rows, &ready.names, &models.unwrap_or_default())
}

/// Before a save: baked models and rows no placeable of the module uses
/// any longer, asked about (Tidy Up, Keep Them). Whether the save is held
/// (`then` goes on once answered).
pub(crate) fn hold_save(app: &mut Moonglow, then: Action) -> bool {
    if std::mem::take(&mut app.bake.cleared) {
        return false;
    }
    let unused = unused_now(app);
    if unused.is_empty() || app.bake.kept.as_ref() == Some(&unused) {
        return false;
    }
    app.bake.ask = Some(TidyAsk { unused, then });
    true
}

/// The save's question, while it waits.
pub(crate) fn save_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(ask) = app.bake.ask.clone() else { return };
    let mut open = true;
    let (mut tidy, mut keep, mut cancel) = (false, false, false);
    let hak = rules(app).map(|(r, _)| r.hak).unwrap_or_default();
    crate::widgets::dialog(ctx, "Baked Models No Longer Used", &mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_max_width(460.0);
            ui.label("No placeable of this module uses these any longer:");
            ui.add_space(4.0);
            for r in &ask.unused.rows {
                ui.label(format!("  •  row {} ({}): {}", r.row, r.model, r.label));
            }
            let in_rows = |m: &str| ask.unused.rows.iter().any(|r| r.model.eq_ignore_ascii_case(m));
            for m in ask.unused.models.iter().filter(|m| !in_rows(m)) {
                ui.label(format!("  •  {m} in {hak}.hak, which no row names"));
            }
            ui.add_space(4.0);
            ui.weak(
                "This module alone was looked at: another module, or a blueprint in a hak, may \
                 still use one. Tidy Up takes them out of the rows file and the hak (kept \
                 beside itself as .bak); Keep Them leaves them.",
            );
            ui.separator();
            ui.horizontal(|ui| {
                tidy = ui.button("Tidy Up").clicked();
                keep = ui.button("Keep Them").clicked();
                cancel = crate::widgets::cancel(ui);
            });
        });
    if tidy || keep {
        app.bake.ask = None;
        if tidy {
            tidy_up(app, &ask.unused);
        } else {
            app.bake.kept = Some(ask.unused.clone());
        }
        app.bake.cleared = true;
        // (The claims and the plugins were asked before, and the module is
        // as it was then: not asked again.)
        app.claims.cleared = true;
        app.plugins.save.cleared = true;
        app.actions.push(ask.then);
    } else if cancel || !open {
        app.bake.ask = None;
        app.log.info("Not saved: the baked models no longer used are as they were");
    }
}

/// Takes `unused` out of the rows file and the hak.
fn tidy_up(app: &mut Moonglow, unused: &bake::Unused) {
    let Ok((ready, mut rows)) = rules(app) else { return };
    let path = app.ws.as_ref().and_then(|w| w.module.project_file.bake_rows_path());
    let hak = app.install.as_ref().and_then(|i| i.find_hak(&ready.hak));
    match bake::tidy(unused, &mut rows, hak.as_deref()) {
        Ok(()) => {
            if let Some(path) = path
                && let Err(e) = rows.write(&path)
            {
                app.log.error(format!("Tidy Up: {e}"));
            }
            app.log.info(format!(
                "Tidy Up: {} row(s) and {} baked model(s) taken out",
                unused.rows.len(),
                unused.models.len()
            ));
        }
        Err(e) => app.log.error(format!("Tidy Up: {e}")),
    }
    if let Some(ws) = app.ws.as_mut() {
        ws.module.reread_project_file();
    }
    after_files(app);
}
