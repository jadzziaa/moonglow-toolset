//! The read-only GFF view (M4's U23, the Resources board): a resource's
//! fields in three columns (name, type, value), a number's meaning beside
//! it as `mg gff --names` gives it (`mg_rules::rows::meaning` and
//! `GameData::meaning_name`, by `gff_view::meaning_text`), struct and list
//! rows that fold.
//!
//! Its rows in sight alone are drawn. The rows the folds leave shown are
//! listed once per change of a fold ([`Tree::changes`]), never per frame,
//! and each row's meaning is found the first time it shows and kept.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use egui::Ui;
use mg_gff::{Field, Gff, Struct, Value};
use mg_rules::rows::Meaning;

/// A row's place: a field of a struct, or an item of a list (the field
/// before it in the path).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Step {
    Field(u32),
    Item(u32),
}

/// A row shown: where it is and how deep.
#[derive(Debug)]
struct Row {
    path: Vec<Step>,
    depth: u16,
}

/// What is kept of a view while its resource is: which rows are unfolded,
/// the rows that leaves shown (made again only when a fold changes), and
/// each shown row's meaning, found once.
#[derive(Debug, Default)]
pub(crate) struct Tree {
    open: HashSet<Vec<Step>>,
    /// Counts the folds' changes: what the rows were listed for.
    changes: u64,
    rows: Option<(u64, Arc<Vec<Row>>)>,
    named: HashMap<Vec<Step>, Option<String>>,
}

thread_local! {
    /// Every listing of a read-only GFF view's shown rows, wherever it is
    /// drawn (the read-only tab, the hak editor's Preview): a test's count
    /// that no tree is walked per frame (M4's U23 fix: one counter for
    /// all). The UI's thread's.
    static WALKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// The most rows one view drew in the last pass that drew one, and
    /// that pass: a test's count of rows in sight alone.
    static DRAWN: std::cell::Cell<(u64, usize)> = const { std::cell::Cell::new((0, 0)) };
}

/// A GFF read once for its read-only view, with its view's [`Tree`].
#[derive(Debug)]
pub(crate) struct ReadGff {
    pub(crate) gff: Gff,
    pub(crate) tree: Mutex<Tree>,
}

impl From<Gff> for ReadGff {
    fn from(gff: Gff) -> ReadGff {
        ReadGff { gff, tree: Mutex::default() }
    }
}

/// The read-only GFF views' counts, wherever they are drawn: how many
/// times their shown rows were listed, all told, and the most rows one
/// drew in the last pass that drew one.
pub(crate) fn counts() -> (usize, usize) {
    (WALKS.with(std::cell::Cell::get), DRAWN.with(std::cell::Cell::get).1)
}

/// The rows the folds leave shown, top down.
fn walk(
    s: &Struct,
    at: &mut Vec<Step>,
    depth: u16,
    open: &HashSet<Vec<Step>>,
    rows: &mut Vec<Row>,
) {
    for (i, f) in s.fields.iter().enumerate() {
        at.push(Step::Field(i as u32));
        rows.push(Row { path: at.clone(), depth });
        if open.contains(at.as_slice()) {
            match &f.value {
                Value::Struct(child) => walk(child, at, depth + 1, open, rows),
                Value::List(items) => {
                    for (j, item) in items.iter().enumerate() {
                        at.push(Step::Item(j as u32));
                        rows.push(Row { path: at.clone(), depth: depth + 1 });
                        if open.contains(at.as_slice()) {
                            walk(item, at, depth + 2, open, rows);
                        }
                        at.pop();
                    }
                }
                _ => {}
            }
        }
        at.pop();
    }
}

/// What a row is.
enum Node<'a> {
    Field(&'a Field),
    Item { list: &'a Field, index: usize, item: &'a Struct },
}

