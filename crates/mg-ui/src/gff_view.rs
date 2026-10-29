//! A generic editor for any GFF resource: its field tree with every value
//! editable. The fallback before (and beside) the dedicated editors.
//!
//! A number that is a row of a 2DA (an area's music, a creature's feats,
//! an item's base item…) is shown with the row's name beside it, and a
//! list to choose the row by name (`mg_rules::rows`).

use std::collections::HashMap;

use egui::Ui;
use mg_core::{Gender, Language};
use mg_edit::{Command, Edit, GffPath, Step};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;
use mg_rules::rows::{RowTable, list_table, row_table};
use mg_rules::{ChoiceColumns, GameData};

use crate::text::{decode, encode, to_editor, with_english};
use crate::{Action, Moonglow};

struct Ctx<'a> {
    key: ResKey,
    /// The file's type (`GIT `, `UTC `…): what its fields mean.
    file_type: String,
    game: Option<&'a GameData>,
    actions: &'a mut Vec<Action>,
    buffers: &'a mut HashMap<egui::Id, String>,
}

/// The labels of the fields and lists a path goes through.
fn labels(path: &GffPath) -> Vec<&str> {
    path.0
        .iter()
        .map(|s| match s {
            Step::Field(l) | Step::Item(l, _) => l.as_str(),
        })
        .collect()
}

/// `n` as a value of `like`'s type, if it fits.
fn of_type(like: &Value, n: i64) -> Option<Value> {
    Some(match like {
        Value::Byte(_) => Value::Byte(u8::try_from(n).ok()?),
        Value::Char(_) => Value::Char(i8::try_from(n).ok()?),
        Value::Word(_) => Value::Word(u16::try_from(n).ok()?),
        Value::Short(_) => Value::Short(i16::try_from(n).ok()?),
        Value::Dword(_) => Value::Dword(u32::try_from(n).ok()?),
        Value::Int(_) => Value::Int(i32::try_from(n).ok()?),
        Value::Dword64(_) => Value::Dword64(u64::try_from(n).ok()?),
        Value::Int64(_) => Value::Int64(n),
        _ => return None,
    })
}

/// `text` on one line, cut to `max` characters (with an ellipsis).
fn shortened(text: &str, max: usize) -> String {
    let line = text.lines().next().unwrap_or_default();
    if line.chars().count() <= max && line.len() == text.trim_end().len() {
        line.to_string()
    } else {
        format!("{}…", line.chars().take(max).collect::<String>())
    }
}

/// What a row of a table is called, as the field view shows it beside the
/// number: the row's name, or that the table has no such row.
pub(crate) fn row_text(game: &GameData, table: &RowTable, row: i64) -> String {
    game.row_name(table, row).unwrap_or_else(|| format!("(no row {row} in {}.2da)", table.table))
}

impl Ctx<'_> {
    fn set(&mut self, path: &GffPath, label: &str, value: Value) {
        self.actions.push(Action::Apply(Command::new(
            format!("Set {label}"),
            vec![Edit::SetField {
                key: self.key,
                path: path.clone(),
                label: label.to_string(),
                value: Some(value),
            }],
        )));
    }

    /// A text box that commits on focus loss; text with line breaks gets a
    /// multi-line box and keeps its line-end style.
    fn text(&mut self, ui: &mut Ui, id: egui::Id, current: &str) -> Option<String> {
        // (As the editor shows it: a lone CR is a line break there too.)
        let rows = to_editor(current).0.contains('\n').then_some(3);
        crate::widgets::commit_text_in(self.buffers, ui, id, current, rows, 280.0, None)
    }

    /// [`Ctx::text`] for a localized string's text (its line ends as
    /// Aurora's: [`crate::widgets::commit_loc_text`]).
    fn loc_text(&mut self, ui: &mut Ui, id: egui::Id, current: &str) -> Option<String> {
        let rows = to_editor(current).0.contains('\n').then_some(3);
        crate::widgets::commit_loc_text_in(self.buffers, ui, id, current, rows, 280.0, None)
    }
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let Moonglow { ws, actions, buffers, game, .. } = app;
    let Some(ws) = ws else { return };
    let doc = match ws.doc(&key) {
        Ok(d) => d,
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return;
        }
    };
    ui.horizontal(|ui| {
        ui.strong(key.to_string());
        ui.weak(format!("{} file", doc.file_type_str()));
    });
    ui.separator();
    let file_type = doc.file_type_str();
    let mut ctx = Ctx { key, file_type, game: game.as_deref(), actions, buffers };
    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        struct_ui(&mut ctx, ui, &doc.root, &GffPath::root());
    });
}

