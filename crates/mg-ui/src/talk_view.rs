//! The Talk Table editor: the module's custom talk table, its lines by the
//! StrRefs the game knows them by (16777216 and up), with the feminine
//! table beside it when there is one. The table is a file of its own in the
//! user's `tlk/` folder, saved with the module, with its own undo; a module
//! kept in a nasher project may keep it there as JSON instead, one a
//! language, saved as `nwn_tlk` writes it. A table found in a hak or the
//! module is shown read-only. Open File and New File edit a `.tlk` anywhere
//! instead, with or without a module open.

use egui::Ui;
use mg_core::StrRef;
use mg_module::talk::{self, Line, Table};
use mg_schema::{ExoString, GffValue, ifo};

use crate::text::encode;
use crate::{Action, Moonglow};

#[derive(Debug, Default)]
pub struct TalkView {
    pub filter: String,
    pub selected: Option<usize>,
    /// Bumped on every change to the table, for the cached search.
    revision: u64,
    found: Option<(String, u64, Vec<usize>)>,
    /// The selected line as edited (with plain line ends:
    /// [`shown_line`]), and as the table had it when last applied (an undo
    /// changes the table under it).
    edit: Option<(usize, Line, Line)>,
    error: Option<String>,
    /// New Talk Table: its name and whether it has a feminine table.
    pub new_name: String,
    pub new_feminine: bool,
    /// Only the lines with text or a sound are listed.
    pub only_text: bool,
    /// Those lines, and the revision they are of.
    texted: Option<(u64, Vec<usize>)>,
    /// The list is to scroll to the selected line (the list changed under
    /// it).
    reveal: bool,
    /// The table is a file opened or made on its own (Open File, New
    /// File), not the module's.
    pub outside: bool,
    /// Go to: a StrRef or a line's number as typed, and why it went
    /// nowhere.
    pub go_to: String,
    go_to_problem: Option<String>,
    /// The module's table that was found and could not be read (its name):
    /// said once and not tried again while the module names it.
    unreadable: Option<String>,
    /// The module's table that was not found (its name), and when it was
    /// looked for: looked for again every few seconds, not every frame.
    missing: Option<(String, std::time::Instant)>,
    /// The languages a nasher project keeps the table open in (as JSON, a
    /// file each), where it keeps more than one.
    languages: Vec<(mg_core::Language, std::path::PathBuf)>,
    /// The open table's files and their times when last looked at, to
    /// read it again when they change outside Moonglow ([`read_again`]).
    stamps: Option<Vec<(std::path::PathBuf, Option<std::time::SystemTime>)>>,
}

impl TalkView {
    fn changed(&mut self) {
        self.revision += 1;
    }
}

/// The name the module gives its talk table, if any.
fn named(app: &Moonglow) -> Option<String> {
    app.ws.as_ref()?.module.custom_tlk().ok().flatten().filter(|n| !n.trim().is_empty())
}

/// Opens the module's talk table for editing, if it names one the game
/// finds and another isn't open. Unsaved changes to an open one stay.
/// The name the module gives its table, if any (read from `module.ifo`
/// here, once for those who ask after).
pub(crate) fn load(app: &mut Moonglow) -> Option<String> {
    let name = named(app);
    if let Some(name) = &name
        && !(app.talk_view.outside && app.talk.is_some())
    {
        open(app, name);
    }
    name
}

/// Opens the module's talk table `name`, unless it is the one open, the
/// open one has unsaved changes, or it was tried and could not be read.
fn open(app: &mut Moonglow, name: &str) {
    if app.talk.as_ref().is_some_and(|t| t.name == name || t.is_dirty()) {
        return;
    }
    if app.talk_view.unreadable.as_deref() == Some(name) {
        return;
    }
    let again = std::time::Duration::from_secs(3);
    if (app.talk_view.missing.as_ref()).is_some_and(|(n, at)| n == name && at.elapsed() < again) {
        return;
    }
    let (Some(game), Some(install), Some(ws)) = (&app.game, &app.install, &app.ws) else {
        return;
    };
    let dirs = install.tlk_dirs();
    // (A nasher project's own, in the game's language where it keeps one
    // a language.)
    let language = Some(game.language);
    let find = |name: &str| talk::find_for(&ws.module, &game.resman, &dirs, name, language);
    let found = match find(name) {
        Ok(Some(found)) => found,
        Ok(None) => {
            app.talk = None;
            app.talk_view.missing = Some((name.to_string(), std::time::Instant::now()));
            return;
        }
        Err(e) => {
            app.log.error(format!("Talk table {name}: {e}"));
            app.talk_view.unreadable = Some(name.to_string());
            return;
        }
    };
    let languages = project_languages(app, name);
    let feminine = match &found.source {
        talk::Source::Json { .. } => {
            project_feminine(app, name, found.tlk().ok().map(|t| t.language))
        }
        _ => find(&talk::feminine(name)).ok().flatten(),
    };
    match Table::open(name, found, feminine) {
        Ok(t) => {
            app.talk = Some(t);
            app.talk_view = TalkView { languages, ..TalkView::default() };
        }
        Err(e) => {
            app.log.error(format!("Talk table {name}: {e}"));
            app.talk_view.unreadable = Some(name.to_string());
        }
    }
}