/// A row found by its path: it, the struct it is in (a list's item: the
/// list's), and the labels of the structs and lists above that, as
/// `mg gff --names` passes them.
fn locate<'a>(root: &'a Struct, path: &[Step]) -> Option<(Node<'a>, &'a Struct, Vec<String>)> {
    let (mut s, mut labels, mut i) = (root, Vec::new(), 0);
    loop {
        let Step::Field(f) = *path.get(i)? else { return None };
        let field = s.fields.get(f as usize)?;
        if i + 1 == path.len() {
            return Some((Node::Field(field), s, labels));
        }
        match (&field.value, path[i + 1]) {
            (Value::Struct(child), Step::Field(_)) => {
                labels.push(field.label.to_string_lossy());
                s = child;
                i += 1;
            }
            (Value::List(items), Step::Item(j)) => {
                let item = items.get(j as usize)?;
                if i + 2 == path.len() {
                    return Some((Node::Item { list: field, index: j as usize, item }, s, labels));
                }
                labels.push(field.label.to_string_lossy());
                s = item;
                i += 2;
            }
            _ => return None,
        }
    }
}

/// Where the names beside numbers come from: the game's tables and talk
/// table, and the module's factions (none: a faction's number is shown
/// bare, as `mg gff --names` without a module has it).
#[derive(Clone, Copy)]
pub(crate) struct Names<'a> {
    pub(crate) game: Option<&'a mg_rules::GameData>,
    pub(crate) factions: &'a [String],
}

/// A row's meaning: a number's row label or string (or what is missing),
/// a localized string's talk-table text, a list item's name.
fn meaning_of(
    names: Names<'_>,
    file_type: &str,
    node: &Node<'_>,
    parent: &Struct,
    labels: &[String],
) -> Option<String> {
    use mg_rules::rows::{list_item_name, meaning};
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    // (What it names, else what is missing; nothing for a value that
    // means none: `missing`.)
    let said = |m: &Meaning, v: &Value, n: i64| match names.game {
        Some(g) => match g.meaning_name(m, names.factions, n) {
            Some(name) => Some(name),
            None => missing(names, m, v, n),
        },
        None => Some(crate::gff_view::meaning_text(None, names.factions, m, n)),
    };
    match node {
        Node::Item { list, index, item } => {
            let list = list.label.to_string_lossy();
            list_item_name(names.game, file_type, &labels, &list, (*index, item.id))
        }
        Node::Field(f) => match &f.value {
            Value::LocString(ls) if !ls.strref.is_none() => {
                said(&Meaning::StrRef, &f.value, i64::from(ls.strref.0))
            }
            v => {
                let value = v.as_integer()?;
                let label = f.label.to_string_lossy();
                let m = meaning(names.game, file_type, &labels, &label, &|l| parent.integer(l))?;
                if matches!(m, Meaning::Faction) && names.factions.is_empty() {
                    return None;
                }
                said(&m, v, value)
            }
        },
    }
}

/// What a number that names nothing shows (M4's U23 fix), the game's data
/// read: nothing for a value that means none, where `mg gff --names` too
/// prints nothing, by what `mg_rules` knows of them: no string
/// (`StrRef::is_none`), a number below any row, the field's largest (the
/// row no table has: `mg_rules`' `word` and `byte`, an item property's
/// 255 and 65535 for none), a row the table has with no name (a blank or
/// `****` cell is none: `GameData::row_name`). A string of the module's
/// talk table where none is read says so; a row the table lacks, or a
/// string the game's lacks, says what is missing.
fn missing(names: Names<'_>, m: &Meaning, v: &Value, value: i64) -> Option<String> {
    let game = names.game?;
    match m {
        Meaning::StrRef => {
            let s = mg_core::StrRef(u32::try_from(value).ok()?);
            if s.is_none() {
                return None;
            }
            if s.is_custom() && game.custom_tlk().is_none() {
                let i = s.index();
                return Some(format!("(string {i} of the module's talk table, not read here)"));
            }
            Some(format!("(no string {value})"))
        }
        Meaning::Faction | Meaning::Animation => {
            Some(crate::gff_view::meaning_text(Some(game), names.factions, m, value))
        }
        m => {
            let largest = match v {
                Value::Byte(_) => i64::from(u8::MAX),
                Value::Word(_) => i64::from(u16::MAX),
                Value::Dword(_) => i64::from(u32::MAX),
                _ => i64::MAX,
            };
            if value < 0 || value == largest {
                return None;
            }
            let (table, _) = m.table()?;
            let rows = game.table(table).map_or(0, |t| t.len());
            if usize::try_from(value).is_ok_and(|r| r < rows) {
                return None;
            }
            Some(format!("(no row {value} in {table}.2da)"))
        }
    }
}

