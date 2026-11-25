//! The Journal Editor (`module.jrl`, M4's U22, the Journal board): its
//! header (Add with its word, Copy, Cut, Paste and Delete, Expand All and
//! Collapse All; OK and Cancel at its end), over a tree of the categories
//! (quests) and their entries, an entry's ID, an End badge where it
//! finishes its category and its text on one line. The chosen entry's
//! fields are the Right Dock's **Entry**, its category's (or the chosen
//! category's) **Category** ([`entry_panel`], [`category_panel`]); the
//! Status Bar's hints and counts are [`shown`]'s. Each change is one
//! undoable command. The journal is read once a revision of the workspace
//! ([`JournalView::walks`] counts each read), and only the rows in sight
//! are laid out. The selection, the categories opened out and the panels'
//! fields' ids follow their objects when the lists are written anew (an
//! entry added, deleted or pasted before them, an Undo: [`retarget`]).

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use egui::Ui;
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath, Step};
use mg_gff::{Struct, Value};
use mg_module::journal::{PRIORITIES, entries, list_value, new_category, new_entry, new_journal};
use mg_resman::ResKey;
use mg_schema::{StructExt, jrl};

use crate::blueprint::Form;
use crate::text::decode;
use crate::{Action, Moonglow, Tab};

/// What is selected: a category, or an entry of one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Node {
    Category(usize),
    Entry(usize, usize),
}

impl Node {
    /// Its place in the journal.
    fn path(self) -> GffPath {
        match self {
            Node::Category(c) => category_path(c),
            Node::Entry(c, e) => entry_path(c, e),
        }
    }

    /// The node at `path`, if it is a category's or an entry's place.
    fn at(path: &GffPath) -> Option<Node> {
        let cats = jrl::CATEGORIES.label;
        let list = jrl::categories::ENTRY_LIST.label;
        match &path.0[..] {
            [Step::Item(l, c)] if l == cats => Some(Node::Category(*c)),
            [Step::Item(l, c), Step::Item(m, e)] if l == cats && m == list => {
                Some(Node::Entry(*c, *e))
            }
            _ => None,
        }
    }