/// The languages the module's nasher project keeps its table `name` in, a
/// JSON file each: none unless it keeps more than one.
fn project_languages(app: &Moonglow, name: &str) -> Vec<(mg_core::Language, std::path::PathBuf)> {
    let Some(p) = app.ws.as_ref().and_then(|ws| ws.module.project.as_deref()) else {
        return Vec::new();
    };
    let tables = p.talk_tables(name).unwrap_or_default();
    let out: Vec<_> = tables.into_iter().map(|t| (t.language(), t.path)).collect();
    if out.len() > 1 { out } else { Vec::new() }
}

/// The feminine table of the project's table `name` in `language`: its
/// own target's (`<name>f`), or none (not a `.tlk` of the tlk folder,
/// which `nasher install` may have left there).
fn project_feminine(
    app: &Moonglow,
    name: &str,
    language: Option<mg_core::Language>,
) -> Option<talk::Found> {
    let p = app.ws.as_ref()?.module.project.as_deref()?;
    let style = p.settings.tlk_style;
    let tables = p.talk_tables(&talk::feminine(name)).unwrap_or_default();
    let f = tables.into_iter().find(|f| Some(f.language()) == language)?;
    Some(talk::Found { source: talk::Source::Json { path: f.path, style }, data: f.file })
}

/// The project's table in another of its languages (its file `path`), in
/// place of the one open (which has no unsaved changes).
fn open_language(app: &mut Moonglow, path: &std::path::Path) {
    let Some(t) = &app.talk else { return };
    let (name, languages) = (t.name.clone(), std::mem::take(&mut app.talk_view.languages));
    let Some(p) = app.ws.as_ref().and_then(|ws| ws.module.project.as_deref()) else { return };
    let style = p.settings.tlk_style;
    let read = |path: &std::path::Path| {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let source = talk::Source::Json { path: path.to_path_buf(), style };
        Ok::<_, String>(talk::Found { source, data })
    };
    let language = languages.iter().find(|(_, p)| p == path).map(|(l, _)| *l);
    let feminine = project_feminine(app, &name, language);
    match read(path).and_then(|f| Table::open(&name, f, feminine)) {
        Ok(t) => {
            app.log.info(format!("Talk table {} ({} lines)", t.source, t.len()));
            app.talk = Some(t);
            app.talk_view = TalkView { languages, ..TalkView::default() };
        }
        Err(e) => {
            app.log.error(format!("Talk table: {e}"));
            app.talk_view.languages = languages;
        }
    }
}

/// The talk table open in the editor, read again from its files where
/// they changed outside Moonglow (a pull of a project's JSON; their times
/// are looked at, as the custom talk table's): the editor shows what the
/// files hold now. One with unsaved changes keeps them (the next Save
/// writes them over the files), and the log says so.
pub(crate) fn read_again(app: &mut Moonglow) {
    let Some(t) = app.talk.as_mut() else {
        app.talk_view.stamps = None;
        return;
    };
    let files: Vec<std::path::PathBuf> = std::iter::once(&t.source)
        .chain(t.feminine.as_ref().map(|(s, _)| s))
        .filter_map(|s| s.file().map(std::path::Path::to_path_buf))
        .collect();
    let now: Vec<_> = (files.into_iter())
        .map(|p| {
            let at = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
            (p, at)
        })
        .collect();
    let was = app.talk_view.stamps.replace(now.clone());
    let same_files =
        was.as_ref().is_some_and(|w| w.iter().map(|(p, _)| p).eq(now.iter().map(|(p, _)| p)));
    // (First looked at, the files are read to see whether they are as
    // the table opened was: they may have changed since.)
    let first = !same_files;
    if !first && was.as_ref() == Some(&now) {
        return;
    }
    if first && t.is_dirty() {
        return;
    }
    match t.read_again() {
        Ok(true) => {
            app.log.info(format!("Talk table {} read again (changed outside Moonglow)", t.source));
            app.talk_view.edit = None;
            app.talk_view.changed();
        }
        Ok(false) => {}
        Err(_) => app.log.warn(format!(
            "Talk table {} changed outside Moonglow while it has unsaved changes here: they are \
             kept, and Save writes them over the file",
            t.source
        )),
    }
}

/// Saves the talk table if it has unsaved changes, and has the game data
/// read it again. Returns false on an error (logged).
pub(crate) fn save(app: &mut Moonglow) -> bool {
    let Some(t) = &mut app.talk else { return true };
    if !t.is_dirty() || !t.editable() {
        return true;
    }
    match t.save() {
        Ok(()) => {
            app.log.info(format!("Saved talk table {} ({} lines)", t.source, t.len()));
            // (Written here: not a change made outside, for `read_again`:
            // its files' times now are the ones known.)
            let files = std::iter::once(&t.source).chain(t.feminine.as_ref().map(|(s, _)| s));
            let stamps = (files.filter_map(|s| s.file()))
                .map(|p| (p.to_path_buf(), std::fs::metadata(p).and_then(|m| m.modified()).ok()))
                .collect();
            app.talk_view.stamps = Some(stamps);
            app.reload_custom_tlk();
            true
        }
        Err(e) => {
            app.log.error(format!("Talk table: {e}"));
            false
        }
    }
}

/// Adds a line to the module's talk table (String Edit › Move to Talk
/// Table); returns its StrRef.
pub(crate) fn add_line(app: &mut Moonglow, line: &Line) -> Result<StrRef, String> {
    load(app);
    let own = !app.talk_view.outside;
    let t = app.talk.as_mut().filter(|t| own && t.editable()).ok_or("no talk table to add to")?;
    let row = t.add_line(line)?;
    app.talk_view.changed();
    Ok(Table::strref(row))
}

