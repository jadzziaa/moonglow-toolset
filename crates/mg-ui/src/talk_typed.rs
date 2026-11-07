//! Text typed into the module's talk table (`docs/typed-text-proposal.md`):
//! with Options › Language › Put text typed into the talk table on, a
//! name, a description or a conversation line typed becomes a line of the
//! module's custom talk table, at the next free line of the project's
//! range (`moonglow.toml`'s `[talk]`, [`mg_module::talk_range`]), and the
//! field names it by its StrRef. The commands a field's typing makes are
//! marked so (`Action::ApplyTyped`); [`apply`] puts their text in the
//! table and applies them.
//!
//! The line is the module's change, outside the table's own history: it
//! is kept with the module's step that set the field (by the step's place
//! in the module's undo list), and the module's Undo and Redo take it out
//! and put it back with the field ([`stepped`]).

use std::collections::BTreeSet;

use mg_core::{Gender, Language, LocString};
use mg_edit::{Command, Edit};
use mg_gff::Value;
use mg_module::talk::{Line, Table, Typed};
use mg_module::talk_range;

use crate::settings::TalkReuse;
use crate::{Action, Moonglow};

/// What the builder answered for a text the table has already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Name that line.
    Use(usize),
    /// Add another line.
    Add,
    /// Keep the text in the file, Aurora's way.
    File,
}

/// The question asked of a text the table has a line with already.
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    /// The command typed, as it was made.
    pub cmd: Command,
    pub text: String,
    /// The line that has it.
    pub row: usize,
    /// What was answered of its other texts.
    pub answers: Vec<(String, Answer)>,
}

/// The lines typed, with the module's steps they go with, and the
/// question being asked.
#[derive(Debug, Default)]
pub struct TypedState {
    /// The lines of each step Undo can take back, by its depth in the
    /// module's undo list (`Workspace::undo_depth` once it is applied).
    undo: Vec<(usize, Vec<Typed>)>,
    /// The lines of each step Redo can put back, by the depth it is
    /// applied at.
    redo: Vec<(usize, Vec<Typed>)>,
    pub ask: Option<Ask>,
    /// Why typed text stayed in the file, each said once.
    said: BTreeSet<String>,
}

impl TypedState {
    /// Forgets the lines' steps (the module closed, or its history gone):
    /// the lines stay in the table.
    pub(crate) fn forget(&mut self) {
        *self = TypedState::default();
    }

    /// Drops what no longer goes with a step of the module's history
    /// (`depth` commands to undo and `redo` to redo): their lines stay.
    pub(crate) fn prune(&mut self, depth: usize, redo: usize) {
        self.undo.retain(|(d, _)| *d <= depth);
        self.redo.retain(|(d, _)| *d > depth && *d <= depth + redo);
    }

    /// A command was applied: nothing is to be redone any more.
    pub(crate) fn applied(&mut self) {
        self.redo.clear();
    }

    /// The rows Redo would put lines back on: not given to another text.
    fn held(&self) -> BTreeSet<usize> {
        self.redo.iter().flat_map(|(_, t)| t.iter().map(|t| t.row)).collect()
    }
}

/// A text typed into one field or more (Edit Together's several): the
/// edits of the command that set it.
struct Wanted {
    text: String,
    feminine: Option<String>,
    edits: Vec<usize>,
    /// Some edit of them changes the field's text.
    changed: bool,
}

/// The text of a string in `language` (masculine, else feminine), as the
/// field shows it.
fn text_in(ls: &LocString, language: Language, gender: Gender) -> Option<String> {
    let bytes = ls.get(language, gender)?;
    Some(crate::text::entry_codepage(language).decode(bytes).into_owned())
}

/// The table typed text goes into: its language and whether it has a
/// feminine table; or why there is none.
fn table_language(app: &mut Moonglow) -> Result<(Language, bool), String> {
    let Some(name) = crate::talk_view::load(app) else {
        return Err("the module has no custom talk table (Module Properties › Custom Content \
                    names one)"
            .into());
    };
    if crate::talk_view::several_languages(app) {
        return Err(format!(
            "the project keeps the talk table {name} in more than one language: a line in one \
             of them would be empty in the others"
        ));
    }
    match crate::talk_view::can_add(app) {
        Some(found) => Ok(found),
        None => Err(format!(
            "the talk table {name} can't be written here (one of a hak's, or not found where the \
             game looks)"
        )),
    }
}