    /// Its category.
    fn category(self) -> usize {
        match self {
            Node::Category(c) | Node::Entry(c, _) => c,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Clip {
    Category(Struct),
    Entry(Struct),
}

/// The journal editor's state.
#[derive(Debug, Clone, Default)]
pub struct JournalView {
    pub selected: Option<Node>,
    /// What Copy or Cut took (kept across closing the tab, as Aurora's).
    pub clipboard: Option<Clip>,
    /// The categories opened out to their entries (closed to begin with,
    /// as in Aurora).
    pub open: BTreeSet<usize>,
    /// The ids the Entry and Category panels keep their fields' state by,
    /// by their object's place (given on first asking; moved with it,
    /// [`retarget`]): what is typed follows its entry or category.
    pub(crate) ids: HashMap<GffPath, egui::Id>,
    /// The journal as read at a revision of the workspace.
    copy: Option<Arc<Seen>>,
    /// The tree's rows for the journal read and the categories open.
    rows: Option<(u64, BTreeSet<usize>, Arc<Vec<Row>>)>,
    /// How many times the journal was read whole.
    walks: usize,
    /// The pointer was over the tree at the last frame.
    hovered: bool,
    /// The selection is to be brought into sight (chosen by a panel, or
    /// added).
    reveal: bool,
    /// The frame (egui's pass) in which a number field of the panels (an
    /// entry's ID, a category's XP) last had the keyboard: what is typed
    /// in it is set as it is left ([`typing_pending`]). Let go of at the
    /// end of a frame it was not seen in.
    pub(crate) number_typed: Option<u64>,
}

impl JournalView {
    /// How many times the journal was read whole (once for each change of
    /// the workspace it is drawn after, never for a frame alone).
    pub fn walks(&self) -> usize {
        self.walks
    }
}

/// The journal as one revision of the workspace has it.
#[derive(Debug)]
struct Seen {
    revision: u64,
    /// The game's data it was read with (its talk table), by
    /// `Moonglow::game_names`.
    names: (u64, u64),
    /// The language its words are shown in.
    language: mg_core::Language,
    /// The module has `module.jrl`.
    exists: bool,
    /// Its categories, or why it can't be read.
    cats: Result<Vec<Struct>, String>,
    /// What the tree and the panels show of each category.
    seen: Vec<CategorySeen>,
}

#[derive(Debug)]
struct CategorySeen {
    name: String,
    tag: String,
    entries: Vec<EntrySeen>,
}

#[derive(Debug)]
struct EntrySeen {
    id: u32,
    end: bool,
    /// Its text's first line.
    text: String,
}

impl Seen {
    fn cats(&self) -> &[Struct] {
        self.cats.as_deref().unwrap_or_default()
    }

    fn entry_count(&self) -> usize {
        self.seen.iter().map(|c| c.entries.len()).sum()
    }
}

/// A row of the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Root,
    /// "The module has no journal categories."
    Empty,
    Category(usize),
    Entry(usize, usize),
}

pub(crate) fn key() -> ResKey {
    ResKey::new(ResRef::from_str("module").expect("valid"), ResType::JRL)
}

fn category_path(c: usize) -> GffPath {
    GffPath::root().item(jrl::CATEGORIES.label, c)
}

fn entry_path(c: usize, e: usize) -> GffPath {
    category_path(c).item(jrl::categories::ENTRY_LIST.label, e)
}

/// The journal editor's header's name (`mg_uikit::header`; a screen
/// reader's and the tests' group).
pub const JOURNAL_HEADER: &str = "Journal header";

/// The tree's name.
pub const JOURNAL_TREE: &str = "Journal";

/// A command replacing the category list (creating the journal if the
/// module has none).
fn set_categories(what: &str, exists: bool, cats: Vec<Struct>) -> Action {
    let mut edits = Vec::new();
    if !exists {
        edits.push(Edit::SetResource { key: key(), data: new_journal().to_bytes().ok() });
    }
    edits.push(Edit::SetField {
        key: key(),
        path: GffPath::root(),
        label: jrl::CATEGORIES.label.to_string(),
        value: Some(list_value(cats)),
    });
    Action::Apply(Command::new(what, edits))
}

fn set_entries(what: &str, category: usize, list: Vec<Struct>) -> Action {
    Action::Apply(Command::new(
        what,
        vec![Edit::SetField {
            key: key(),
            path: category_path(category),
            label: jrl::categories::ENTRY_LIST.label.to_string(),
            value: Some(list_value(list)),
        }],
    ))
}

/// The journal as the workspace has it now: read once a revision (and
/// again where the language its words are shown in changed); the
/// selection the journal no longer has let go of, the category of an
/// entry chosen opened out. `None` without a module.
fn begin(app: &mut Moonglow) -> Option<Arc<Seen>> {
    let names = app.game_names();
    let ws = app.ws.as_mut()?;
    let revision = ws.revision();
    let language = crate::text::edit_language();
    let game = app.game.clone();
    let view = &mut app.journal_view;
    let kept =
        |c: &&Arc<Seen>| c.revision == revision && c.language == language && c.names == names;
    if let Some(c) = view.copy.as_ref().filter(kept) {
        return Some(c.clone());
    }
    view.walks += 1;
    let exists = ws.module.contains(&key());
    let cats = match ws.doc(&key()) {
        Ok(g) => Ok(g.root.items(&jrl::CATEGORIES).to_vec()),
        Err(_) if !exists => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    };
    let game = game.as_deref();
    let seen = cats.as_deref().unwrap_or_default().iter().map(|c| seen_category(game, c)).collect();
    let copy = Arc::new(Seen { revision, names, language, exists, cats, seen });
    // A selection the journal no longer has (an Undo) is let go of.
    let valid = |n: &Node| match *n {
        Node::Category(c) => c < copy.seen.len(),
        Node::Entry(c, e) => copy.seen.get(c).is_some_and(|s| e < s.entries.len()),
    };
    view.selected = view.selected.filter(valid);
    view.open.retain(|c| *c < copy.seen.len());
    view.copy = Some(copy.clone());
    Some(copy)
}

/// What the tree shows of a localized string: its own text in the language
/// edited, else the talk table's (Chapter1's are StrRefs), else another
/// language's (`widgets::loc_shown`'s order).
fn tree_text(game: Option<&mg_rules::GameData>, ls: &mg_core::LocString) -> String {
    let own = crate::text::edited_text(ls);
    if !own.is_empty() {
        return own;
    }
    let table = (!ls.strref.is_none()).then(|| game.and_then(|g| g.string(ls.strref))).flatten();
    match table.filter(|t| !t.is_empty()) {
        Some(text) => text,
        None => crate::text::elsewhere(ls).map(|(_, text)| text).unwrap_or_default(),
    }
}

fn seen_category(game: Option<&mg_rules::GameData>, cat: &Struct) -> CategorySeen {
    let name = tree_text(game, &cat.read(&jrl::categories::NAME));
    let tag = decode(cat.read(&jrl::categories::TAG).as_bytes());
    let entries = entries(cat)
        .iter()
        .map(|entry| {
            use jrl::categories::entry_list as f;
            let text = tree_text(game, &entry.read(&f::TEXT));
            EntrySeen {
                id: entry.read(&f::ID),
                end: entry.read(&f::END) != 0,
                text: text.lines().next().unwrap_or_default().to_string(),
            }
        })
        .collect();
    CategorySeen { name, tag, entries }
}

/// The tree's rows for `copy` and the categories open, made again only as
/// either changes.
fn rows(view: &mut JournalView, copy: &Seen) -> Arc<Vec<Row>> {
    if let Some((r, open, rows)) = &view.rows
        && *r == copy.revision
        && *open == view.open
    {
        return rows.clone();
    }
    let mut rows = vec![Row::Root];
    if copy.seen.is_empty() {
        rows.push(Row::Empty);
    }
    for (c, cat) in copy.seen.iter().enumerate() {
        rows.push(Row::Category(c));
        if view.open.contains(&c) {
            rows.extend((0..cat.entries.len()).map(|e| Row::Entry(c, e)));
        }
    }
    let rows = Arc::new(rows);
    view.rows = Some((copy.revision, view.open.clone(), rows.clone()));
    rows
}

/// The places of the journal's objects the view holds (the selection, the
/// categories open, the panels' fields' ids), for `blueprint::follow` to
/// follow through the edits about to run.
pub(crate) fn held_places(app: &Moonglow) -> Vec<GffPath> {
    let view = &app.journal_view;
    let mut places: Vec<GffPath> = view.ids.keys().cloned().collect();
    places.extend(view.selected.map(Node::path));
    places.extend(view.open.iter().map(|c| category_path(*c)));
    places.sort_by(|a, b| a.0.cmp(&b.0));
    places.dedup();
    places
}

/// The view's objects pointed at where the edits just run moved them
/// (`moved`: each place that moved, and where to, `None` gone): the
/// selection, the categories open, the panels' fields' ids.
pub(crate) fn retarget(app: &mut Moonglow, moved: &HashMap<GffPath, Option<GffPath>>) {
    if moved.is_empty() {
        return;
    }
    let view = &mut app.journal_view;
    let now = |p: GffPath| match moved.get(&p) {
        Some(to) => to.clone(),
        None => Some(p),
    };
    if let Some(n) = view.selected {
        view.selected = now(n.path()).as_ref().and_then(Node::at);
        if view.selected.is_none() || view.selected != Some(n) {
            view.reveal = view.selected.is_some();
        }
    }
    view.open = std::mem::take(&mut view.open)
        .into_iter()
        .filter_map(|c| match now(category_path(c)).as_ref().and_then(Node::at) {
            Some(Node::Category(c)) => Some(c),
            _ => None,
        })
        .collect();
    view.ids = std::mem::take(&mut view.ids)
        .into_iter()
        .filter_map(|(p, id)| Some((now(p)?, id)))
        .collect();
    // What is typed into a panel's field and not yet set goes with its
    // object, or, its object gone (or not to be told in a journal written
    // whole), is dropped: never set on what is at its place now (it would
    // be, were its field not drawn again: \`Moonglow::held_text\`).
    let journal = |h: &&mut crate::blueprint::HeldText| {
        h.window.as_ref().is_some_and(|w| w.tab == Tab::Journal)
    };
    let mut gone = None;
    if let Some(held) = app.held_text.as_mut().filter(journal)
        && let Some(command) = held.command.as_mut()
    {
        for e in &mut command.edits {
            if let Edit::SetField { key: k, path, .. } = e
                && *k == key()
                && let Some(to) = moved.get(path)
            {
                match to {
                    Some(to) => *path = to.clone(),
                    None => gone = Some(held.id),
                }
            }
        }
    }
    if let Some(id) = gone {
        app.held_text = None;
        app.buffers.remove(&id);
    }
}

/// The id the panels keep the fields of the object at `path` by: given on
/// first asking, kept as the object moves ([`retarget`]).
fn object_id(app: &mut Moonglow, path: &GffPath) -> egui::Id {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let view = &mut app.journal_view;
    // (Those of objects not chosen any more go: the chosen and its
    // category's alone are drawn.)
    let keep: Vec<GffPath> =
        view.selected.map(|n| vec![n.path(), category_path(n.category())]).unwrap_or_default();
    if view.ids.len() > 8 {
        view.ids.retain(|p, _| keep.contains(p));
    }
    *view.ids.entry(path.clone()).or_insert_with(|| {
        egui::Id::new(("journal-object", NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)))
    })
}

/// The id a field named `name` of the journal's object at `path` keeps its
/// state by (`blueprint::Form::id` for the journal), where the panels gave
/// the object one.
pub(crate) fn field_id(app: &Moonglow, path: &GffPath) -> Option<egui::Id> {
    app.journal_view.ids.get(path).copied()
}

/// What the header asked for.
#[derive(Default)]
struct Asked {
    add: bool,
    copy: bool,
    cut: bool,
    paste: bool,
    delete: bool,
    expand: bool,
    collapse: bool,
    pressed: Option<crate::blueprint::Pressed>,
}

/// The journal editor (the Journal board's editor): its header over its
/// tree. Its fields are the Right Dock's Entry and Category, drawn before
/// it (their changes noted for its Cancel as they are made,
/// `blueprint::panel_drawn`).
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let targets = [(key(), GffPath::root())];
    let pending = app.actions.len();
    let pressed = editor(app, ui);
    crate::blueprint::window_drawn(app, pending, &Tab::Journal, &targets, pressed);
}