/// Whether String Edit can move text into the talk table, and the table's
/// language if so.
pub(crate) fn can_add(app: &mut Moonglow) -> Option<(mg_core::Language, bool)> {
    let name = load(app);
    let t = app.talk.as_ref().filter(|t| t.editable() && !app.talk_view.outside)?;
    (Some(&t.name) == name.as_ref()).then(|| (t.tlk.language, t.feminine.is_some()))
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let outside = app.talk_view.outside && app.talk.is_some();
    if app.ws.is_none() && !outside {
        ui.label("Open a module to edit its talk table, or a talk table file on its own.");
        ui.horizontal_wrapped(|ui| files(&mut app.actions, ui));
        return;
    }
    let name = load(app);
    let open = app.talk.as_ref().is_some_and(|t| outside || Some(&t.name) == name.as_ref());
    if !open {
        no_table(app, ui, name);
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| files(&mut app.actions, ui));
        return;
    }
    // The line chosen when the module's tabs were kept, chosen again and
    // brought into view (once the table is open).
    let len = app.talk.as_ref().map_or(0, |t| t.len());
    let kept = app.open_tabs.take_view(&crate::Tab::TalkTable).and_then(|v| v.talk_line());
    if let Some(line) = kept.filter(|l| *l < len) {
        app.talk_view.selected = Some(line);
        app.talk_view.reveal = true;
    }
    // (The pointer over the editor: Undo and Redo are the table's there.)
    crate::side_doc::note_pointer(app, ui, &crate::Tab::TalkTable);
    table(app, ui);
}

/// Undo (or redo) in the talk table, the pointer over its editor
/// (`Moonglow::local_undo`): whether it is open (with nothing to undo,
/// nothing is).
pub(crate) fn undo(app: &mut Moonglow, redo: bool) -> bool {
    let Some(t) = app.talk.as_mut() else { return false };
    if !(if redo { t.can_redo() } else { t.can_undo() }) {
        return true;
    }
    if redo {
        t.redo()
    } else {
        t.undo()
    }
    app.talk_view.changed();
    true
}

/// Open File… and New File…: a talk table that is a file anywhere. (Put
/// into the row they are called in: in a wrapping row of their own at
/// the end of one, they stood in a sliver past the row's end, one under
/// the other, and held the list a long way down.)
fn files(actions: &mut Vec<Action>, ui: &mut Ui) {
    if ui
        .button("Open File…")
        .on_hover_text(
            "Edit a .tlk file anywhere (its feminine table too, if a file named with an f \
             after its name is beside it)",
        )
        .clicked()
    {
        actions.push(Action::TalkFile(false));
    }
    if ui.button("New File…").on_hover_text("Make an empty .tlk file and edit it").clicked() {
        actions.push(Action::TalkFile(true));
    }
}

/// Open File (or, `new`, New File): the editor takes a talk table that is
/// a file of its own, whatever table the module names.
pub(crate) fn file(app: &mut Moonglow, new: bool) {
    if app.talk.as_ref().is_some_and(|t| t.is_dirty()) {
        app.log.error("The talk table has unsaved changes: save it (or undo them) first.");
        return;
    }
    let start = app.install.as_ref().and_then(|i| i.user_dir.clone()).map(|d| d.join("tlk"));
    let named = |path: &std::path::Path| {
        path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    };
    let table = if new {
        let suggested = start.map(|d| d.join("new.tlk"));
        let kind = crate::dialogs::FileKind::TalkTable;
        let Some(path) = app.dialogs.save_file(kind, suggested.as_deref()) else {
            return;
        };
        let path = if path.extension().is_some() { path } else { path.with_extension("tlk") };
        let language = app.game.as_deref().map_or(mg_core::Language::ENGLISH, |g| g.language);
        let dir = path.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
        let mut t = Table::create(&named(&path), &dir, language, false);
        t.source = talk::Source::File(path);
        t.save().map(|()| t)
    } else {
        let kind = crate::dialogs::FileKind::TalkTable;
        let Some(path) = app.dialogs.open_file(kind, start.as_deref()) else {
            return;
        };
        let found = |p: &std::path::Path| {
            let data = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
            Ok::<_, String>(talk::Found { source: talk::Source::File(p.to_path_buf()), data })
        };
        let name = named(&path);
        let beside = path.with_file_name(format!("{}.tlk", talk::feminine(&name)));
        let feminine = if beside.is_file() { found(&beside).map(Some) } else { Ok(None) };
        found(&path).and_then(|f| Table::open(&name, f, feminine?))
    };
    match table {
        Ok(t) => {
            app.log.info(format!("Talk table {} ({} lines)", t.source, t.len()));
            app.talk = Some(t);
            app.talk_view = TalkView { outside: true, ..Default::default() };
        }
        Err(e) => app.log.error(format!("Talk table: {e}")),
    }
}

