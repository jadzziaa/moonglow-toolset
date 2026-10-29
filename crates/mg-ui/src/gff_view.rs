//! A generic editor for any GFF resource: its field tree with every value
//! editable. The fallback before (and beside) the dedicated editors.
//!
//! A number that is a row of a 2DA (an area's music, a creature's feats,
//! an item's base item, an item property's subtype and value…), a
//! faction or a conversation line's animation is shown with its name
//! beside it, and a list to choose another by name; a talk-table string
//! held as a plain number with its text (`mg_rules::rows::meaning`, which
//! `mg gff --names` reads too).

use std::collections::HashMap;

use egui::Ui;
use mg_core::{Gender, Language};
use mg_edit::{Command, Edit, GffPath, Step};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;
use mg_rules::GameData;
use mg_rules::rows::{Meaning, list_item_name, meaning};

use crate::text::{decode, encode, to_editor, with_english};
use crate::{Action, Moonglow};

struct Ctx<'a> {
    key: ResKey,
    /// The file's type (`GIT `, `UTC `…): what its fields mean.
    file_type: String,
    game: Option<&'a GameData>,
    /// The module's factions' names, by their places (`repute.fac`).
    factions: Vec<String>,
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

/// What a field's number is called, as the field view shows it beside
/// the number: its name, or that it names nothing there (a row the table
/// lacks).
pub(crate) fn meaning_text(
    game: Option<&GameData>,
    factions: &[String],
    meaning: &Meaning,
    value: i64,
) -> String {
    let name = match game {
        Some(g) => g.meaning_name(meaning, factions, value),
        None => mg_rules::rows::plain_name(meaning, factions, value),
    };
    name.unwrap_or_else(|| match meaning {
        Meaning::Faction => format!("(no faction {value} in repute.fac)"),
        Meaning::Animation => format!("(animation {value})"),
        Meaning::StrRef => format!("(no string {value})"),
        m => format!("(no row {value} in {}.2da)", m.table().map_or("?", |t| t.0)),
    })
}

/// The module's factions' names, by their places in `repute.fac`; without
/// one, the standard factions every module starts with (as the blueprint
/// editors' lists have them).
fn faction_names(ws: &mut mg_edit::Workspace) -> Vec<String> {
    let key = ResKey::parse("repute", mg_core::ResType::FAC).expect("valid");
    let Ok(fac) = ws.doc(&key) else {
        return mg_module::factions::STANDARD.iter().map(|n| n.to_string()).collect();
    };
    let list = fac.root.list("FactionList").unwrap_or(&[]);
    list.iter().map(|f| decode(f.string("FactionName").unwrap_or_default())).collect()
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

/// The field tree of `key`, over its OK and Cancel, as a Properties
/// window's (`blueprint::ok_cancel`): every change is the module's at
/// once, and Cancel sets back what the window changed since it opened.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let targets = [(key, GffPath::root())];
    let pressed = app.ws.is_some().then(|| crate::blueprint::ok_cancel(ui, &targets[0], false));
    let pending = app.actions.len();
    fields(app, ui, key);
    let tab = crate::Tab::Gff(key);
    crate::blueprint::window_drawn(app, pending, &tab, &targets, pressed.flatten());
}

fn fields(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let Moonglow { ws, actions, buffers, game, .. } = app;
    let Some(ws) = ws else { return };
    let factions = faction_names(ws);
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
    let mut ctx = Ctx { key, file_type, game: game.as_deref(), factions, actions, buffers };
    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        struct_ui(&mut ctx, ui, &doc.root, &GffPath::root());
    });
}

/// The kind of object the struct at `path` of a file of `file_type` is:
/// a blueprint's root, or an object an area's GIT places.
fn object_at(file_type: &str, path: &GffPath) -> Option<mg_rules::ObjectKind> {
    use mg_rules::ObjectKind;
    match path.0.as_slice() {
        [] => ObjectKind::from_restype(mg_core::ResType::from_extension(file_type.trim())?),
        [Step::Item(list, _)] if file_type.trim() == "GIT" => ObjectKind::from_list(list),
        _ => None,
    }
}