/// The header (the Journal board's `.editor-header`).
fn header(ui: &mut Ui, view: &JournalView, copy: &Seen) -> Asked {
    use crate::icons::editor as icon;
    use mg_uikit::header::{EditorHeader, Item};
    let selected = view.selected.is_some();
    let can_add = true;
    let can_paste = matches!(
        (&view.clipboard, view.selected),
        (Some(Clip::Category(_)), _) | (Some(Clip::Entry(_)), Some(_))
    );
    let any = !copy.seen.is_empty();
    let header = EditorHeader::new(JOURNAL_HEADER)
        .left(
            Item::button(icon::ADD, "Add")
                .enabled(can_add)
                .tip("A category, or an entry in the selected category"),
        )
        .left(Item::icon(icon::COPY, "Copy").enabled(selected).tip("Copy the selection"))
        .left(Item::icon(icon::CUT, "Cut").enabled(selected).tip("Cut the selection"))
        .left(
            Item::icon(icon::PASTE, "Paste")
                .enabled(can_paste)
                .tip("Paste what was copied: a category at the end, an entry into the category"),
        )
        .left(Item::icon(icon::DELETE, "Delete").enabled(selected).tip("Delete the selection"))
        .left(Item::separator())
        .left(Item::icon(icon::EXPAND_ALL, "Expand All").enabled(any).tip("Open every category"))
        .left(
            Item::icon(icon::COLLAPSE_ALL, "Collapse All").enabled(any).tip("Close every category"),
        );
    let out =
        crate::widgets::header_with_ok_cancel(ui, header, None, crate::blueprint::OK_CANCEL_TIPS);
    Asked {
        add: out.clicked("Add"),
        copy: out.clicked("Copy"),
        cut: out.clicked("Cut"),
        paste: out.clicked("Paste"),
        delete: out.clicked("Delete"),
        expand: out.clicked("Expand All"),
        collapse: out.clicked("Collapse All"),
        pressed: out.end,
    }
}

