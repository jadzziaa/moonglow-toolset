//! The Talk Table editor: the module's custom talk table, its lines by the
//! StrRefs the game knows them by (16777216 and up), with the feminine
//! table beside it when there is one. The table is a file of its own in the
//! user's `tlk/` folder, saved with the module, with its own undo. A table
//! found in a hak or the module is shown read-only. Open File and New File
//! edit a `.tlk` anywhere instead, with or without a module open.

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
    /// The selected line as edited, and as the table had it when last
    /// applied (an undo changes the table under it).
    edit: Option<(usize, Line, Line)>,
    error: Option<String>,
    /// New Talk Table: its name and whether it has a feminine table.
    pub new_name: String,
    pub new_feminine: bool,
    /// The pointer is over the editor: Ctrl+Z and Ctrl+Y are the table's
    /// there.
    pub(crate) hovered: bool,
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
pub(crate) fn load(app: &mut Moonglow) {
    if app.talk_view.outside && app.talk.is_some() {
        return;
    }
    let Some(name) = named(app) else { return };
    if app.talk.as_ref().is_some_and(|t| t.name == name || t.is_dirty()) {
        return;
    }
    let (Some(game), Some(install)) = (&app.game, &app.install) else { return };
    let dirs = install.tlk_dirs();
    let Some(found) = talk::find(&game.resman, &dirs, &name) else {
        app.talk = None;
        return;
    };
    let feminine = talk::find(&game.resman, &dirs, &talk::feminine(&name));
    match Table::open(&name, found, feminine) {
        Ok(t) => {
            app.talk = Some(t);
            app.talk_view = TalkView::default();
        }
        Err(e) => app.log.error(format!("Talk table {name}: {e}")),
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
    load(app);
    let t = app.talk.as_ref().filter(|t| t.editable() && !app.talk_view.outside)?;
    (Some(&t.name) == named(app).as_ref()).then(|| (t.tlk.language, t.feminine.is_some()))
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let outside = app.talk_view.outside && app.talk.is_some();
    if app.ws.is_none() && !outside {
        ui.label("Open a module to edit its talk table, or a talk table file on its own.");
        files(&mut app.actions, ui);
        return;
    }
    load(app);
    let name = named(app);
    let open = app.talk.as_ref().is_some_and(|t| outside || Some(&t.name) == name.as_ref());
    if !open {
        no_table(app, ui, name);
        ui.add_space(8.0);
        files(&mut app.actions, ui);
        return;
    }
    app.talk_view.hovered = ui.rect_contains_pointer(ui.max_rect());
    table(app, ui);
}

/// Undo (or redo) in the talk table, when the pointer is over its editor
/// and it has something to undo: whether it did.
pub(crate) fn undo(app: &mut Moonglow, redo: bool) -> bool {
    let Some(t) = app.talk.as_mut().filter(|_| app.talk_view.hovered) else { return false };
    if !(if redo { t.can_redo() } else { t.can_undo() }) {
        return false;
    }
    if redo {
        t.redo()
    } else {
        t.undo()
    }
    app.talk_view.changed();
    true
}

/// Open File… and New File…: a talk table that is a file anywhere.
fn files(actions: &mut Vec<Action>, ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
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
    });
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
        let Some(path) = app.dialogs.save_file(crate::dialogs::FileKind::Any, suggested.as_deref())
        else {
            return;
        };
        let path = if path.extension().is_some() { path } else { path.with_extension("tlk") };
        let language = app.game.as_deref().map_or(mg_core::Language::ENGLISH, |g| g.language);
        let dir = path.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
        let mut t = Table::create(&named(&path), &dir, language, false);
        t.source = talk::Source::File(path);
        t.save().map(|()| t)
    } else {
        let Some(path) = app.dialogs.open_file(crate::dialogs::FileKind::Any, start.as_deref())
        else {
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
    if import {
        let Some(path) = app.dialogs.open_file(crate::dialogs::FileKind::Any, start.as_deref())
        else {
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
        let Some(path) = app.dialogs.save_file(crate::dialogs::FileKind::Any, start.as_deref())
        else {
            return;
        };
        match std::fs::write(&path, if json { t.to_json() } else { t.to_csv() }) {
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
    let Some(dir) = app.install.as_ref().and_then(|i| i.user_dir.clone()).map(|d| d.join("tlk"))
    else {
        ui.label("Making a talk table needs the game's user folder (Tools › Options).");
        return;
    };
    let v = &mut app.talk_view;
    ui.horizontal(|ui| {
        ui.label("New talk table");
        ui.add(egui::TextEdit::singleline(&mut v.new_name).desired_width(140.0).hint_text("name"));
        ui.checkbox(&mut v.new_feminine, "With a feminine table")
            .on_hover_text("For languages whose text differs for female characters");
    });
    let new = v.new_name.trim().to_string();
    let problem = if new.is_empty() {
        Some("Name it: up to 16 lower-case letters, digits and _.".to_string())
    } else if new.len() > 16
        || !new.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        Some("A name has up to 16 lower-case letters, digits and _.".to_string())
    } else if dir.join(format!("{new}.tlk")).exists() {
        Some(format!("{new}.tlk is already in the tlk folder: choose it in Module Properties."))
    } else {
        None
    };
    if let Some(p) = &problem {
        ui.weak(p);
    } else {
        ui.weak(format!("Made in {}", dir.display()));
    }
    if ui.add_enabled(problem.is_none(), egui::Button::new("Create")).clicked() {
        let language = app.game.as_deref().map_or(mg_core::Language::ENGLISH, |g| g.language);
        let mut t = Table::create(&new, &dir, language, v.new_feminine);
        match t.save() {
            Ok(()) => {
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

fn table(app: &mut Moonglow, ui: &mut Ui) {
    let Moonglow { talk, talk_view: v, actions, ws, .. } = app;
    let t = talk.as_mut().expect("open");
    let editable = t.editable();
    let mut leave = false;
    ui.horizontal(|ui| {
        ui.strong(format!("{}.tlk", t.name));
        let lang = t.tlk.language.name().unwrap_or("?");
        let fem = if t.feminine.is_some() { ", with a feminine table" } else { "" };
        ui.weak(format!("{} lines, {lang}{fem}; {}", t.len(), t.source));
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
    });
    if leave {
        *talk = None;
        *v = TalkView::default();
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
    // To and from a spreadsheet or a repository, and what the list shows.
    ui.horizontal_wrapped(|ui| {
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
    let list_height = (ui.available_height() - 230.0).max(120.0);
    let feminine = t.feminine.is_some();
    let mut list = egui::ScrollArea::vertical()
        .id_salt("talk-rows")
        .max_height(list_height)
        .auto_shrink([false, true]);
    // The list changed under the selected line: it comes back into view.
    if std::mem::take(&mut v.reveal)
        && let Some(at) = v.selected.and_then(|s| rows.iter().position(|r| *r == s))
    {
        let pitch = row_height + ui.spacing().item_spacing.y;
        list = list.vertical_scroll_offset((at as f32 * pitch - list_height / 2.0).max(0.0));
    }
    list.show_rows(ui, row_height, rows.len(), |ui, range| {
        for &row in &rows[range] {
            let line = t.line(row);
            ui.horizontal(|ui| {
                let strref = Table::strref(row).0.to_string();
                let selected = v.selected == Some(row);
                let r = ui.add_sized(
                    [90.0, row_height],
                    egui::Button::selectable(selected, egui::RichText::new(strref).monospace()),
                );
                if r.clicked() {
                    v.selected = Some(row);
                }
                let w = if feminine { 300.0 } else { 600.0 };
                // The first line of a text, left-aligned in its column.
                let text = |ui: &mut Ui, s: &str| {
                    let first = s.lines().next().unwrap_or_default();
                    let layout = egui::Layout::left_to_right(egui::Align::Center);
                    ui.allocate_ui_with_layout(egui::vec2(w, row_height), layout, |ui| {
                        ui.set_min_width(w);
                        ui.add(egui::Label::new(first).truncate());
                    });
                };
                text(ui, &line.text);
                if let Some(f) = &line.feminine {
                    text(ui, f);
                }
                if !line.sound.is_empty() {
                    ui.weak(&line.sound);
                }
            });
        }
    });
    if rows.is_empty() && !query.is_empty() {
        ui.weak("No line has that.");
    }
    ui.separator();
    let Some(row) = v.selected else {
        ui.weak("Choose a line to edit it.");
        return;
    };
    let current = t.line(row);
    if v.edit.as_ref().is_none_or(|(r, _, applied)| *r != row || *applied != current) {
        v.edit = Some((row, current.clone(), current));
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
    ui.add_enabled_ui(editable, |ui| {
        ui.label(if feminine { "Text (masculine)" } else { "Text" });
        if ui
            .add(
                egui::TextEdit::multiline(&mut line.text)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text("the line's text"),
            )
            .changed()
        {
            typing = Some(0);
        }
        if let Some(f) = &mut line.feminine {
            ui.label("Feminine");
            if ui
                .add(
                    egui::TextEdit::multiline(f)
                        .desired_rows(3)
                        .desired_width(f32::INFINITY)
                        .hint_text("the feminine text"),
                )
                .changed()
            {
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
        match t.set_line(row, line, typing) {
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