/// Export: the table's lines to a file, as CSV for a spreadsheet or as
/// JSON (`nwn_tlk`'s, nasher's). Import: lines read from one, by their
/// StrRefs, as one undoable step.
pub(crate) fn transfer(app: &mut Moonglow, import: bool, json: bool) {
    let Some(t) = app.talk.as_mut() else { return };
    let ext = if json { "json" } else { "csv" };
    let start =
        t.source.file().and_then(|f| f.parent()).map(|d| d.join(format!("{}.{ext}", t.name)));
    let kind = crate::dialogs::FileKind::TalkLines { json };
    if import {
        let Some(path) = app.dialogs.open_file(kind, start.as_deref()) else {
            return;
        };
        let read = std::fs::read_to_string(&path).map_err(|e| e.to_string());
        let read =
            read.and_then(|text| if json { t.import_json(&text) } else { t.import_csv(&text) });
        match read {
            Ok((changed, added)) => {
                app.talk_view.changed();
                app.log.info(format!(
                    "Read {}: {changed} lines changed, {added} added",
                    path.display()
                ));
            }
            Err(e) => app.log.error(format!("{}: {e}", path.display())),
        }
    } else {
        let Some(path) = app.dialogs.save_file(kind, start.as_deref()) else {
            return;
        };
        match std::fs::write(&path, if json { t.to_json() } else { t.to_csv().into_bytes() }) {
            Ok(()) => app.log.info(format!("Wrote {} lines to {}", t.len(), path.display())),
            Err(e) => app.log.error(format!("{}: {e}", path.display())),
        }
    }
}

/// The module has no talk table, or one the game can't find: make one.
fn no_table(app: &mut Moonglow, ui: &mut Ui, name: Option<String>) {
    match &name {
        None => {
            ui.label("This module has no talk table of its own.");
            ui.weak(
                "A talk table holds text that 2DAs, conversations and scripts name by \
                 StrRef: needed for new classes, feats, spells or items in haks.",
            );
        }
        Some(n) => {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!(
                    "The module names the talk table {n}, which isn't in its haks, the module \
                     or the tlk folder: the game won't load the module."
                ),
            );
            if talk::name_hint(n).is_none() && app.talk_view.new_name.is_empty() {
                app.talk_view.new_name = n.clone();
            }
        }
    }
    ui.add_space(8.0);
    // (A nasher project keeps its table in it, as nasher does: its file
    // where the rules put it, and a target of its own.)
    let project = app.ws.as_ref().and_then(|ws| ws.module.project.as_deref());
    let in_project = project.filter(|p| mg_module::nasher::kept_as(&p.settings).is_some());
    let user_dir = app.install.as_ref().and_then(|i| i.user_dir.clone()).map(|d| d.join("tlk"));
    let Some(dir) = user_dir.or_else(|| in_project.map(|p| p.root().join("tlk"))) else {
        ui.label("Making a talk table needs the game's user folder (Tools › Options).");
        return;
    };
    let file_of = |n: &str| match in_project {
        Some(p) => p.talk_table_file(n),
        None => Some(dir.join(format!("{n}.tlk"))),
    };
    let new = app.talk_view.new_name.trim().to_string();
    let made_in = file_of(&new);
    // (In the project, as its nasher.cfg names it: `src/mytlk.tlk.json`.)
    let shown = |f: &std::path::Path| match in_project {
        Some(p) => f.strip_prefix(p.root()).unwrap_or(f).display().to_string().replace('\\', "/"),
        None => f.display().to_string(),
    };
    let made_shown = made_in.as_deref().map(shown);
    let in_project = in_project.is_some();
    let v = &mut app.talk_view;
    ui.horizontal(|ui| {
        ui.label("New talk table");
        ui.add(egui::TextEdit::singleline(&mut v.new_name).desired_width(140.0).hint_text("name"));
        ui.checkbox(&mut v.new_feminine, "With a feminine table")
            .on_hover_text("For languages whose text differs for female characters");
    });
    let problem = if new.is_empty() {
        Some("Name it: up to 16 lower-case letters, digits and _.".to_string())
    } else if new.len() > 16
        || !new.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        Some("A name has up to 16 lower-case letters, digits and _.".to_string())
    } else if made_in.is_none() {
        Some("The project's rules discard talk tables: nowhere to make one.".to_string())
    } else if made_in.as_ref().is_some_and(|f| f.exists()) {
        let f = made_shown.as_deref().unwrap_or_default();
        Some(format!("{f} is there already: choose it in Module Properties."))
    } else {
        None
    };
    if let Some(p) = &problem {
        ui.weak(p);
    } else if let Some(f) = &made_shown {
        match in_project {
            true => ui.weak(format!("Made in the project as {f}, with a target of its own")),
            false => ui.weak(format!("Made in {}", dir.display())),
        };
    }
    if ui.add_enabled(problem.is_none(), egui::Button::new("Create")).clicked() {
        let language = app.game.as_deref().map_or(mg_core::Language::ENGLISH, |g| g.language);
        let feminine = v.new_feminine;
        let made = match in_project {
            true => in_project_table(app, &new, language, feminine),
            false => {
                let mut t = Table::create(&new, &dir, language, feminine);
                t.save().map(|()| t)
            }
        };
        match made {
            Ok(t) => {
                app.log.info(format!("Made talk table {}", t.source));
                app.talk = Some(t);
                app.talk_view = TalkView::default();
                if name.as_deref() != Some(new.as_str()) {
                    app.actions.push(Action::Apply(mg_edit::Command::new(
                        "Set the talk table",
                        vec![mg_edit::Edit::SetField {
                            key: crate::module_props::info_key(),
                            path: mg_edit::GffPath::root(),
                            label: ifo::MOD_CUSTOM_TLK.label.to_string(),
                            value: Some(ExoString(encode(&new)).into_value()),
                        }],
                    )));
                } else {
                    app.reload_custom_tlk();
                }
            }
            Err(e) => app.log.error(format!("Talk table: {e}")),
        }
    }
}