fn editor(app: &mut Moonglow, ui: &mut Ui) -> Option<crate::blueprint::Pressed> {
    let Some(copy) = begin(app) else {
        ui.label("No module is open.");
        return None;
    };
    if let Err(e) = &copy.cats {
        ui.colored_label(ui.visuals().error_fg_color, e);
        return None;
    }
    let asked = header(ui, &app.journal_view, &copy);
    let cats = copy.cats();
    let exists = copy.exists;
    let view = &mut app.journal_view;
    let mut actions = Vec::new();
    let copied = |n: Node| match n {
        Node::Category(c) => Clip::Category(cats[c].clone()),
        Node::Entry(c, e) => Clip::Entry(entries(&cats[c])[e].clone()),
    };
    if asked.add {
        match view.selected {
            Some(Node::Category(c) | Node::Entry(c, _)) => {
                let mut list = entries(&cats[c]).to_vec();
                list.push(new_entry(&list));
                view.selected = Some(Node::Entry(c, list.len() - 1));
                actions.push(set_entries("Add journal entry", c, list));
            }
            None => {
                let mut list = cats.to_vec();
                list.push(new_category(&list));
                view.selected = Some(Node::Category(list.len() - 1));
                actions.push(set_categories("Add journal category", exists, list));
            }
        }
        view.reveal = true;
    }
    if asked.copy || asked.cut {
        view.clipboard = view.selected.map(copied);
    }
    if asked.paste {
        match (view.clipboard.clone(), view.selected) {
            (Some(Clip::Category(s)), _) => {
                let mut list = cats.to_vec();
                list.push(s);
                view.selected = Some(Node::Category(list.len() - 1));
                actions.push(set_categories("Paste journal category", exists, list));
            }
            (Some(Clip::Entry(s)), Some(n)) => {
                let c = n.category();
                let mut list = entries(&cats[c]).to_vec();
                list.push(s);
                view.selected = Some(Node::Entry(c, list.len() - 1));
                actions.push(set_entries("Paste journal entry", c, list));
            }
            _ => {}
        }
        view.reveal = true;
    }
    if asked.cut || asked.delete {
        match view.selected {
            Some(Node::Category(c)) => {
                let mut list = cats.to_vec();
                list.remove(c);
                view.selected = (!list.is_empty()).then(|| Node::Category(c.min(list.len() - 1)));
                actions.push(set_categories("Delete journal category", exists, list));
            }
            Some(Node::Entry(c, e)) => {
                let mut list = entries(&cats[c]).to_vec();
                list.remove(e);
                view.selected = Some(if list.is_empty() {
                    Node::Category(c)
                } else {
                    Node::Entry(c, e.min(list.len() - 1))
                });
                actions.push(set_entries("Delete journal entry", c, list));
            }
            None => {}
        }
    }
    if asked.expand {
        view.open = (0..copy.seen.len()).collect();
    }
    if asked.collapse {
        view.open.clear();
        // (Its entry, now out of sight, isn't left selected.)
        if let Some(Node::Entry(c, _)) = view.selected {
            view.selected = Some(Node::Category(c));
        }
    }
    // The category of the entry selected shows it (a new entry, too).
    if let Some(Node::Entry(c, _)) = view.selected {
        view.open.insert(c);
    }
    tree(app, ui, &copy);
    app.actions.extend(actions);
    asked.pressed
}

/// The room over the tree's first row (the board's `.jr-tree`, 4).
const TREE_TOP: f32 = 4.0;

/// The tree (the Journal board's `.jr-tree`): Root, then each category
/// (its arrow, the journal's icon, its name, its tag quiet and, closed,
/// how many entries it has), its entries under it where it is open (an
/// indent guide, the ID, End where it finishes the category, its text cut
/// to the row); only the rows in sight laid out. A click chooses a row, a
/// double click on a category opens or closes it.
fn tree(app: &mut Moonglow, ui: &mut Ui, copy: &Seen) {
    let all = rows(&mut app.journal_view, copy);
    let view = &mut app.journal_view;
    let t = mg_uikit::tokens(ui.ctx());
    let mut chosen = None;
    let mut toggled = None;
    let reveal = std::mem::take(&mut view.reveal);
    let scroll = egui::ScrollArea::vertical().id_salt("journal-tree").auto_shrink(false);
    let pitch = mg_uikit::metrics::ROW_HEIGHT;
    let shown = scroll.show_viewport(ui, |ui, sight| {
        mg_uikit::name_group(ui, JOURNAL_TREE);
        let count = all.len();
        ui.set_height(2.0 * TREE_TOP + count as f32 * pitch);
        let first = (((sight.min.y - TREE_TOP) / pitch).floor().max(0.0) as usize).min(count);
        let last = ((((sight.max.y - TREE_TOP) / pitch).ceil().max(0.0) as usize) + 1).min(count);
        let top = ui.max_rect().top() + TREE_TOP;
        let (left, width) = (ui.max_rect().left(), ui.max_rect().width());
        let row_rect = |n: usize| {
            let y = top + n as f32 * pitch;
            egui::Rect::from_min_size(egui::pos2(left, y), egui::vec2(width, pitch))
        };
        if reveal
            && let Some(n) =
                view.selected.and_then(|s| all.iter().position(|r| row_node(*r) == Some(s)))
        {
            ui.scroll_to_rect(row_rect(n), None);
        }
        for n in first..last {
            let rect = row_rect(n);
            let row = all[n];
            let selected = match row {
                Row::Root => view.selected.is_none(),
                _ => row_node(row) == view.selected && view.selected.is_some(),
            };
            match draw_row(
                ui,
                &t,
                rect,
                row,
                copy,
                selected,
                view.open.contains(&row_category(row)),
            ) {
                RowDone::Chosen => chosen = Some(row),
                RowDone::Toggled => toggled = Some(row_category(row)),
                RowDone::ChosenToggled => {
                    chosen = Some(row);
                    toggled = Some(row_category(row));
                }
                RowDone::Nothing => {}
            }
        }
    });
    view.hovered = ui.rect_contains_pointer(shown.inner_rect);
    if let Some(row) = chosen {
        view.selected = row_node(row);
    }
    if let Some(c) = toggled {
        if !view.open.remove(&c) {
            view.open.insert(c);
        } else if matches!(view.selected, Some(Node::Entry(x, _)) if x == c) {
            // (Its entry, now out of sight, isn't left selected.)
            view.selected = Some(Node::Category(c));
        }
        ui.ctx().request_repaint();
    }
    if chosen.is_some() {
        ui.ctx().request_repaint();
    }
}