/// A value as its column shows it.
fn value_words(v: &Value) -> String {
    match v {
        Value::Struct(s) => counted(s.fields.len(), "field", "fields"),
        Value::List(items) => counted(items.len(), "item", "items"),
        v => crate::browser::value_text(v),
    }
}

/// What a row's first copy item copies, and what it is called (M4's
/// U23 fix): the value itself, not as the column shows it. "Copy Value":
/// a number, a string or resource name as the text it is (its newlines
/// kept, no quotes), a void's bytes as hexadecimal. A localized string
/// says what it copies: its text in the editing language ("Copy English
/// Text"), else in the language it is shown in, else its StrRef ("Copy
/// StrRef"; its talk-table text is its meaning, Copy Meaning). `None`
/// for a struct or a list, or a string with neither.
pub(crate) fn value_copy(v: &Value) -> Option<(String, String)> {
    let value = |text: String| Some(("Copy Value".to_string(), text));
    if let Some(number) = v.number_text() {
        return value(number);
    }
    let in_language = |language: mg_core::Language, text: String| {
        let name =
            language.name().map_or_else(|| format!("Language {}", language.0), str::to_string);
        Some((format!("Copy {name} Text"), text))
    };
    match v {
        Value::String(b) | Value::ResRef(b) => value(crate::text::decode(b)),
        Value::Void(b) => value(b.iter().map(|x| format!("{x:02x}")).collect()),
        Value::LocString(ls) => {
            let own = crate::text::edited_text(ls);
            if !own.is_empty() {
                return in_language(crate::text::edit_language(), own);
            }
            if let Some((language, text)) = crate::text::elsewhere(ls) {
                return in_language(language, text);
            }
            (!ls.strref.is_none()).then(|| ("Copy StrRef".to_string(), ls.strref.0.to_string()))
        }
        _ => None,
    }
}

/// What Copy All Languages copies: a localized string's text in each
/// language it has, a line each ("French (female): …"). `None` where it
/// has text in one language or none.
pub(crate) fn all_languages(v: &Value) -> Option<String> {
    let Value::LocString(ls) = v else { return None };
    if ls.strings.len() < 2 {
        return None;
    }
    let lines = ls.strings.iter().map(|(k, b)| {
        let language = k.language();
        let name =
            language.name().map_or_else(|| format!("Language {}", language.0), str::to_string);
        let female = matches!(k.gender(), mg_core::Gender::Female);
        let text = crate::text::codepage_of(language).decode(b);
        match female {
            true => format!("{name} (female): {text}"),
            false => format!("{name}: {text}"),
        }
    });
    Some(lines.collect::<Vec<_>>().join("\n"))
}

/// `n` things, singular where one.
fn counted(n: usize, one: &str, many: &str) -> String {
    match n {
        1 => format!("1 {one}"),
        n => format!("{n} {many}"),
    }
}

/// The name column's and the type column's widths at most (the board's
/// 347 and 132), and how far each level is set in.
const NAME_W: f32 = 347.0;
const TYPE_W: f32 = 132.0;
const INDENT: f32 = 18.0;