/// Makes the talk table `name` in the module's nasher project
/// ([`mg_module::nasher::Project::add_talk_table`]: its file, a target of
/// its own) and opens it from there.
fn in_project_table(
    app: &mut Moonglow,
    name: &str,
    language: mg_core::Language,
    feminine: bool,
) -> Result<Table, String> {
    let p = app.ws.as_mut().and_then(|ws| ws.module.project.as_deref_mut()).ok_or("no project")?;
    let made = p.add_talk_table(name, language, feminine).map_err(|e| e.to_string())?;
    let style = p.settings.tlk_style;
    let warnings = std::mem::take(&mut p.warnings);
    for w in warnings {
        app.log.warn(w);
    }
    let found = |path: &std::path::Path| -> Result<talk::Found, String> {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(talk::Found { source: talk::Source::Json { path: path.to_path_buf(), style }, data })
    };
    let feminine = made.get(1).map(|f| found(f)).transpose()?;
    app.log.info(format!(
        "nasher.cfg: a target packs {}",
        made.iter().map(|f| f.display().to_string()).collect::<Vec<_>>().join(" and ")
    ));
    Table::open(name, found(&made[0])?, feminine)
}

/// A line as its fields show it: with plain line ends
/// ([`crate::text::to_editor`]).
fn shown_line(line: &Line) -> Line {
    let plain = |s: &str| crate::text::to_editor(s).0;
    Line { text: plain(&line.text), feminine: line.feminine.as_deref().map(plain), ..line.clone() }
}

/// A line as typed in its fields, to set where the table has `stored`:
/// with the line ends of the stored text, those of its lines kept, plain
/// for a text that had none (the game's own talk table has no CRLF).
fn stored_line(typed: &Line, stored: &Line) -> Line {
    let ends = |typed: &str, stored: &str| crate::text::with_line_ends_or(typed, stored, false);
    let feminine = (typed.feminine.as_deref())
        .map(|f| ends(f, stored.feminine.as_deref().unwrap_or_default()));
    Line { text: ends(&typed.text, &stored.text), feminine, ..typed.clone() }
}