fn row_node(row: Row) -> Option<Node> {
    match row {
        Row::Category(c) => Some(Node::Category(c)),
        Row::Entry(c, e) => Some(Node::Entry(c, e)),
        Row::Root | Row::Empty => None,
    }
}

fn row_category(row: Row) -> usize {
    match row {
        Row::Category(c) | Row::Entry(c, _) => c,
        Row::Root | Row::Empty => usize::MAX,
    }
}

enum RowDone {
    Nothing,
    Chosen,
    Toggled,
    ChosenToggled,
}

/// One row of the tree in `rect` (the board's `.jr-row`: 8 at its left,
/// 12 at its right, 6 between its parts).
fn draw_row(
    ui: &mut Ui,
    t: &mg_uikit::Tokens,
    rect: egui::Rect,
    row: Row,
    copy: &Seen,
    selected: bool,
    open: bool,
) -> RowDone {
    const GAP: f32 = 6.0;
    const ARROW: f32 = 14.0;
    let font = egui::TextStyle::Body.resolve(ui.style());
    let small = egui::FontId::proportional(mg_uikit::metrics::SMALL_SIZE);
    let mono = egui::FontId::monospace(12.0);
    let painter = ui.painter_at(rect.intersect(ui.clip_rect()));
    let right = rect.right() - 12.0;
    let id = ui.id().with(("journal-row", row_key(row)));
    if row == Row::Empty {
        let words = "The module has no journal categories.";
        let galley = painter.layout_no_wrap(words.to_string(), font, t.text_3);
        let at =
            egui::pos2(rect.left() + 8.0 + ARROW + GAP, rect.center().y - galley.size().y / 2.0);
        painter.galley(at, galley, t.text_3);
        let r = ui.interact(rect, id, egui::Sense::hover());
        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, words));
        return RowDone::Nothing;
    }
    // (The arrow's button first, so that the keyboard's focus comes to it
    // before the row: the row answers the pointer from after it.)
    let arrow = egui::Rect::from_min_size(
        egui::pos2(rect.left() + 8.0, rect.top() + (rect.height() - 16.0) / 2.0),
        egui::vec2(ARROW, 16.0),
    );
    let mut toggled = false;
    let fill_at = painter.add(egui::Shape::Noop);
    if let Row::Category(c) = row {
        let name = &copy.seen[c].name;
        let (icon, verb, hint) = match open {
            true => (mg_uikit::Icon::ChevronDown, "Collapse", "Close the category"),
            false => (mg_uikit::Icon::ChevronRight, "Expand", "Show its entries"),
        };
        let b = egui::Button::new("").frame(false).min_size(arrow.size());
        let r = ui.put(arrow, b).on_hover_text(hint);
        r.widget_info(|| {
            let named = format!("{verb} {name}");
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &named)
        });
        mg_uikit::say_expanded(&r, open);
        let color = if selected { t.on_fill } else { t.text_3 };
        icon.paint(
            &painter,
            egui::Rect::from_center_size(arrow.center(), egui::Vec2::splat(12.0)),
            color,
        );
        toggled = r.clicked();
    }
    let sense_rect = rect.with_min_x(arrow.right());
    let r = ui.interact(sense_rect, id, egui::Sense::click());
    let fill = match (selected, r.hovered()) {
        (true, _) => Some(t.accent_fill),
        (false, true) => Some(t.hover_row),
        _ => None,
    };
    if let Some(fill) = fill {
        painter.set(fill_at, egui::Shape::rect_filled(rect, 0.0, fill));
    }
    let (text, text_2, text_3) = match selected {
        true => (t.on_fill, t.on_fill, t.on_fill),
        false => (t.text, t.text_2, t.text_3),
    };
    let mut x = arrow.right() + GAP;
    let cy = rect.center().y;
    let put = |words: String, font: egui::FontId, color: egui::Color32, at: f32, most: f32| {
        let mut job = egui::text::LayoutJob::simple_singleline(words, font, color);
        job.wrap = egui::text::TextWrapping::truncate_at_width(most.max(0.0));
        let galley = painter.layout_job(job);
        let w = galley.size().x;
        painter.galley(egui::pos2(at, cy - galley.size().y / 2.0), galley, color);
        w
    };
    match row {
        Row::Root => {
            put("Root".into(), font, text, x, right - x);
        }
        Row::Category(c) => {
            let cat = &copy.seen[c];
            let icon = egui::Rect::from_min_size(egui::pos2(x, cy - 8.0), egui::Vec2::splat(16.0));
            mg_uikit::Icon::Journal.paint(&painter, icon, text_2);
            x = icon.right() + GAP;
            let tag = format!("[{}]", cat.tag);
            let count = match open {
                true => String::new(),
                false => entries_words(cat.entries.len()),
            };
            let measure = |w: &str, f: &egui::FontId| {
                painter.layout_no_wrap(w.to_string(), f.clone(), text).size().x
            };
            let tail = measure(&tag, &small)
                + if count.is_empty() { 0.0 } else { 4.0 + GAP + measure(&count, &small) };
            let name_w = put(cat.name.clone(), font, text, x, right - x - tail - GAP);
            x += name_w + GAP;
            let tag_w = put(tag, small.clone(), text_3, x, right - x);
            x += tag_w + GAP + 4.0;
            if !count.is_empty() {
                put(count, small, text_3, x, right - x);
            }
        }
        Row::Entry(c, e) => {
            let entry = &copy.seen[c].entries[e];
            // The indent guide, under the category's arrow.
            let gx = rect.left() + 8.0 + 7.0 + 0.5;
            let guide = match selected {
                true => t.on_fill.gamma_multiply(0.18),
                false => t.line_soft,
            };
            painter.line_segment(
                [egui::pos2(gx, rect.top()), egui::pos2(gx, rect.bottom())],
                (1.0, guide),
            );
            let id = format!("[{:04}]", entry.id);
            put(id, mono, text_2, x, 48.0);
            x += 48.0 + GAP;
            if entry.end {
                let words = painter.layout_no_wrap("End".into(), small, t.on_status);
                let size = words.size() + egui::vec2(8.0, 2.0);
                let badge = egui::Rect::from_min_size(egui::pos2(x, cy - size.y / 2.0), size);
                painter.rect_filled(badge, 3.0, t.ok);
                painter.galley(badge.min + egui::vec2(4.0, 1.0), words, t.on_status);
            }
            x += 32.0 + GAP;
            put(entry.text.clone(), font, text_2, x, right - x);
        }
        Row::Empty => {}
    }
    r.widget_info(|| {
        // (Its name and tag alone: whether it is open its arrow says.)
        let said = match row {
            Row::Root => "Root".to_string(),
            Row::Category(c) => {
                let cat = &copy.seen[c];
                format!("{} [{}]", cat.name, cat.tag)
            }
            Row::Entry(c, e) => {
                let entry = &copy.seen[c].entries[e];
                match entry.end {
                    true => format!("[{:04}] {}, finishes the category", entry.id, entry.text),
                    false => format!("[{:04}] {}", entry.id, entry.text),
                }
            }
            Row::Empty => String::new(),
        };
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &said)
    });
    match (
        r.clicked() || r.double_clicked(),
        toggled || (r.double_clicked() && matches!(row, Row::Category(_))),
    ) {
        (true, true) => RowDone::ChosenToggled,
        (true, false) => RowDone::Chosen,
        (false, true) => RowDone::Toggled,
        (false, false) => RowDone::Nothing,
    }
}