/// Says once why typed text stays in the file.
fn note(app: &mut Moonglow, why: String) {
    if app.typed.said.insert(why.clone()) {
        app.log.warn(format!("Text typed is kept in the file: {why}"));
    }
}

/// The next line text typed (or Move to Talk Table) takes: in the
/// builder's lines of the project's range, past lines with text, `mg
/// tlk-alloc`'s and those Redo would put back; the table's end without a
/// range. An error where the range is full or the map can't be read.
pub(crate) fn next_line(app: &Moonglow, t: &Table) -> Result<usize, String> {
    let Some(ws) = &app.ws else { return Ok(t.len()) };
    let pf = &ws.module.project_file;
    let allowed = pf.talk_allowed();
    let map = pf.talk_map()?.map(|m| talk_range::map_lines(&m)).unwrap_or_default();
    let held = app.typed.held();
    let len = u32::try_from(t.len()).unwrap_or(u32::MAX);
    let row = talk_range::next_free(
        &allowed,
        len,
        |l| t.is_blank(l as usize),
        |l| map.contains(&l) || held.contains(&(l as usize)),
    );
    row.map(|r| r as usize).ok_or_else(|| {
        format!(
            "the {} of the talk table typed text takes are all used (Module Properties › Custom \
             Content sets other lines)",
            allowed.describe()
        )
    })
}

/// Whether text typed goes into the talk table: the open module's project
/// says (`[talk] typed`), else the builder's Options › Language.
pub(crate) fn enabled(app: &Moonglow) -> bool {
    let project = app.ws.as_ref().and_then(|ws| ws.module.project_file.talk.typed);
    project.unwrap_or(app.settings.talk_typed)
}

/// Applies a command as `Action::Apply` does, saying why it failed.
fn plain(app: &mut Moonglow, cmd: Command) {
    if let Err(e) = app.apply(cmd)
        && !std::mem::take(&mut app.claims.held)
    {
        app.log.error(e.to_string());
    }
}

/// Applies a command a field's typing made (`Action::ApplyTyped`): its
/// text typed put in the talk table first, where Options say so.
pub(crate) fn apply(app: &mut Moonglow, cmd: Command) {
    apply_with(app, cmd, Vec::new());
}