fn struct_ui(ctx: &mut Ctx<'_>, ui: &mut Ui, s: &Struct, path: &GffPath) {
    // The field that puts the object in the PC faction, as Verify warns of
    // it (`pc-faction`): said beside it.
    let pc_faction =
        object_at(&ctx.file_type, path).and_then(|k| mg_rules::objects::pc_faction(k, s));
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
                                // An item that stands for a row of a table
                                // (a creature's skills), or a store's page:
                                // each named.
                                let labels = labels(path);
                                for (i, item) in items.iter().enumerate() {
                                    let item_path = path.item(&label, i);
                                    let at = (i, item.id);
                                    let name = list_item_name(
                                        ctx.game,
                                        &ctx.file_type,
                                        &labels,
                                        &label,
                                        at,
                                    )
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
                        // What the number stands for: a row of a table
                        // (with the game's data), a faction, an animation,
                        // a talk-table string.
                        let sibling = |l: &str| s.integer(l);
                        let means = v.as_integer().and_then(|n| {
                            let m =
                                meaning(ctx.game, &ctx.file_type, &labels(path), &label, &sibling)?;
                            let needs_game = !matches!(m, Meaning::Faction | Meaning::Animation);
                            (ctx.game.is_some() || !needs_game).then_some((m, n))
                        });
                        let warned = pc_faction == Some(label.as_str());
                        if means.is_none() && !warned {
                            value_ui(ctx, ui, id, path, &label, v);
                        } else {
                            ui.horizontal(|ui| {
                                value_ui(ctx, ui, id, path, &label, v);
                                if let Some((m, n)) = &means {
                                    meaning_ui(ctx, ui, id, (path, &label), v, m, *n);
                                }
                                if warned {
                                    ui.colored_label(
                                        ui.visuals().warn_fg_color,
                                        PC_FACTION_WARNING,
                                    )
                                    .on_hover_text(PC_FACTION_HOVER);
                                }
                            });
                        }
                    }
                }
                ui.end_row();
            }
        },
    );
}

/// Beside an object's faction field of 0 (what Verify's `pc-faction`
/// warns of).
const PC_FACTION_WARNING: &str = "⚠ The PC faction: the players' alone";
const PC_FACTION_HOVER: &str = "Faction 0 is the player characters'. The game puts a creature \
    of it in Hostile; creatures regard a placeable or door of it as they regard players. Give \
    it another faction (Verify warns of it: pc-faction).";

/// Beside a number that stands for something: its name, in a list to
/// choose another by name (a row of the table, a faction, an animation);
/// a talk-table string's text.
fn meaning_ui(
    ctx: &mut Ctx<'_>,
    ui: &mut Ui,
    id: egui::Id,
    (path, label): (&GffPath, &str),
    v: &Value,
    meaning: &Meaning,
    value: i64,
) {
    let shown = meaning_text(ctx.game, &ctx.factions, meaning, value);
    if *meaning == Meaning::StrRef {
        ui.weak(shortened(&shown, 48)).on_hover_text(&shown);
        return;
    }
    let choices = match ctx.game {
        Some(g) => g.meaning_choices(meaning, &ctx.factions),
        None => std::sync::Arc::new(mg_rules::rows::plain_choices(meaning, &ctx.factions)),
    };
    let hover = match meaning {
        Meaning::Faction => format!("Faction {value} of the module's (repute.fac)"),
        Meaning::Animation => format!("Animation {value}, as Aurora lists them"),
        m => format!("Row {value} of {}.2da", m.table().map_or("?", |t| t.0)),
    };
    let picked = crate::widgets::row_choice(id.with("row"), &choices, value)
        .width(220.0)
        .numbered()
        .shown(shown)
        .hover_text(hover)
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