/// A row's key for its widgets' ids.
fn row_key(row: Row) -> (u8, usize, usize) {
    match row {
        Row::Root => (0, 0, 0),
        Row::Empty => (1, 0, 0),
        Row::Category(c) => (2, c, 0),
        Row::Entry(c, e) => (3, c, e),
    }
}

/// The journal's panels' editor: the journal, where `editor` is its tab.
fn journal_of(editor: Option<&Tab>) -> bool {
    editor == Some(&Tab::Journal)
}

/// What the Entry and Category panels say after their titles (the Journal
/// board's "[0030] of M1Q5_Bertrand" and "M1Q5_Bertrand").
pub(crate) fn panel_detail(app: &Moonglow, panel: crate::shell::Panel) -> Option<String> {
    let view = &app.journal_view;
    let copy = view.copy.as_ref()?;
    let n = view.selected?;
    let cat = copy.seen.get(n.category())?;
    match (panel, n) {
        (crate::shell::Panel::Entry, Node::Entry(_, e)) => {
            Some(format!("[{:04}] of {}", cat.entries.get(e)?.id, cat.tag))
        }
        (crate::shell::Panel::Category, _) => Some(cat.tag.clone()),
        _ => None,
    }
}

/// The room round a panel's body (the board's `.content`: 8 at its left,
/// top and foot, 4 at its right before its scroll bar).
const PANEL_MARGIN: egui::Margin = egui::Margin { left: 8, right: 12, top: 8, bottom: 8 };

/// The width of the panels' labels (the board's `--lbl-w`, 96).
const LABEL_WIDTH: f32 = 96.0;