/// [`apply`], with what was answered of texts the table has already.
pub(crate) fn apply_with(app: &mut Moonglow, cmd: Command, answers: Vec<(String, Answer)>) {
    let loc = |e: &Edit| matches!(e, Edit::SetField { value: Some(Value::LocString(_)), .. });
    if !enabled(app) || app.ws.is_none() || !cmd.edits.iter().any(loc) {
        return plain(app, cmd);
    }
    let (language, has_feminine) = match table_language(app) {
        Ok(l) => l,
        Err(why) => {
            note(app, why);
            return plain(app, cmd);
        }
    };
    // The texts typed, each with the fields it goes in.
    let mut wanted: Vec<Wanted> = Vec::new();
    for (i, e) in cmd.edits.iter().enumerate() {
        let Edit::SetField { key, path, label, value: Some(Value::LocString(new)) } = e else {
            continue;
        };
        let Some(text) = text_in(new, language, Gender::Male).filter(|t| !t.trim().is_empty())
        else {
            continue;
        };
        // (The game shows any text of a string's own before its line, in
        // another language too: such a string keeps its text.)
        if talk_range::other_languages(new, language) {
            note(
                app,
                format!(
                    "a string with text in another language than the talk table's ({}) as well: \
                     the game shows that text before a talk-table line, to every player",
                    language.name().unwrap_or("its language")
                ),
            );
            continue;
        }
        let feminine = text_in(new, language, Gender::Female).filter(|f| *f != text);
        if feminine.is_some() && !has_feminine {
            note(
                app,
                "a string with a feminine text of its own, where the talk table has no feminine \
                 table to keep it in"
                    .into(),
            );
            continue;
        }
        let ws = app.ws.as_mut().expect("checked");
        let old = ws.doc(key).ok().and_then(|g| path.get(&g.root)).and_then(|s| s.locstring(label));
        // (Changed from what the field shows: its own text, else its
        // talk-table line's.)
        let shown = old.and_then(|o| {
            text_in(o, language, Gender::Male).or_else(|| {
                let row = talk_range::line_of(o.strref)? as usize;
                let t = app.talk.as_ref()?;
                (row < t.len()).then(|| t.line(row).text)
            })
        });
        let changed = shown.as_ref() != Some(&text);
        match wanted.iter_mut().find(|w| w.text == text && w.feminine == feminine) {
            Some(w) => {
                w.edits.push(i);
                w.changed |= changed;
            }
            None => wanted.push(Wanted { text, feminine, edits: vec![i], changed }),
        }
    }
    wanted.retain(|w| w.changed);
    if wanted.is_empty() {
        return plain(app, cmd);
    }
    // Each text's line: one with the text already (asked first), or a new
    // one.
    let mut choices = Vec::new();
    for w in &wanted {
        let answered = answers.iter().find(|(t, _)| *t == w.text).map(|a| a.1);
        let has = app.talk.as_ref().and_then(|t| t.find_text(&w.text, w.feminine.as_deref()));
        let choice = match (answered, has) {
            (Some(a), _) => a,
            (None, None) => Answer::Add,
            (None, Some(row)) => match app.settings.talk_reuse {
                TalkReuse::Use => Answer::Use(row),
                TalkReuse::Add => Answer::Add,
                TalkReuse::File => Answer::File,
                TalkReuse::Ask => {
                    app.typed.ask = Some(Ask { cmd, text: w.text.clone(), row, answers });
                    return;
                }
            },
        };
        choices.push(choice);
    }
    let mut typed: Vec<Typed> = Vec::new();
    let mut said = Vec::new();
    let mut edits = cmd.edits.clone();
    for (w, choice) in wanted.iter().zip(choices) {
        let row = match choice {
            Answer::File => continue,
            Answer::Use(row) => {
                said.push(format!("uses line {row} of the talk table ({})", strref(row)));
                row
            }
            Answer::Add => {
                let Some(t) = app.talk.as_ref() else { continue };
                let row = match next_line(app, t) {
                    Ok(r) => r,
                    Err(e) => {
                        app.log.error(format!("Text typed kept in the file: {e}"));
                        continue;
                    }
                };
                let line = Line {
                    text: w.text.clone(),
                    feminine: Some(w.feminine.clone().unwrap_or_else(|| w.text.clone())),
                    sound: String::new(),
                    sound_length: 0.0,
                };
                let t = app.talk.as_mut().expect("checked");
                match t.put_typed(row, &line) {
                    Ok(done) => typed.push(done),
                    Err(e) => {
                        app.log.error(format!("Text typed kept in the file: {e}"));
                        continue;
                    }
                }
                said.push(format!("is line {row} of the talk table ({})", strref(row)));
                row
            }
        };
        for &i in &w.edits {
            if let Edit::SetField { value: Some(Value::LocString(ls)), .. } = &mut edits[i] {
                *ls = talk_range::in_line(ls, language, row as u32);
            }
        }
    }
    if !typed.is_empty() {
        shown(app);
    }
    let rewritten = Command::new(cmd.label.clone(), edits);
    let depth = app.ws.as_ref().map_or(0, |ws| ws.undo_depth());
    let result = app.apply(rewritten);
    let now = app.ws.as_ref().map_or(0, |ws| ws.undo_depth());
    match result {
        Ok(()) if now == depth + 1 => {
            if !typed.is_empty() {
                app.typed.undo.push((now, typed));
            }
            for (w, s) in wanted.iter().zip(&said) {
                app.log.info(format!("“{}” {s}", short(&w.text)));
            }
        }
        // (Nothing changed: a blueprint only looked at.)
        Ok(()) => take_back(app, &typed),
        Err(e) => {
            take_back(app, &typed);
            if std::mem::take(&mut app.claims.held) {
                // (Asked first: what goes ahead is the command as typed,
                // its text put in the table then.)
                if let Some(ask) = &mut app.claims.protected_ask {
                    ask.then = Action::ApplyTyped(cmd);
                }
            } else {
                app.log.error(e.to_string());
            }
        }
    }
}

fn strref(row: usize) -> u32 {
    Table::strref(row).0
}

/// A text as the log quotes it: its first line, cut at 60 characters.
fn short(text: &str) -> String {
    let first = text.lines().next().unwrap_or_default();
    match first.char_indices().nth(60) {
        Some((at, _)) => format!("{}…", &first[..at]),
        None if first.len() < text.len() => format!("{first}…"),
        None => first.to_string(),
    }
}

/// The table as it is now, lines typed and taken back, shown at once: in
/// the editor, and in the game data's custom talk table, which the fields
/// and lists read their text from (where it is the module's table) before
/// the module is saved.
fn shown(app: &mut Moonglow) {
    app.talk_view.changed_outside();
    let Some(t) = app.talk.as_ref().filter(|_| !app.talk_view.outside) else { return };
    let tlk = t.tlk.clone();
    app.release_game_data();
    if let Some(game) = crate::exclusive(&mut app.game) {
        game.set_custom_tlk(Some(tlk));
        game.invalidate();
    }
}