fn table(app: &mut Moonglow, ui: &mut Ui) {
    let Moonglow { talk, talk_view: v, actions, ws, settings, .. } = app;
    let t = talk.as_mut().expect("open");
    let editable = t.editable();
    let mut leave = false;
    let mut language = None;
    ui.horizontal(|ui| {
        ui.strong(format!("{}.tlk", t.name));
        let lang = t.tlk.language.name().unwrap_or("?");
        let fem = if t.feminine.is_some() { ", with a feminine table" } else { "" };
        // The nasher project's other languages of it, a file each.
        if !v.languages.is_empty() {
            let open = t.source.file().map(std::path::Path::to_path_buf).unwrap_or_default();
            let mut chosen = open.clone();
            let paths: Vec<std::path::PathBuf> =
                v.languages.iter().map(|(_, p)| p.clone()).collect();
            let named = |p: &std::path::PathBuf| {
                let l = v.languages.iter().find(|(_, q)| q == p).map(|(l, _)| *l);
                l.and_then(mg_core::Language::name).unwrap_or("?").to_string()
            };
            ui.add_enabled_ui(!t.is_dirty(), |ui| {
                crate::widgets::dropdown("talk-language")
                    .selected_text(named(&open))
                    .show(ui, &mut chosen, &paths, named)
                    .on_hover_text(
                        "The project keeps this table in more than one language, a file each \
                         (save or undo the changes to choose another)",
                    );
            });
            language = (chosen != open).then_some(chosen);
        }
        if t.is_dirty() {
            ui.weak("(unsaved)");
        }
        if v.outside
            && ws.is_some()
            && ui
                .add_enabled(!t.is_dirty(), egui::Button::new("Module's Table"))
                .on_hover_text("Back to the talk table the module names")
                .clicked()
        {
            leave = true;
        }
        // (Where it is, cut short where the tab is narrow: the whole of
        // it under the pointer.)
        let about = format!("{} lines, {lang}{fem}; {}", t.len(), t.source);
        ui.add(egui::Label::new(egui::RichText::new(&about).weak()).truncate())
            .on_hover_text(about);
    });
    if leave {
        *talk = None;
        *v = TalkView::default();
        return;
    }
    if let Some(path) = language {
        open_language(app, &path);
        return;
    }
    if !editable {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            "This table is in a hak or the module: change it where that is built (Moonglow \
             edits tables in the tlk folder).",
        );
    }
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Find");
        ui.add(
            egui::TextEdit::singleline(&mut v.filter)
                .desired_width(180.0)
                .hint_text("words or a StrRef"),
        );

        ui.add_enabled_ui(editable, |ui| {
            if ui.button("Add Line").clicked() {
                let empty = Line {
                    text: String::new(),
                    feminine: t.feminine.as_ref().map(|_| String::new()),
                    sound: String::new(),
                    sound_length: 0.0,
                };
                if let Ok(row) = t.add_line(&empty) {
                    v.selected = Some(row);
                    v.filter.clear();
                    // (An empty line: it would not be listed.)
                    v.only_text = false;
                    v.reveal = true;
                    changed = true;
                }
            }
            if ui
                .add_enabled(!t.is_empty(), egui::Button::new("Remove Last Line"))
                .on_hover_text("Only the last: removing another would renumber those after it")
                .clicked()
            {
                t.remove_last();
                changed = true;
            }
            // The table's own (the toolbar's Undo is the module's).
            let hint = "In the talk table (Ctrl+Z, with the pointer over it)";
            if ui.add_enabled(t.can_undo(), egui::Button::new("Undo")).on_hover_text(hint).clicked()
            {
                t.undo();
                changed = true;
            }
            if ui.add_enabled(t.can_redo(), egui::Button::new("Redo")).on_hover_text(hint).clicked()
            {
                t.redo();
                changed = true;
            }
            if ui.add_enabled(t.is_dirty(), egui::Button::new("Save")).clicked() {
                // Saved through the app, which reloads the game's copy.
                actions.push(Action::SaveTalkTable);
            }
        });
    });
    // Go to a line; to and from a spreadsheet or a repository; and what
    // the list shows.
    ui.horizontal_wrapped(|ui| {
        ui.label("Go to");
        let field = ui
            .add(
                egui::TextEdit::singleline(&mut v.go_to)
                    .desired_width(110.0)
                    .hint_text("StrRef or line"),
            )
            .on_hover_text(
                "A StrRef (16777216 and up) or a line's number, then Enter: the list goes \
                 to that line and it is chosen",
            );
        if field.changed() {
            v.go_to_problem = None;
        }
        if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let typed = v.go_to.trim();
            let row = typed
                .parse::<u32>()
                .ok()
                .map(|n| n.checked_sub(talk::CUSTOM).unwrap_or(n) as usize);
            match row {
                Some(row) if row < t.len() => {
                    v.selected = Some(row);
                    // (Among all the lines: one found alone is nowhere
                    // to go to.)
                    v.filter.clear();
                    v.reveal = true;
                    v.go_to_problem = None;
                }
                Some(_) if t.is_empty() => {
                    v.go_to_problem = Some("The table has no lines yet.".into());
                }
                Some(_) => {
                    let last = Table::strref(t.len() - 1).0;
                    v.go_to_problem = Some(format!(
                        "The table has {} lines: StrRefs {} to {last}.",
                        t.len(),
                        talk::CUSTOM
                    ));
                }
                None if typed.is_empty() => v.go_to_problem = None,
                None => v.go_to_problem = Some("A StrRef or a line's number.".into()),
            }
        }
        if let Some(problem) = &v.go_to_problem {
            ui.colored_label(ui.visuals().warn_fg_color, problem);
        }
        // Where the module names the selected line.
        let line = v.selected.filter(|_| ws.is_some());
        if ui
            .add_enabled(line.is_some(), egui::Button::new("Find References"))
            .on_hover_text(
                "Where the module's blueprints, areas, conversations and 2DAs name the \
                 selected line",
            )
            .clicked()
            && let Some(row) = line
        {
            actions.push(Action::FindStrRef(Table::strref(row).0));
        }
        if ui
            .checkbox(&mut v.only_text, "Only lines with text")
            .on_hover_text(
                "Leave the empty lines out of the list (reserved ranges). The selected line \
                 stays selected and in view when this is switched off again.",
            )
            .changed()
        {
            v.reveal = true;
        }
        let import = ui.add_enabled(editable, egui::Button::new("Import CSV…")).on_hover_text(
            "Lines from a spreadsheet saved as CSV (as Export CSV writes it): each row sets the \
             line of its StrRef; rows past the end add lines",
        );
        if import.clicked() {
            actions.push(Action::TalkCsv(true));
        }
        if ui
            .button("Export CSV…")
            .on_hover_text("The lines as a CSV file, for a spreadsheet or a translator")
            .clicked()
        {
            actions.push(Action::TalkCsv(false));
        }
        let import = ui.add_enabled(editable, egui::Button::new("Import JSON…")).on_hover_text(
            "Lines from a talk table kept as JSON (nwn_tlk's, as a nasher repository has it): \
             each entry sets the line of its id; lines the file leaves out stay",
        );
        if import.clicked() {
            actions.push(Action::TalkJson(true));
        }
        if ui
            .button("Export JSON…")
            .on_hover_text("The lines with text as JSON, as nwn_tlk writes a talk table")
            .clicked()
        {
            actions.push(Action::TalkJson(false));
        }
        files(actions, ui);
    });
    if changed {
        v.changed();
    }
    // The rows shown: all, or those found; with Only Lines with Text,
    // those of them that have text or a sound.
    let query = v.filter.trim().to_string();
    if v.only_text && v.texted.as_ref().is_none_or(|(r, _)| *r != v.revision) {
        let has = |row: &usize| {
            let l = t.line(*row);
            !l.text.is_empty() || !l.sound.is_empty() || l.feminine.is_some_and(|f| !f.is_empty())
        };
        v.texted = Some((v.revision, (0..t.len()).filter(has).collect()));
    }
    let rows: Vec<usize> = if query.is_empty() {
        (0..t.len()).collect()
    } else {
        let fresh = v.found.as_ref().is_some_and(|(q, r, _)| *q == query && *r == v.revision);
        if !fresh {
            v.found = Some((query.clone(), v.revision, t.find(&query)));
        }
        v.found.as_ref().map(|f| f.2.clone()).unwrap_or_default()
    };
    let rows: Vec<usize> = match v.texted.as_ref().filter(|_| v.only_text) {
        // (The selected line stays listed while it is being written.)
        Some((_, texted)) => rows
            .into_iter()
            .filter(|r| texted.binary_search(r).is_ok() || v.selected == Some(*r))
            .collect(),
        None => rows,
    };
    v.selected = v.selected.filter(|&s| s < t.len());
    let row_height = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
    let feminine = t.feminine.is_some();
    // The list and the chosen line's editor share the tab's height, and
    // the editor's text boxes what is left of its part: a long text
    // scrolls in its box, and the sound's row stays in sight under it.
    let boxes = if feminine { 2.0 } else { 1.0 };
    let (text, gap) = (ui.text_style_height(&egui::TextStyle::Body), ui.spacing().item_spacing.y);
    let field = ui.spacing().interact_size.y + gap;
    // (The separator, the StrRef's row, a label for each box, the sound's
    // row and a line for what is wrong.)
    let fixed = 2.0 * gap + 6.0 + 2.0 * field + (boxes + 1.0) * (text + gap);
    let least = 3.0 * text + 8.0;
    let available = ui.available_height();
    // The split between them is dragged (its share of the tab is kept,
    // each table's in Moonglow's settings); at first, with two boxes, a
    // little more than half; with one, a little less. The boxes keep
    // three lines, and the editor at most seven tenths of the tab: the
    // list keeps its room, as the conversation editor's lines do.
    let split_key = format!("{}{}", t.name.to_lowercase(), if feminine { "+f" } else { "" });
    let share = (settings.talk_splits.iter().find(|(k, _)| *k == split_key))
        .map(|(_, s)| f32::from(*s) / 1000.0);
    let share = share.unwrap_or(if feminine { 0.55 } else { 0.45 });
    let (fewest, most) = split_limits(available, fixed + boxes * (least + gap));
    let editor = (available * share).clamp(fewest, most);
    let list_height = (available - editor).max(120.0);
    let box_height = ((available - list_height - fixed) / boxes - gap).max(least);
    let mut list = egui::ScrollArea::vertical()
        .id_salt("talk-rows")
        .max_height(list_height)
        .auto_shrink([false, true]);
    // The keys are the list's from a click on a line (GitHub issues 12
    // and 24): Up and Down (Page Up and Down, Home and End) choose a
    // line, Enter goes to its text.
    use crate::palette_view::{Keys, give_keys, keys_are};
    let seen_id = egui::Id::new("talk-rows-seen");
    let mut to_text = false;
    if keys_are(ui.ctx(), Keys::Talk) && !ui.ctx().egui_wants_keyboard_input() && !rows.is_empty() {
        use egui::Key;
        let at = v.selected.and_then(|s| rows.iter().position(|r| *r == s));
        let last = rows.len() as isize - 1;
        let steps = [
            (Key::ArrowDown, 1),
            (Key::ArrowUp, -1),
            (Key::PageDown, 10),
            (Key::PageUp, -10),
            (Key::End, isize::MAX / 2),
            (Key::Home, isize::MIN / 2),
        ];
        let pressed = ui.input_mut(|i| {
            steps.into_iter().find(|(k, _)| i.consume_key(egui::Modifiers::NONE, *k))
        });
        if let Some((_, by)) = pressed {
            let to = at.map_or(0, |at| (at as isize).saturating_add(by).clamp(0, last)) as usize;
            v.selected = Some(rows[to]);
            // (Into view, if it is not among the lines drawn.)
            let seen: Option<(usize, usize)> = ui.data(|d| d.get_temp(seen_id));
            v.reveal = seen.is_none_or(|(from, end)| to <= from || to + 2 >= end);
        }
        to_text = v.selected.is_some()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            give_keys(ui.ctx(), Keys::Area);
        }
    }
    // The list changed under the selected line: it comes back into view.
    if std::mem::take(&mut v.reveal)
        && let Some(at) = v.selected.and_then(|s| rows.iter().position(|r| *r == s))
    {
        let pitch = row_height + ui.spacing().item_spacing.y;
        list = list.vertical_scroll_offset((at as f32 * pitch - list_height / 2.0).max(0.0));
    }
    let shown = list.show_rows(ui, row_height, rows.len(), |ui, range| {
        ui.data_mut(|d| d.insert_temp(seen_id, (range.start, range.end)));
        for &row in &rows[range] {
            let line = t.line(row);
            // The whole row chooses its line (not its number alone), and
            // shows the pointer over it; twice, to go to its text.
            let behind = ui.painter().add(egui::Shape::Noop);
            let selected = v.selected == Some(row);
            let drawn = ui.horizontal(|ui| {
                let strref = Table::strref(row).0.to_string();
                let number = egui::RichText::new(strref).monospace();
                let number = match selected {
                    true => number.color(ui.visuals().selection.stroke.color),
                    false => number,
                };
                ui.allocate_ui_with_layout(
                    egui::vec2(90.0, row_height),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_width(90.0);
                        ui.add(egui::Label::new(number).selectable(false));
                    },
                );
                let w = if feminine { 300.0 } else { 600.0 };
                // The first line of a text, left-aligned in its column.
                let text = |ui: &mut Ui, s: &str| {
                    let first = s.lines().next().unwrap_or_default();
                    let layout = egui::Layout::left_to_right(egui::Align::Center);
                    ui.allocate_ui_with_layout(egui::vec2(w, row_height), layout, |ui| {
                        ui.set_min_width(w);
                        ui.add(egui::Label::new(first).truncate().selectable(false));
                    });
                };
                text(ui, &line.text);
                if let Some(f) = &line.feminine {
                    text(ui, f);
                }
                if !line.sound.is_empty() {
                    ui.add(
                        egui::Label::new(egui::RichText::new(&line.sound).weak()).selectable(false),
                    );
                }
            });
            let mut rect = drawn.response.rect;
            rect.max.x = rect.max.x.max(ui.max_rect().right());
            let r = ui
                .interact(rect, ui.id().with(("talk-row", row)), egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if r.clicked() || r.double_clicked() {
                v.selected = Some(row);
                give_keys(ui.ctx(), Keys::Talk);
                to_text |= r.double_clicked();
            }
            let fill = match (selected, r.hovered()) {
                (true, _) => Some(ui.visuals().selection.bg_fill),
                (false, true) => Some(ui.visuals().widgets.hovered.weak_bg_fill),
                _ => None,
            };
            if let Some(fill) = fill {
                ui.painter().set(
                    behind,
                    egui::Shape::rect_filled(rect.expand2([2.0, 1.0].into()), 2.0, fill),
                );
            }
        }
    });
    crate::palette_view::keys_outline(ui, shown.inner_rect, Keys::Talk);
    if rows.is_empty() && !query.is_empty() {
        ui.weak("No line has that.");
    }
    if let Some(to) = split(ui, editor) {
        let to = to.clamp(fewest, most) / available.max(1.0);
        let to = (to * 1000.0).round().clamp(0.0, 1000.0) as u16;
        settings.talk_splits.retain(|(k, _)| *k != split_key);
        settings.talk_splits.insert(0, (split_key, to));
        settings.talk_splits.truncate(50);
    }
    let Some(row) = v.selected else {
        ui.weak("Choose a line to edit it.");
        return;
    };
    let current = t.line(row);
    if v.edit.as_ref().is_none_or(|(r, _, applied)| *r != row || *applied != current) {
        v.edit = Some((row, shown_line(&current), current));
        v.error = None;
    }
    let (_, line, applied) = v.edit.as_mut().expect("set");
    let strref = Table::strref(row).0;
    ui.horizontal(|ui| {
        ui.strong(format!("StrRef {strref}"));
        ui.weak(format!("line {row}"));
        if ui.small_button("Copy").on_hover_text("Copy the StrRef").clicked() {
            ui.ctx().copy_text(strref.to_string());
        }
    });
    let before = line.clone();
    // Typing in a field: 0 the text, 1 the feminine text.
    let mut typing = None;
    // A text box of the height it has here: a longer text scrolls in it.
    let text_box = |ui: &mut Ui, salt: &str, text: &mut String, hint: &str| {
        // (Each line's box scrolled as it was left, not as another's.)
        let area = egui::ScrollArea::vertical()
            .id_salt((salt, row))
            .max_height(box_height)
            .auto_shrink(false);
        area.show(ui, |ui| {
            let edit = egui::TextEdit::multiline(text)
                .desired_rows(3)
                .desired_width(f32::INFINITY)
                .min_size(egui::vec2(0.0, box_height))
                .hint_text(hint);
            crate::widgets::text_area(ui, egui::Id::new(salt), edit).response.changed()
        })
        .inner
    };
    // (Enter in the list, or a line clicked twice: to its text.)
    if to_text && editable {
        ui.memory_mut(|m| m.request_focus(egui::Id::new("talk-text")));
    }
    ui.add_enabled_ui(editable, |ui| {
        ui.label(if feminine { "Text (masculine)" } else { "Text" });
        if text_box(ui, "talk-text", &mut line.text, "the line's text") {
            typing = Some(0);
        }
        if let Some(f) = &mut line.feminine {
            ui.label("Feminine");
            if text_box(ui, "talk-feminine", f, "the feminine text") {
                typing = Some(1);
            }
        }
        ui.horizontal(|ui| {
            ui.label("Sound");
            ui.add(egui::TextEdit::singleline(&mut line.sound).desired_width(140.0).char_limit(16));
            ui.label("Length (s)");
            ui.add(egui::DragValue::new(&mut line.sound_length).speed(0.05).range(0.0..=600.0));
        });
    });
    if *line != before {
        match t.set_line(row, &stored_line(line, applied), typing) {
            Ok(()) => {
                *applied = t.line(row);
                v.error = None;
                v.revision += 1;
            }
            Err(e) => v.error = Some(e),
        }
    }
    if let Some(e) = &v.error {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
}