/// A panel begins: a press in it gives the panel the keys (F6 outlined),
/// whether F6 came to it, and the journal as it is. `None`: not the
/// journal's.
fn begin_panel(
    app: &mut Moonglow,
    ui: &mut Ui,
    editor: Option<&Tab>,
    keys: crate::palette_view::Keys,
) -> Option<(Arc<Seen>, bool)> {
    if !journal_of(editor) {
        return None;
    }
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        crate::palette_view::give_keys(ui.ctx(), keys);
    }
    let focus = app.dialog_panel_focus.take_if(|k| *k == keys).is_some();
    let copy = begin(app)?;
    Some((copy, focus))
}

/// What a panel says with nothing for it to show.
fn say(ui: &mut Ui, words: &str) {
    egui::Frame::new().inner_margin(PANEL_MARGIN).show(ui, |ui| {
        ui.weak(words);
    });
}

/// A labeled row of a panel's form.
fn form_row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(LABEL_WIDTH, mg_uikit::metrics::CONTROL_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(LABEL_WIDTH);
                if !label.is_empty() {
                    crate::widgets::field_label(ui, label);
                }
            },
        );
        add(ui)
    })
    .inner
}

/// The width of a number field (ID, XP): the board's.
const NUMBER_WIDTH: f32 = 76.0;

/// The journal's Entry (the Journal board's Right Dock, `.entry-panel`):
/// the chosen entry's ID, Finish category, and its Text in the language
/// edited, under its heading (the language's name, whose text it is where
/// not its own, the languages button), the text filling the rest. With a category chosen, or nothing, it says
/// so. Its fields keep their state by the entry ([`object_id`]): what is
/// typed follows it.
pub(crate) fn entry_panel(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let keys = crate::palette_view::Keys::JournalEntry;
    let Some((copy, focus)) = begin_panel(app, ui, editor, keys) else { return };
    let Some(Node::Entry(c, e)) = app.journal_view.selected else {
        say(ui, "Choose an entry to edit it. Add with a category chosen adds one.");
        return;
    };
    let Some(entry) = copy.cats().get(c).and_then(|cat| entries(cat).get(e)).cloned() else {
        return;
    };
    let path = entry_path(c, e);
    let object = object_id(app, &path);
    let pending = app.actions.len();
    use jrl::categories::entry_list as f;
    // (Where the Text reaches: the panel's foot, less its margin.)
    let bottom = ui.max_rect().bottom() - f32::from(PANEL_MARGIN.bottom);
    egui::ScrollArea::vertical().id_salt(object.with("entry-scroll")).auto_shrink(false).show(
        ui,
        |ui| {
            egui::Frame::new().inner_margin(PANEL_MARGIN).show(ui, |ui| {
                ui.push_id(object, |ui| {
                    crate::blueprint::narrow_form(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = mg_uikit::metrics::FORM_ROW_GAP;
                        let mut form = Form {
                            app: &mut *app,
                            key: key(),
                            path: path.clone(),
                            also: Default::default(),
                            root: entry,
                            together: Default::default(),
                            own_tab: false,
                        };
                        // (F6: its Text takes the keyboard, as it is drawn.)
                        if focus {
                            let id = form.field_id(f::TEXT.label);
                            ui.memory_mut(|m| m.request_focus(id));
                        }
                        let r = form_row(ui, "ID", |ui| {
                            let ids = 0..=i64::from(u32::MAX);
                            form.number_at_right(ui, "Entry ID", f::ID.label, ids, NUMBER_WIDTH)
                        });
                        note_number(form.app, ui, r.rect);
                        form_row(ui, "", |ui| form.check(ui, "Finish category", f::END.label));
                        form.locstring_memo_to(ui, "Text", f::TEXT.label, bottom);
                    });
                });
            });
        },
    );
    crate::blueprint::panel_drawn(
        app,
        ui.ctx(),
        pending,
        &Tab::Journal,
        &[(key(), GffPath::root())],
    );
}