/// The columns' widths in `width`: the board's, shared out where the view
/// is narrower than they and a value's room.
fn columns(width: f32) -> (f32, f32) {
    let want = NAME_W + TYPE_W + 160.0;
    if width >= want {
        return (NAME_W, TYPE_W);
    }
    let k = (width / want).max(0.0);
    (NAME_W * k, TYPE_W * k)
}

/// Draws `g`'s fields read-only, its rows in sight alone. `salt`: the
/// scroll area's and the rows'.
pub(crate) fn ui(
    ui: &mut Ui,
    g: &ReadGff,
    names: Names<'_>,
    salt: impl std::hash::Hash + std::fmt::Debug,
) {
    let t = mg_uikit::theme::tokens(ui.ctx());
    let file_type = String::from_utf8_lossy(&g.gff.file_type).into_owned();
    let mut tree = g.tree.lock().unwrap_or_else(|e| e.into_inner());
    let rows = match &tree.rows {
        Some((at, rows)) if *at == tree.changes => rows.clone(),
        _ => {
            let mut rows = Vec::new();
            walk(&g.gff.root, &mut Vec::new(), 0, &tree.open, &mut rows);
            let rows = Arc::new(rows);
            tree.rows = Some((tree.changes, rows.clone()));
            WALKS.with(|w| w.set(w.get() + 1));
            rows
        }
    };
    let id = egui::Id::new(("read-only-gff", salt));
    let height = mg_uikit::metrics::ROW_HEIGHT;
    let mono = egui::TextStyle::Monospace.resolve(ui.style());
    let body = egui::TextStyle::Body.resolve(ui.style());
    let stripe = t.panel.lerp_to_gamma(t.bar, 0.5);
    let mut toggled = None;
    let mut drawn = 0;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let area = egui::ScrollArea::vertical().id_salt(id).auto_shrink([false, false]);
        area.show_rows(ui, height, rows.len(), |ui, range| {
            let width = ui.available_width();
            let (name_w, type_w) = columns(width);
            for at in range {
                drawn += 1;
                let row = &rows[at];
                let Some((node, parent, labels)) = locate(&g.gff.root, &row.path) else {
                    ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
                    continue;
                };
                let meaning = tree
                    .named
                    .entry(row.path.clone())
                    .or_insert_with(|| meaning_of(names, &file_type, &node, parent, &labels))
                    .clone();
                let (name, kind, value, folds) = match &node {
                    Node::Field(f) => {
                        let kind = match &f.value {
                            Value::Struct(s) => format!("struct {}", s.id),
                            v => v.field_type().json_name().to_string(),
                        };
                        let folds = matches!(f.value, Value::Struct(_) | Value::List(_));
                        (f.label.to_string_lossy(), kind, value_words(&f.value), folds)
                    }
                    Node::Item { index, item, .. } => {
                        let fields = counted(item.fields.len(), "field", "fields");
                        (format!("[{index}]"), format!("struct {}", item.id), fields, true)
                    }
                };
                let open = folds && tree.open.contains(&row.path);
                let sense = egui::Sense::click();
                let (rect, r) = ui.allocate_exact_size(egui::vec2(width, height), sense);
                let r = r.on_hover_cursor(match folds {
                    true => egui::CursorIcon::PointingHand,
                    false => egui::CursorIcon::Default,
                });
                if !ui.is_rect_visible(rect) {
                    continue;
                }
                let p = ui.painter_at(rect);
                if at % 2 == 0 {
                    p.rect_filled(rect, 0.0, stripe);
                }
                let x = rect.left() + f32::from(row.depth) * INDENT;
                if folds {
                    let color = if r.hovered() { t.text } else { t.text_3 };
                    let turn = if open { 1.0 } else { 0.0 };
                    mg_uikit::controls::paint_chevron(
                        &p,
                        egui::pos2(x + 13.0, rect.center().y),
                        turn,
                        color,
                    );
                }
                let text = |s: &str, font: &egui::FontId, color, max: f32| {
                    let mut job =
                        egui::text::LayoutJob::simple_singleline(s.into(), font.clone(), color);
                    job.wrap = egui::text::TextWrapping::truncate_at_width(max.max(0.0));
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                let name_x = x + 25.0;
                let name_room = rect.left() + name_w - 8.0 - name_x;
                let name_g = text(&name, &mono, t.text, name_room);
                let mid = |g: &egui::Galley| rect.center().y - g.size().y / 2.0;
                p.galley(egui::pos2(name_x, mid(&name_g)), name_g.clone(), t.text);
                let type_x = rect.left() + name_w;
                let type_g = text(&kind, &body, t.text_3, type_w - 8.0);
                p.galley(egui::pos2(type_x, mid(&type_g)), type_g, t.text_3);
                // The value, and its meaning after it, quieter.
                let value_x = type_x + type_w;
                let mut job = egui::text::LayoutJob::default();
                let flat = |s: &str| s.replace(['\n', '\r'], " ");
                // (A struct's or list's count muted, in body text: the
                // board's; a value in the code font.)
                let value_fmt = match folds {
                    true => egui::TextFormat::simple(body.clone(), t.text_3),
                    false => egui::TextFormat::simple(mono.clone(), t.text),
                };
                job.append(&flat(&value), 0.0, value_fmt);
                if let Some(m) = &meaning {
                    let gap = if value.is_empty() { 0.0 } else { 10.0 };
                    job.append(&flat(m), gap, egui::TextFormat::simple(body.clone(), t.text_2));
                }
                let room = rect.right() - value_x - 6.0;
                job.wrap = egui::text::TextWrapping::truncate_at_width(room.max(0.0));
                let value_g = ui.fonts_mut(|f| f.layout_job(job));
                p.galley(egui::pos2(value_x, mid(&value_g)), value_g.clone(), t.text);
                let whole = || -> std::borrow::Cow<'_, str> {
                    match &meaning {
                        Some(m) if value.is_empty() => std::borrow::Cow::Borrowed(m),
                        Some(m) => std::borrow::Cow::Owned(format!("{value}  {m}")),
                        None => std::borrow::Cow::Borrowed(&value),
                    }
                };
                let r = match value_g.elided || name_g.elided {
                    true => r.on_hover_ui(|ui| {
                        ui.label(format!("{name}: {}", whole()));
                    }),
                    false => r,
                };
                r.widget_info(|| {
                    let said = format!("{kind}, {}", whole());
                    let mut info = egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &name);
                    info.current_text_value = Some(said);
                    info
                });
                if folds {
                    ui.ctx().accesskit_node_builder(r.id, |n| n.set_expanded(open));
                }
                if folds && r.clicked() {
                    toggled = Some(row.path.clone());
                }
                // Its menu: a right click, or Shift+F10 while it has the
                // keyboard (egui has no menu key), its first item taking
                // the keyboard, Escape giving it back to the row. Ctrl+C
                // while it has the keyboard copies its value. (What each
                // copies is worked out only when asked: not per frame.)
                let field = match &node {
                    Node::Field(f) => Some(&f.value),
                    Node::Item { .. } => None,
                };
                let focused = r.has_focus();
                let keyed = focused
                    && ui.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::F10));
                let copy = focused
                    && ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
                if copy && let Some((_, raw)) = field.and_then(value_copy) {
                    ui.ctx().copy_text(raw);
                }
                let set = if r.secondary_clicked() || keyed {
                    Some(egui::SetOpenCommand::Bool(true))
                } else if r.clicked() {
                    Some(egui::SetOpenCommand::Bool(false))
                } else {
                    None
                };
                // Where it opened: at the pointer for a click, under the
                // row's start for the key (kept, as the menu stays open).
                let at_id = r.id.with("menu-at");
                if r.secondary_clicked() || keyed {
                    let pointer = ui.ctx().pointer_interact_pos().filter(|_| !keyed);
                    let at = pointer.unwrap_or(r.rect.left_bottom());
                    ui.data_mut(|d| d.insert_temp(at_id, at));
                }
                let at = ui.data(|d| d.get_temp::<egui::Pos2>(at_id));
                let menu = egui::Popup::menu(&r).open_memory(set);
                let was_open = menu.is_open();
                let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                let menu = menu.at_position(at.unwrap_or(r.rect.left_bottom()));
                menu.show(|ui| {
                    mg_uikit::menu_rows(ui);
                    let mut first = keyed;
                    let mut copy = |ui: &mut Ui, words: &str, text: &dyn Fn() -> String| {
                        let b = ui.button(words);
                        if std::mem::take(&mut first) {
                            b.request_focus();
                        }
                        if b.clicked() {
                            ui.ctx().copy_text(text());
                            ui.close();
                        }
                    };
                    if let Some((words, _)) = field.and_then(value_copy) {
                        copy(ui, &words, &|| field.and_then(value_copy).unwrap_or_default().1);
                    }
                    if let Some(f) = field.filter(|f| all_languages(f).is_some()) {
                        copy(ui, "Copy All Languages", &|| all_languages(f).unwrap_or_default());
                    }
                    if let Some(m) = &meaning {
                        copy(ui, "Copy Meaning", &|| m.clone());
                    }
                    copy(ui, "Copy Name", &|| name.clone());
                });
                // (Escape closed it: the keyboard back on the row.)
                if was_open && escape {
                    r.request_focus();
                }
            }
        });
    });
    let pass = ui.ctx().cumulative_pass_nr();
    DRAWN.with(|d| {
        let (at, most) = d.get();
        d.set((pass, if at == pass { most.max(drawn) } else { drawn }));
    });
    if let Some(path) = toggled {
        if !tree.open.remove(&path) {
            tree.open.insert(path);
        }
        tree.changes += 1;
        ui.ctx().request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(label: &str, v: i32) -> Field {
        Field { label: mg_gff::Label::new(label).unwrap(), value: Value::Int(v) }
    }

    /// Folded, the root's fields alone are rows; unfolded, a list's items
    /// and an item's fields follow it, each a level deeper.
    #[test]
    fn the_rows_are_those_the_folds_leave_shown() {
        let item = Struct { id: 3, fields: vec![int("A", 1), int("B", 2)] };
        let list =
            Field { label: mg_gff::Label::new("L").unwrap(), value: Value::List(vec![item]) };
        let root = Struct { id: u32::MAX, fields: vec![int("X", 0), list] };
        let mut rows = Vec::new();
        walk(&root, &mut Vec::new(), 0, &HashSet::new(), &mut rows);
        assert_eq!(rows.len(), 2);
        let mut open = HashSet::new();
        open.insert(vec![Step::Field(1)]);
        rows.clear();
        walk(&root, &mut Vec::new(), 0, &open, &mut rows);
        assert_eq!(rows.len(), 3, "the item, folded");
        open.insert(vec![Step::Field(1), Step::Item(0)]);
        rows.clear();
        walk(&root, &mut Vec::new(), 0, &open, &mut rows);
        let depths: Vec<u16> = rows.iter().map(|r| r.depth).collect();
        assert_eq!(depths, [0, 0, 1, 2, 2]);
        let Some((Node::Field(f), parent, labels)) = locate(&root, &rows[4].path) else {
            panic!("found")
        };
        assert_eq!(f.label.to_string_lossy(), "B");
        assert_eq!(parent.id, 3);
        assert_eq!(labels, ["L"]);
    }

    /// The type column's words are the counts' singular where one.
    #[test]
    fn a_count_is_singular_where_one() {
        assert_eq!(value_words(&Value::List(vec![Struct::default()])), "1 item");
        assert_eq!(value_words(&Value::List(Vec::new())), "0 items");
        let s = Struct { id: 1, fields: vec![int("A", 1)] };
        assert_eq!(value_words(&Value::Struct(s)), "1 field");
    }
}