/// The least and the most the line's editor under the list takes of the
/// tab's `available` height: what it needs (`fewest`), and at most seven
/// tenths of the tab.
fn split_limits(available: f32, fewest: f32) -> (f32, f32) {
    (fewest, (available * 0.7).max(fewest))
}

/// The line between the list and the line's editor, which is dragged up
/// and down (drawn as a separator is, lit while the pointer is on it):
/// the editor's height dragged to, from its `editor` height now.
fn split(ui: &mut Ui, editor: f32) -> Option<f32> {
    // (As tall as a separator, which it was.)
    let size = egui::vec2(ui.available_width(), 6.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    // (A little taller than the line, to be found by the pointer.)
    let grip = rect.expand2(egui::vec2(0.0, 3.0));
    let r = ui.interact(grip, egui::Id::new("talk-split-grip"), egui::Sense::drag());
    if r.hovered() || r.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    let visuals = ui.visuals();
    let stroke = if r.dragged() {
        visuals.widgets.active.fg_stroke
    } else if r.hovered() {
        visuals.widgets.hovered.fg_stroke
    } else {
        visuals.widgets.noninteractive.bg_stroke
    };
    ui.painter().hline(rect.x_range(), rect.center().y, stroke);
    // (Where the pointer is against where it was pressed, from the height
    // the editor had then: a drag's first steps, before egui calls it a
    // drag, count too.)
    if r.drag_started() {
        ui.data_mut(|d| d.insert_temp(r.id, editor));
    }
    if !r.dragged() {
        return None;
    }
    let from: f32 = ui.data(|d| d.get_temp(r.id))?;
    let (origin, at) = ui.input(|i| (i.pointer.press_origin(), i.pointer.latest_pos()));
    Some(from - (at?.y - origin?.y))
}