fn struct_ui(ctx: &mut Ctx<'_>, ui: &mut Ui, s: &Struct, path: &GffPath) {
    egui::Grid::new(("gff", ctx.key, path.to_string())).num_columns(3).striped(true).show(
        ui,
        |ui| {
            for f in &s.fields {
                let label = f.label.to_string_lossy();
                let id = egui::Id::new(("gff", ctx.key, path.to_string(), &label));
                match &f.value {
                    Value::Struct(child) => {
                        ui.label(&label);
                        ui.weak(format!("struct {}", child.id));
                        let child_path = path.field(&label);
                        egui::CollapsingHeader::new(format!("{} fields", child.fields.len()))
                            .id_salt(id)
                            .show(ui, |ui| struct_ui(ctx, ui, child, &child_path));
                    }
                    Value::List(items) => {
                        ui.label(&label);
                        ui.weak("list");
                        egui::CollapsingHeader::new(format!("{} items", items.len()))
                            .id_salt(id)
                            .show(ui, |ui| {
                                // A list with an item per row of a table (a
                                // creature's skills): each named.
                                let rows = list_table(&ctx.file_type, &labels(path), &label);
                                for (i, item) in items.iter().enumerate() {
                                    let item_path = path.item(&label, i);
                                    let name = rows
                                        .zip(ctx.game)
                                        .and_then(|(t, g)| g.row_name(&t, i as i64))
                                        .map(|n| format!("  {n}"))
                                        .unwrap_or_default();
                                    egui::CollapsingHeader::new(format!(
                                        "[{i}] struct {}{name}",
                                        item.id
                                    ))
                                    .id_salt((id, i))
                                    .show(ui, |ui| struct_ui(ctx, ui, item, &item_path));
                                }
                            });
                    }
                    v => {
                        ui.label(&label);
                        ui.weak(v.field_type().json_name());
                        let table = v.as_integer().and_then(|n| {
                            Some((row_table(&ctx.file_type, &labels(path), &label)?, n))
                        });
                        match (table, ctx.game) {
                            (Some((table, row)), Some(game)) => {
                                ui.horizontal(|ui| {
                                    value_ui(ctx, ui, id, path, &label, v);
                                    row_ui(ctx, ui, id, path, &label, v, game, &table, row);
                                });
                            }
                            _ => value_ui(ctx, ui, id, path, &label, v),
                        }
                    }
                }
                ui.end_row();
            }
        },
    );
}

/// Beside a number that is a row of `table`: the row's name, in a list of
/// the table's rows to choose another by name.
#[allow(clippy::too_many_arguments)]
fn row_ui(
    ctx: &mut Ctx<'_>,
    ui: &mut Ui,
    id: egui::Id,
    path: &GffPath,
    label: &str,
    v: &Value,
    game: &GameData,
    table: &RowTable,
    row: i64,
) {
    let columns = ChoiceColumns { name: table.name, label: table.label };
    let choices = game.choice_list(table.table, columns).unwrap_or_default();
    let picked = crate::widgets::row_choice(id.with("row"), &choices, row)
        .width(220.0)
        .numbered()
        .shown(row_text(game, table, row))
        .hover_text(format!("Row {row} of {}.2da", table.table))
        .show(ui);
    if let Some(value) = picked.and_then(|p| of_type(v, p)) {
        ctx.set(path, label, value);
    }
}

fn value_ui(ctx: &mut Ctx<'_>, ui: &mut Ui, id: egui::Id, path: &GffPath, label: &str, v: &Value) {
    macro_rules! number {
        ($x:expr, $variant:ident) => {{
            if let Some(n) = crate::widgets::drag_number(ui, *$x, |d| d) {
                ctx.set(path, label, Value::$variant(n));
            }
        }};
    }
    match v {
        Value::Byte(x) => number!(x, Byte),
        Value::Char(x) => number!(x, Char),
        Value::Word(x) => number!(x, Word),
        Value::Short(x) => number!(x, Short),
        Value::Dword(x) => number!(x, Dword),
        Value::Int(x) => number!(x, Int),
        Value::Dword64(x) => number!(x, Dword64),
        Value::Int64(x) => number!(x, Int64),
        Value::Float(x) => {
            if let Some(n) = crate::widgets::drag_number(ui, *x, |d| d.speed(0.01)) {
                ctx.set(path, label, Value::Float(n));
            }
        }
        Value::Double(x) => {
            if let Some(n) = crate::widgets::drag_number(ui, *x, |d| d.speed(0.01)) {
                ctx.set(path, label, Value::Double(n));
            }
        }
        Value::String(b) => {
            if let Some(t) = ctx.text(ui, id, &decode(b)) {
                ctx.set(path, label, Value::String(encode(&t)));
            }
        }
        Value::ResRef(b) => {
            // Resrefs longer than 16 characters are not accepted; the field
            // shows the stored value again.
            if let Some(t) = ctx.text(ui, id, &decode(b))
                && t.len() <= 16
            {
                ctx.set(path, label, Value::ResRef(encode(&t)));
            }
        }
        Value::LocString(ls) => {
            ui.horizontal(|ui| {
                if !ls.strref.is_none() {
                    // The talk table's string, when the game's data is at
                    // hand: what the number stands for.
                    let text = ctx.game.and_then(|g| g.string(ls.strref));
                    let r = ui.weak(format!("StrRef {}", ls.strref.0));
                    if let Some(text) = text {
                        r.on_hover_text(&text);
                        ui.weak(shortened(&text, 48));
                    }
                }
                let english = crate::text::codepage_of(Language::ENGLISH);
                let current = ls
                    .text_in(Language::ENGLISH, Gender::Male, english)
                    .map(|t| t.into_owned())
                    .unwrap_or_default();
                if let Some(t) = ctx.loc_text(ui, id, &current) {
                    ctx.set(path, label, Value::LocString(with_english(ls.clone(), &t)));
                }
            });
        }
        Value::Void(b) => {
            ui.weak(format!("{} bytes", b.len()));
        }
        Value::Struct(_) | Value::List(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chosen_row_keeps_the_fields_type() {
        assert_eq!(of_type(&Value::Int(1), 57), Some(Value::Int(57)));
        assert_eq!(of_type(&Value::Byte(1), 200), Some(Value::Byte(200)));
        assert_eq!(of_type(&Value::Word(1), 9000), Some(Value::Word(9000)));
        // A row the type can't hold, and a field that isn't a number.
        assert_eq!(of_type(&Value::Byte(1), 300), None);
        assert_eq!(of_type(&Value::Float(1.0), 3), None);
    }

    #[test]
    fn long_strings_are_shown_cut() {
        assert_eq!(shortened("Rural Day 1", 48), "Rural Day 1");
        assert_eq!(shortened("A long description", 6), "A long…");
        assert_eq!(shortened("Two\nlines", 48), "Two…");
    }
}