/// Takes typed lines back out of the table (last first).
fn take_back(app: &mut Moonglow, typed: &[Typed]) {
    let Some(t) = app.talk.as_mut() else { return };
    for done in typed.iter().rev() {
        if !t.take_back(done) {
            app.log.warn(format!(
                "Line {} of the talk table was changed in its editor since it was typed: left as \
                 it is",
                done.row
            ));
        }
    }
    shown(app);
}

/// Whether the module's Redo may go ahead: the lines it puts back are
/// still free. `depth`: the module's undo depth before it.
pub(crate) fn may_redo(app: &Moonglow, depth: usize) -> Result<(), String> {
    let Some((d, typed)) = app.typed.redo.last() else { return Ok(()) };
    if *d != depth + 1 {
        return Ok(());
    }
    let Some(t) = &app.talk else { return Ok(()) };
    match typed.iter().find(|done| !t.can_put_back(done)) {
        Some(done) => Err(format!(
            "Redo: line {} of the talk table, which it puts back, has been given another text \
             since",
            done.row
        )),
        None => Ok(()),
    }
}

/// After the module's Undo (or, `redo`, Redo) took a step: its typed lines
/// taken out of the table or put back. `depth`: the module's undo depth
/// before the step.
pub(crate) fn stepped(app: &mut Moonglow, redo: bool, depth: usize) {
    if redo {
        if app.typed.redo.last().is_some_and(|(d, _)| *d == depth + 1) {
            let (d, typed) = app.typed.redo.pop().expect("checked");
            if let Some(t) = app.talk.as_mut() {
                for done in &typed {
                    if let Err(e) = t.put_back(done) {
                        app.log.warn(e);
                    }
                }
                shown(app);
            }
            app.typed.undo.push((d, typed));
        }
    } else if app.typed.undo.last().is_some_and(|(d, _)| *d == depth) {
        let (d, typed) = app.typed.undo.pop().expect("checked");
        if app.talk.is_none() {
            app.log.warn("The talk table isn't open: the lines typed with that step stay in it");
        }
        take_back(app, &typed);
        app.typed.redo.push((d, typed));
    }
}

/// The question asked of a text the table has a line with already.
pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(ask) = app.typed.ask.clone() else { return };
    let mut answer = None;
    let mut open = true;
    let mut always = false;
    let title = "Text in the Talk Table";
    crate::widgets::modal(ctx, title).open(&mut open).show(ctx, |ui| {
        ui.set_max_width(420.0);
        ui.label(format!(
            "The talk table has this text already, at line {} (StrRef {}):",
            ask.row,
            strref(ask.row)
        ));
        ui.add(egui::Label::new(egui::RichText::new(short(&ask.text)).strong()).wrap());
        ui.add_space(4.0);
        ui.weak(
            "The field can name that line, or have a line of its own: a line named by several \
             fields changes for each of them when it is changed in the talk table editor.",
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .button(format!("Use Line {}", ask.row))
                .on_hover_text("The field names the line the table has")
                .clicked()
                || crate::widgets::enter(ui)
            {
                answer = Some(Answer::Use(ask.row));
            }
            if ui.button("Add a Line").on_hover_text("A new line with the same text").clicked() {
                answer = Some(Answer::Add);
            }
            if ui
                .button("Keep in the File")
                .on_hover_text("The text is the field's own, as Aurora writes it")
                .clicked()
            {
                answer = Some(Answer::File);
            }
        });
        always = crate::widgets::always_box(
            ui,
            egui::Id::new("talk-typed-always"),
            "Always do this",
            "Tools › Options › Language › When the talk table has the text",
        );
    });
    // (Always so: Options › Language's choice, from now on.)
    if always && let Some(a) = answer {
        app.settings.talk_reuse = match a {
            Answer::Use(_) => TalkReuse::Use,
            Answer::Add => TalkReuse::Add,
            Answer::File => TalkReuse::File,
        };
    }
    if !open || crate::widgets::escape_closes(ctx, title) {
        answer = answer.or(Some(Answer::File));
    }
    if let Some(answer) = answer {
        app.typed.ask = None;
        let mut answers = ask.answers;
        answers.push((ask.text, answer));
        app.actions.push(Action::ApplyTypedWith(ask.cmd, answers));
    }
}