/// The journal's Category (the Journal board's Right Dock, `.cat-panel`):
/// the chosen category's, or the chosen entry's: its Name (and the
/// languages button), Tag, Priority, XP and Comments. With nothing chosen,
/// it says so.
pub(crate) fn category_panel(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let keys = crate::palette_view::Keys::JournalCategory;
    let Some((copy, focus)) = begin_panel(app, ui, editor, keys) else { return };
    let Some(c) = app.journal_view.selected.map(Node::category) else {
        say(ui, "Choose a category or an entry. Add with nothing chosen adds a category.");
        return;
    };
    let Some(cat) = copy.cats().get(c) else { return };
    // (Its fields alone: its entries are the Entry panel's.)
    let list = jrl::categories::ENTRY_LIST.label;
    let root = Struct {
        id: cat.id,
        fields: cat
            .fields
            .iter()
            .filter(|f| f.label.as_bytes() != list.as_bytes())
            .cloned()
            .collect(),
    };
    let prio = root.read(&jrl::categories::PRIORITY);
    let path = category_path(c);
    let object = object_id(app, &path);
    let pending = app.actions.len();
    use jrl::categories as f;
    // (Where Comments reaches: the panel's foot, less its margin.)
    let bottom = ui.max_rect().bottom() - f32::from(PANEL_MARGIN.bottom);
    egui::ScrollArea::vertical().id_salt(object.with("category-scroll")).auto_shrink(false).show(
        ui,
        |ui| {
            egui::Frame::new().inner_margin(PANEL_MARGIN).show(ui, |ui| {
                ui.push_id(object, |ui| {
                    crate::blueprint::narrow_form(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = mg_uikit::metrics::FORM_ROW_GAP;
                        let mut form = Form {
                            app: &mut *app,
                            key: key(),
                            path: path.clone(),
                            also: Default::default(),
                            root,
                            together: Default::default(),
                            own_tab: false,
                        };
                        // (F6: its Name takes the keyboard, as it is drawn.)
                        if focus {
                            let id = form.field_id(f::NAME.label);
                            ui.memory_mut(|m| m.request_focus(id));
                        }
                        form_row(ui, "Name", |ui| {
                            form.locstring_whole(ui, "Category name", f::NAME.label);
                        });
                        form_row(ui, "Tag", |ui| form.tag(ui, "Category tag", f::TAG.label));
                        form_row(ui, "Priority", |ui| {
                            let all: Vec<u32> = (0..PRIORITIES.len() as u32).collect();
                            let mut pick = prio;
                            crate::widgets::dropdown("jrl-prio")
                                .width(ui.available_width())
                                .selected_text(
                                    PRIORITIES.get(prio as usize).copied().unwrap_or("?"),
                                )
                                .show(ui, &mut pick, &all, |p| PRIORITIES[*p as usize]);
                            if pick != prio {
                                form.set(
                                    "Category priority",
                                    f::PRIORITY.label,
                                    Value::Dword(pick),
                                );
                            }
                        });
                        let r = form_row(ui, "XP", |ui| {
                            let xp = 0..=i64::from(u32::MAX);
                            form.number_at_right(ui, "Category XP", f::XP.label, xp, NUMBER_WIDTH)
                        });
                        note_number(form.app, ui, r.rect);
                        ui.horizontal_top(|ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(LABEL_WIDTH, mg_uikit::metrics::CONTROL_HEIGHT),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_min_width(LABEL_WIDTH);
                                    crate::widgets::field_label(ui, "Comments");
                                },
                            );
                            ui.vertical(|ui| {
                                form.memo_to(ui, "Category comments", f::COMMENT.label, bottom)
                            });
                        });
                    });
                });
            });
        },
    );
    crate::blueprint::panel_drawn(
        app,
        ui.ctx(),
        pending,
        &Tab::Journal,
        &[(key(), GffPath::root())],
    );
}

/// What the Status Bar shows beside the journal (the Journal board's):
/// with the pointer over its tree, Click Select and, where it has
/// categories, Double Click Open or close a category; Undo and Redo by
/// their keys where there is a step to take back or do again; none while
/// a field is typed in, a popover, dialog or menu is open. Its counts: "40
/// categories" and "220 entries".
pub(crate) fn shown(
    app: &Moonglow,
    k: &crate::shell::HintKeys<'_>,
    ctx: &egui::Context,
) -> crate::shell::Shown {
    use crate::keys::Cmd;
    use crate::shell::{Count, Shown};
    use mg_uikit::status::Hint;
    let view = &app.journal_view;
    let mut counts = Vec::new();
    if let Some(copy) = &view.copy
        && copy.cats.is_ok()
    {
        counts.push(Count::new(match copy.seen.len() {
            1 => "1 category".to_string(),
            n => format!("{n} categories"),
        }));
        counts.push(Count::new(entries_words(copy.entry_count())));
    }
    let mut hints = Vec::new();
    let typing = ctx.memory(|m| m.focused().is_some()) && ctx.egui_wants_keyboard_input();
    if crate::shell::keys_blocked(ctx) || mg_uikit::popover::Popover::any_open(ctx) || typing {
        return Shown { hints, counts, note: None };
    }
    if view.hovered {
        hints.push(Hint::keys(["Click"], "Select"));
        if view.copy.as_ref().is_some_and(|c| !c.seen.is_empty()) {
            hints.push(Hint::keys(["Double Click"], "Open or close a category"));
        }
    }
    let ws = app.ws.as_ref();
    if ws.is_some_and(|w| w.can_undo().is_some()) {
        hints.extend(k.hint(Cmd::Undo, "Undo"));
    }
    if ws.is_some_and(|w| w.can_redo().is_some()) && !typing_pending(app) {
        hints.extend(k.hint(Cmd::Redo, "Redo"));
    }
    Shown { hints, counts, note: None }
}

/// Whether something typed into the journal's Entry or Category panel
/// waits to be set (a text field typed in, a number field with the
/// keyboard): no Redo is offered then, in the Top Bar, the Edit menu, by
/// its keys or in the Status Bar's hints (the side editors' rule, M4's U22
/// review): Redo would set what is typed first, a new step, and the step
/// to redo would be gone.
pub(crate) fn typing_pending(app: &Moonglow) -> bool {
    let held = app.held_text.as_ref().is_some_and(|h| {
        h.command.is_some() && h.window.as_ref().is_some_and(|w| w.tab == Tab::Journal)
    });
    held || app.journal_view.number_typed.is_some()
}

/// A number field drawn in `rect` has the keyboard: noted for this frame
/// ([`typing_pending`]).
fn note_number(app: &mut Moonglow, ui: &Ui, rect: egui::Rect) {
    let focused = ui.memory(|m| m.focused()).and_then(|id| ui.ctx().read_response(id));
    if focused.is_some_and(|r| rect.contains_rect(r.rect)) {
        app.journal_view.number_typed = Some(ui.ctx().cumulative_pass_nr());
    }
}

/// "1 entry", "220 entries".
fn entries_words(n: usize) -> String {
    match n {
        1 => "1 entry".to_string(),
        n => format!("{n} entries"),
    }
}
