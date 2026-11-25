//! The Conversation Editor (`.dlg`), as the Conversation board draws it
//! (M4's U14): the tree of NPC lines (red, `[OWNER]` or the speaker's tag)
//! and PC lines (blue), links in gray, under its header (Add, Copy, Cut,
//! Paste, Paste As Link, Delete, Move Up and Down, Expand and Collapse
//! All, the highlights, Scripts, Export, Import Lines…, Test, OK and
//! Cancel); the selected line's fields (its text, Text Appears When…,
//! Actions Taken, Other Actions, Comments) and the conversation's Search
//! in the Right Dock's Node section, its own settings in the Conversation
//! panel under it, or in its window's right pane where it floats. Each
//! change is one undoable command.

use mg_uikit::Controls as _;
use std::collections::HashSet;

use egui::{Color32, RichText, Ui};
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Gff, Struct, Value};
use mg_module::dialog::{
    ANIMATIONS, Branch, Kind, Parent, Renumbering, add_node, copy_branch, is_link, link_index,
    link_lines, links, move_link_to, new_dialog, node, paste_branch, remove, shift_link,
    word_count, words_of,
};
use mg_resman::ResKey;
use mg_schema::{GffValue, StructExt, jrl};

use crate::text::{decode, encode, with_edited_text};
use crate::widgets::{FieldTarget, LocStringEdit, commit_text, resref_field};
use crate::{Action, Moonglow, Tab};

/// A selected row: a link under a parent (or nothing: Root).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub parent: Parent,
    pub pos: usize,
}

/// The editor's state for one conversation.
#[derive(Debug, Clone, Default)]
pub struct DialogView {
    pub selected: Option<Row>,
    pub collapsed: HashSet<(Kind, u32)>,
    /// Highlight lines with a comment, an action, a quest, an animation or a
    /// sound (Aurora's filter buttons).
    pub highlight: [bool; 5],
    pub token_picker: bool,
    /// Bookmarked lines.
    pub bookmarks: Vec<(Kind, u32)>,
    pub search: DialogSearch,
    /// Test mode: the lines spoken, an NPC's and the player's reply in
    /// turn, the last reply last.
    pub test: Option<Vec<(Kind, u32)>>,
    /// Test mode: what each condition script is taken to return (TRUE
    /// unless set), since the editor can't run it.
    pub assume: std::collections::BTreeMap<String, bool>,
    /// The Input Text popup's new line, while it is open.
    pub input: Option<NewLine>,
    /// What a line's right-click menu asked for, done on the next frame
    /// as the header's command of that name does it.
    pub menu: Option<Asked>,
    /// Save Set's name beside a script's parameters, while it is being
    /// typed: where (the tab and the line or link) and the name so far.
    pub param_set_name: Option<(String, String)>,
    /// The Parameter Sets window (Add Set › Manage Sets…), while it's open.
    pub param_sets: Option<crate::widgets::SetsWindow>,
    /// The selected row is to be scrolled to as it is drawn ([`go_to_line`]).
    pub reveal: bool,
    /// The selected row is to be scrolled just into sight as it is drawn
    /// (a step of the arrow keys), not to the middle.
    pub nudge: bool,
    /// How many rows the tree had room for when drawn last: Page Up and
    /// Page Down's step.
    page: usize,
    /// Rows to scroll the tree by as it is drawn, the selection staying
    /// (Ctrl+Up and Ctrl+Down; down is more).
    scroll: i32,
    /// The line the selected row shows, as drawn last: the selection
    /// follows it when the lines are numbered anew.
    selected_line: Option<(Kind, u32)>,
    /// The letters typed in the tree in quick succession, and when the
    /// last was ([`type_ahead`]).
    typed: String,
    typed_at: f64,
    /// A field had the keyboard when the editor was drawn last: the
    /// Escape that takes it away doesn't also take the tree's keys.
    was_typing: bool,
    /// The conversation as drawn last: the workspace's revision then, and
    /// a fingerprint of the conversation ([`fingerprint`]).
    seen: Option<(u64, u64)>,
    /// The conversation as it was then: a change none of [`Change`]s
    /// leads to is followed by its lines ([`Renumbering::by_lines`]).
    last: Option<Gff>,
    /// The changes of the conversation the editor made or saw, by which
    /// what is kept by line number (bookmarks, folded lines) follows its
    /// lines across a delete and its undo and redo.
    changes: Vec<Change>,
    /// What its panels, drawn before the editor in the frame, asked for:
    /// set by the editor, for its Cancel ([`ui`]).
    from_panels: Vec<Action>,
    /// The conversation as the workspace's revision had it, shared by the
    /// editor and its panels until the next revision ([`begin`]).
    copy: Option<(u64, std::sync::Arc<Gff>)>,
    /// Its Data drawn last: the pass, and the line chosen it showed
    /// ([`follow_panels`]).
    data_drawn: Option<(u64, Option<Row>)>,
    /// Its Node section's bar drawn last: the pass, and whether Bookmark
    /// could be pressed ([`node_tools_enabled`]).
    bar_drawn: std::cell::Cell<Option<(u64, bool)>>,
    /// What its keys did as the editor was drawn last, for the Status
    /// Bar's hints (`shell::shown`): its commands' keys (Add, Move up,
    /// Find: the pointer over it or its tree with the keys, nothing typed
    /// in), and its tree's own (`*`: the tree with the keys).
    pub(crate) keys_took: (bool, bool),
    /// Its rows' words as laid out cut to the tree's width ([`cut_words`]),
    /// by what they were cut from and to: those of this frame, and those
    /// of the frame before, which a row takes again where nothing changed.
    cuts: CutWords,
    /// How many times a row's words were laid out anew ([`cut_words`]).
    cuts_made: u64,
    /// Where its tree's rows end on the right, as last drawn.
    rows_right: f32,
    /// Cached flattened open tree rows keyed by `(gff_ptr, seen_fp, collapsed_sig)`.
    flat: Option<FlatCache>,
}

type FlatCache = ((usize, Option<u64>, u64), std::sync::Arc<Vec<FlatRow>>);

/// Flattened descriptor of one visible row in the conversation tree.
#[derive(Debug, Clone, Copy)]
struct FlatRow {
    row: Row,
    kind: Kind,
    index: u32,
    depth: u16,
    linked: bool,
    children: bool,
    open: bool,
}

/// Rows' words laid out ([`DialogView::cuts`]): this frame's, the last's.
type CutWords = (
    std::collections::HashMap<u64, std::sync::Arc<egui::Galley>>,
    std::collections::HashMap<u64, std::sync::Arc<egui::Galley>>,
);

/// A change of a conversation, from one fingerprint to another: how it
/// numbered the lines anew, and what it took from the editor.
#[derive(Debug, Clone)]
struct Change {
    before: u64,
    after: u64,
    /// `None`: the lines kept their numbers.
    renumbered: Option<Renumbering>,
    /// Numbered by its lines found again, not by the editor's own delete:
    /// what it takes away is said in the debug log.
    found: bool,
    /// The bookmarks (with their places among them) and folded lines of
    /// lines it deleted, numbered as before it: undo brings them back.
    bookmarks: Vec<(usize, (Kind, u32))>,
    collapsed: Vec<(Kind, u32)>,
}

/// How many changes a conversation's editor remembers to follow its
/// lines by.
const CHANGES: usize = 256;

/// A conversation's fingerprint: a hash of its bytes. Kept with a
/// module's tabs, so it is the same whatever the toolchain: SipHash-1-3
/// with zero keys (`siphasher`), which is what Rust's `DefaultHasher`
/// gave the records written before, and may not give later.
#[doc(hidden)]
pub fn fingerprint(g: &Gff) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = siphasher::sip::SipHasher13::new();
    g.to_bytes().unwrap_or_default().hash(&mut h);
    h.finish()
}

impl DialogView {
    /// How many times its tree laid out a row's words anew: only where the
    /// words, their font or the tree's width changed, and only for rows in
    /// sight (M5's W4).
    pub fn cuts_made(&self) -> u64 {
        self.cuts_made
    }

    /// Where its tree's rows end on the right as last drawn: the scroll
    /// bar's side, a row's words ending the kit's row padding short of it
    /// (M5's W4).
    pub fn rows_right(&self) -> f32 {
        self.rows_right
    }

    /// The fingerprint of the conversation as drawn last (`None`: not
    /// drawn yet): what a module's kept tabs check theirs against.
    pub(crate) fn content(&self) -> Option<u64> {
        self.seen.map(|s| s.1)
    }

    /// Has what is kept by line number follow the lines, if conversation
    /// `g` (at workspace revision `revision`) changed since it was drawn
    /// last: by the changes between (a delete, its undo or redo, in any
    /// number of steps); a change it doesn't know (made elsewhere, or
    /// further back than it remembers) by the lines found again in it.
    /// The Input Text popup's parent follows too (the popup closes when
    /// its line is gone). Returns the renumberings it followed, in turn,
    /// for what is kept by line number outside the view (the conversation
    /// clipboard, an open String Edit of a line: [`follow_outside`]).
    fn follow(&mut self, key: ResKey, g: &Gff, revision: u64) -> Vec<Renumbering> {
        let mut followed = Vec::new();
        if self.seen.is_some_and(|(r, _)| r == revision) {
            return followed;
        }
        let now = fingerprint(g);
        let before = self.last.replace(g.clone());
        let Some((_, was)) = self.seen.replace((revision, now)) else { return followed };
        if was == now {
            return followed;
        }
        let steps = match self.steps(was, now) {
            Some(steps) => steps,
            None => {
                let map = before.map(|b| Renumbering::by_lines(&b, g));
                self.remember(Change {
                    before: was,
                    after: now,
                    renumbered: map,
                    found: true,
                    bookmarks: Vec::new(),
                    collapsed: Vec::new(),
                });
                vec![(self.changes.len() - 1, true)]
            }
        };
        // The selection, as lines: its parent and the line it shows.
        let mut selection = self.selected.map(|r| (r.parent, self.selected_line));
        let mut renumbered = false;
        for (i, forward) in steps {
            let Some(map) = self.changes[i].renumbered.clone() else { continue };
            renumbered |= !map.is_identity();
            let map = if forward { map } else { map.inverse() };
            let line = |(k, i): (Kind, u32)| map.line(k, i).map(|n| (k, n));
            let mut gone = Vec::new();
            let mut kept = Vec::new();
            for (at, b) in std::mem::take(&mut self.bookmarks).into_iter().enumerate() {
                match line(b) {
                    Some(b) => kept.push(b),
                    None => gone.push((at, b)),
                }
            }
            self.bookmarks = kept;
            let (folded, unfolded): (Vec<_>, Vec<_>) =
                std::mem::take(&mut self.collapsed).into_iter().partition(|c| line(*c).is_some());
            self.collapsed = folded.into_iter().filter_map(line).collect();
            if forward {
                if self.changes[i].found && !(gone.is_empty() && unfolded.is_empty()) {
                    crate::trace::note(format!(
                        "{key}: changed elsewhere; bookmarks {:?} and folded lines {:?} \
                         not found again in it, let go",
                        gone.iter().map(|(_, b)| b).collect::<Vec<_>>(),
                        unfolded
                    ));
                }
                self.changes[i].bookmarks = gone;
                self.changes[i].collapsed = unfolded;
            } else {
                // Undone: what it deleted comes back, where it was.
                let back = std::mem::take(&mut self.changes[i].bookmarks);
                for (at, b) in back {
                    if !self.bookmarks.contains(&b) {
                        self.bookmarks.insert(at.min(self.bookmarks.len()), b);
                    }
                }
                self.collapsed.extend(std::mem::take(&mut self.changes[i].collapsed));
            }
            selection = selection.and_then(|(parent, shown)| {
                let parent = match parent {
                    Parent::Root => Parent::Root,
                    Parent::Node(k, i) => line((k, i)).map(|(k, i)| Parent::Node(k, i))?,
                };
                Some((parent, Some(line(shown?)?)))
            });
            if let Some(test) = &mut self.test {
                // (Played up to the first line it deleted.)
                let to = test.iter().position(|l| line(*l).is_none()).unwrap_or(test.len());
                test.truncate(to - to % 2);
                for l in test.iter_mut() {
                    *l = line(*l).unwrap_or(*l);
                }
            }
            self.search.results.retain_mut(|(k, kind, index, _)| {
                if *k != key {
                    return true;
                }
                line((*kind, *index)).map(|(_, n)| *index = n).is_some()
            });
            // (The line Input Text adds a line under: OK went by its
            // number, under whatever line had it by then.)
            if let Some(Parent::Node(k, i)) = self.input.as_ref().map(|n| n.parent) {
                match line((k, i)) {
                    Some((k, n)) => {
                        if let Some(input) = &mut self.input {
                            input.parent = Parent::Node(k, n);
                        }
                    }
                    None => self.input = None,
                }
            }
            followed.push(map);
        }
        if !renumbered {
            return followed;
        }
        self.selected = selection.and_then(|(parent, shown)| {
            let (_, index) = shown?;
            let pos = links(g, parent).iter().position(|l| link_index(l) == index)?;
            Some(Row { parent, pos })
        });
        followed
    }

    /// The changes that lead from fingerprint `from` to `to`, each forward
    /// (`true`) or undone; `None` if none do.
    fn steps(&self, from: u64, to: u64) -> Option<Vec<(usize, bool)>> {
        let mut came: std::collections::HashMap<u64, (u64, usize, bool)> = Default::default();
        let mut queue = std::collections::VecDeque::from([from]);
        while let Some(at) = queue.pop_front() {
            if at == to {
                let mut out = Vec::new();
                let mut here = to;
                while here != from {
                    let (back, i, forward) = came[&here];
                    out.push((i, forward));
                    here = back;
                }
                out.reverse();
                return Some(out);
            }
            for (i, c) in self.changes.iter().enumerate() {
                for (start, end, forward) in [(c.before, c.after, true), (c.after, c.before, false)]
                {
                    if start == at && end != from && !came.contains_key(&end) {
                        came.insert(end, (at, i, forward));
                        queue.push_back(end);
                    }
                }
            }
        }
        None
    }

    fn remember(&mut self, change: Change) {
        // (One between the same two conversations is now this one: two
        // deletes leaving the same bytes, either of two lines alike, the
        // first undone. Undo and redo go by the one done last.)
        self.changes.retain(|c| (c.before, c.after) != (change.before, change.after));
        if self.changes.len() >= CHANGES {
            self.changes.remove(0);
        }
        self.changes.push(change);
    }
}

/// A row of the tree as it is drawn: Root, or a link under its parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shown {
    /// The row (`None`: Root).
    pub row: Option<Row>,
    /// The line it shows (`None`: Root).
    pub line: Option<(Kind, u32)>,
    /// Where the row it is under is among the rows (`None`: Root's own).
    pub up: Option<usize>,
    /// It owns lines, drawn under it while it is open (a link owns none).
    pub children: bool,
    pub open: bool,
}

struct TreeNode {
    row: Option<Row>,
    line: Option<(Kind, u32)>,
    up: Option<usize>,
    depth: u16,
    linked: bool,
    children: bool,
    open: bool,
}

fn collect_tree_nodes(g: &Gff, collapsed: &HashSet<(Kind, u32)>) -> Vec<TreeNode> {
    fn walk(
        g: &Gff,
        parent: Parent,
        up: usize,
        depth: u16,
        collapsed: &HashSet<(Kind, u32)>,
        path: &mut HashSet<(Kind, u32)>,
        out: &mut Vec<TreeNode>,
    ) {
        let kind = parent.child_kind();
        for (pos, l) in links(g, parent).iter().enumerate() {
            let index = link_index(l);
            if node(g, kind, index).is_none() {
                continue;
            }
            let linked = is_link(l);
            let children = !linked && !links(g, Parent::Node(kind, index)).is_empty();
            let open = !collapsed.contains(&(kind, index));
            out.push(TreeNode {
                row: Some(Row { parent, pos }),
                line: Some((kind, index)),
                up: Some(up),
                depth,
                linked,
                children,
                open,
            });
            let here = out.len() - 1;
            // (A line's own lines are drawn under it once: in a file where
            // several lines own one, not again under each, which grows as
            // two to the power of how deep such lines go.)
            if children && open && path.insert((kind, index)) {
                walk(
                    g,
                    Parent::Node(kind, index),
                    here,
                    depth.saturating_add(1),
                    collapsed,
                    path,
                    out,
                );
            }
        }
    }
    let children = !links(g, Parent::Root).is_empty();
    let mut out = vec![TreeNode {
        row: None,
        line: None,
        up: None,
        depth: 0,
        linked: false,
        children,
        open: true,
    }];
    walk(g, Parent::Root, 0, 1, collapsed, &mut HashSet::new(), &mut out);
    out
}

/// The rows of the tree from top to bottom as the tree draws them: Root,
/// then each line whose parents are open.
pub fn shown_rows(g: &Gff, collapsed: &HashSet<(Kind, u32)>) -> Vec<Shown> {
    collect_tree_nodes(g, collapsed)
        .into_iter()
        .map(|n| Shown { row: n.row, line: n.line, up: n.up, children: n.children, open: n.open })
        .collect()
}

fn collapsed_sig(collapsed: &HashSet<(Kind, u32)>) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut xor = (collapsed.len() as u64).wrapping_mul(0x9e3779b97f4a7c15);
    for &(k, i) in collapsed {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (k as u8, i).hash(&mut h);
        xor ^= h.finish();
    }
    xor
}

fn flat_rows(view: &mut DialogView, g: &std::sync::Arc<Gff>) -> std::sync::Arc<Vec<FlatRow>> {
    let key = (
        std::sync::Arc::as_ptr(g) as usize,
        view.seen.map(|s| s.1),
        collapsed_sig(&view.collapsed),
    );
    if let Some((was, rows)) = &view.flat
        && *was == key
    {
        return rows.clone();
    }
    let rows: Vec<FlatRow> = collect_tree_nodes(g, &view.collapsed)
        .into_iter()
        .skip(1)
        .map(|n| {
            let row = n.row.expect("non-root node has row");
            let (kind, index) = n.line.expect("non-root node has line");
            FlatRow {
                row,
                kind,
                index,
                depth: n.depth,
                linked: n.linked,
                children: n.children,
                open: n.open,
            }
        })
        .collect();
    let rows = std::sync::Arc::new(rows);
    view.flat = Some((key, rows.clone()));
    rows
}

/// What a key does in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeStep {
    /// The row at this place among the rows is selected.
    To(usize),
    /// The line is opened out (`true`) or folded.
    Fold((Kind, u32), bool),
    None,
}

/// The keys of the tree, as Windows' tree control has them (Aurora's
/// conversation tree is one): Up and Down go to the row above or below,
/// Home and End to the first and the last, Page Up and Page Down a page
/// (`page` rows) up or down; Right opens a line out, or goes to its first
/// line if it is open; Left folds an open line, or goes to the line it is
/// under; Backspace goes to the line it is under; the keypad's + opens a
/// line out and − folds it, going nowhere. `at` is where the selected row
/// is among `rows`.
pub fn tree_step(rows: &[Shown], at: usize, key: egui::Key, page: usize) -> TreeStep {
    use egui::Key;
    let Some(last) = rows.len().checked_sub(1) else { return TreeStep::None };
    let at = at.min(last);
    let here = rows[at];
    let to = match key {
        Key::ArrowDown => at + 1,
        Key::ArrowUp => at.saturating_sub(1),
        Key::Home => 0,
        Key::End => last,
        Key::PageDown => at + page.max(1),
        Key::PageUp => at.saturating_sub(page.max(1)),
        Key::ArrowRight => match here.line {
            Some(line) if here.children && !here.open => return TreeStep::Fold(line, true),
            // (Its first line, if it is drawn under it: not where a
            // line is under itself.)
            _ if here.children && rows.get(at + 1).is_some_and(|r| r.up == Some(at)) => at + 1,
            _ => return TreeStep::None,
        },
        Key::ArrowLeft => match (here.line, here.up) {
            (Some(line), _) if here.children && here.open => return TreeStep::Fold(line, false),
            (_, Some(up)) => up,
            _ => return TreeStep::None,
        },
        Key::Backspace => match here.up {
            Some(up) => up,
            None => return TreeStep::None,
        },
        Key::Plus | Key::Minus => {
            let open = key == Key::Plus;
            return match here.line {
                Some(line) if here.children && here.open != open => TreeStep::Fold(line, open),
                _ => TreeStep::None,
            };
        }
        _ => return TreeStep::None,
    };
    let to = to.min(last);
    if to == at { TreeStep::None } else { TreeStep::To(to) }
}

/// The row a letter typed goes to, as in Windows' tree control: the next
/// after `at` (round the end, back to itself) whose text (`texts`, a row's
/// each) begins with it, in either case.
pub fn seek(texts: &[String], at: usize, letter: char) -> Option<usize> {
    let n = texts.len();
    let same = |c: char| c.to_lowercase().eq(letter.to_lowercase());
    (1..=n).map(|by| (at + by) % n).find(|i| texts[*i].chars().next().is_some_and(same))
}

/// How long a pause in typing starts the letters typed over (seconds).
pub const TYPE_AHEAD: f64 = 1.0;

/// The row the letters typed in quick succession (`typed`) go to, as in
/// Windows' tree control: one letter, or the same letter again and again,
/// the next row after `at` beginning with it ([`seek`]: round the rows
/// beginning with it); else the first from `at` on (itself first, round
/// the end) whose text begins with them all, in either case.
pub fn type_ahead(texts: &[String], at: usize, typed: &str) -> Option<usize> {
    let first = typed.chars().next()?;
    let lower = |c: char| c.to_lowercase().collect::<String>();
    if typed.chars().all(|c| lower(c) == lower(first)) {
        return seek(texts, at, first);
    }
    let typed: String = typed.chars().map(lower).collect();
    let n = texts.len();
    let begins = |t: &String| t.chars().map(lower).collect::<String>().starts_with(&typed);
    (0..n).map(|by| (at + by) % n).find(|i| begins(&texts[*i]))
}

/// The lines of line `top`'s branch, itself included, through the links
/// that own them (every line, from Root: `None`): what the keypad's *
/// opens out.
pub fn branch_lines(g: &Gff, top: Option<(Kind, u32)>) -> HashSet<(Kind, u32)> {
    let mut stack = match top {
        Some(line) => vec![line],
        None => links(g, Parent::Root).iter().map(|l| (Kind::Entry, link_index(l))).collect(),
    };
    let mut seen = HashSet::new();
    while let Some((k, i)) = stack.pop() {
        if seen.insert((k, i)) {
            let owned = links(g, Parent::Node(k, i)).iter().filter(|l| !is_link(l));
            stack.extend(owned.map(|l| (k.child(), link_index(l))));
        }
    }
    seen
}

/// The row to select in place of `selected` once lines are folded
/// (`collapsed`): itself while it is drawn, else the nearest line it is
/// under that is, as Windows' tree control selects the line folded over
/// the selection. `None` is Root.
pub fn in_sight(g: &Gff, collapsed: &HashSet<(Kind, u32)>, selected: Option<Row>) -> Option<Row> {
    let shown = shown_rows(g, collapsed);
    if shown.iter().any(|r| r.row == selected) {
        return selected;
    }
    let every = shown_rows(g, &HashSet::new());
    let mut at = every.iter().position(|r| r.row == selected)?;
    while let Some(up) = every[at].up {
        if shown.iter().any(|r| r.row == every[up].row) {
            return every[up].row;
        }
        at = up;
    }
    None
}

/// The keys of the tree that [`tree_step`] reads.
const TREE_KEYS: [egui::Key; 11] = [
    egui::Key::ArrowDown,
    egui::Key::ArrowUp,
    egui::Key::ArrowRight,
    egui::Key::ArrowLeft,
    egui::Key::Home,
    egui::Key::End,
    egui::Key::PageDown,
    egui::Key::PageUp,
    egui::Key::Backspace,
    egui::Key::Plus,
    egui::Key::Minus,
];

/// Gives the keys to conversation `key`'s tree (from a click in its
/// editor), as a click gives them to the module tree or a table: they are
/// its from then on, until another list or an area's view takes them.
fn take_keys(ctx: &egui::Context, key: ResKey) {
    crate::palette_view::give_keys(ctx, crate::palette_view::Keys::Dialog);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("dialog-keys-of"), key));
}

/// Whether conversation `key`'s tree has the keys.
pub(crate) fn has_keys(ctx: &egui::Context, key: ResKey) -> bool {
    crate::palette_view::keys_are(ctx, crate::palette_view::Keys::Dialog)
        && ctx.data(|d| d.get_temp::<ResKey>(egui::Id::new("dialog-keys-of"))) == Some(key)
}

/// What was pressed or typed for the tree.
#[derive(Debug, Clone)]
enum Pressed {
    Key(egui::Key),
    /// Letters (or other characters) typed; `*` opens everything out.
    Typed(String),
}

/// The character a key types, if it is one a command can have and the
/// tree also reads typed: a letter, a digit, + or −.
fn typed_by(key: egui::Key) -> Option<char> {
    match key {
        egui::Key::Plus => Some('+'),
        egui::Key::Minus => Some('-'),
        k => {
            let mut chars = k.name().chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => c.to_lowercase().next(),
                _ => None,
            }
        }
    }
}

/// The tree's keys pressed this frame (with no Shift, Ctrl or Alt: Alt+Up
/// and Alt+Down move a line), and the letters typed, each done in turn;
/// Ctrl+Up and Ctrl+Down scroll the tree a row. A key that a conversation
/// command was given in Options › Keyboard (Delete Line on Backspace) is
/// the command's, with the character it types.
fn tree_keys(
    ui: &Ui,
    game: Option<&mg_rules::GameData>,
    g: &Gff,
    view: &mut DialogView,
    keymap: &crate::keys::Keymap,
    names: bool,
) {
    use crate::keys::Group;
    for (arrow, by) in [(egui::Key::ArrowUp, -1), (egui::Key::ArrowDown, 1)] {
        while ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, arrow)) {
            view.scroll += by;
            ui.ctx().request_repaint();
        }
    }
    let command = |e: &egui::Event| keymap.bound_in(Group::Conversation, e);
    loop {
        // (A space goes on with letters being typed, as in "[OWNER] - G";
        // it starts none.)
        let now = ui.input(|i| i.time);
        let going_on = !view.typed.is_empty() && now - view.typed_at <= TYPE_AHEAD;
        let pressed = ui.input_mut(|i| {
            let mut found = None;
            // (What the commands' keys type.)
            let commanded: Vec<char> = (i.events.iter())
                .filter_map(|e| match e {
                    egui::Event::Key { key, .. } if command(e) => typed_by(*key),
                    _ => None,
                })
                .collect();
            i.events.retain(|e| match e {
                egui::Event::Key { key, pressed: true, modifiers, .. }
                    if found.is_none()
                        && modifiers.is_none()
                        && TREE_KEYS.contains(key)
                        && !command(e) =>
                {
                    found = Some(Pressed::Key(*key));
                    false
                }
                egui::Event::Text(t) if found.is_none() => {
                    let mut lower = t.chars().flat_map(char::to_lowercase);
                    let not_typed = |(n, c): (usize, char)| {
                        c.is_whitespace() && (c != ' ' || (n == 0 && !going_on))
                    };
                    if t.is_empty()
                        || t.chars().enumerate().any(not_typed)
                        || lower.any(|c| commanded.contains(&c))
                    {
                        return true;
                    }
                    found = Some(Pressed::Typed(t.clone()));
                    false
                }
                _ => true,
            });
            // (The keypad's + and − type them too: not letters typed.)
            if let Some(Pressed::Key(k @ (egui::Key::Plus | egui::Key::Minus))) = found {
                let typed = if k == egui::Key::Plus { "+" } else { "-" };
                let text =
                    i.events.iter().position(|e| matches!(e, egui::Event::Text(t) if t == typed));
                if let Some(at) = text {
                    i.events.remove(at);
                }
            }
            // (A letter's key, too: no command has it meanwhile.)
            if let Some(Pressed::Typed(t)) = &found {
                for c in t.chars().flat_map(char::to_lowercase) {
                    let key = i.events.iter().position(|e| {
                        matches!(e, egui::Event::Key { key, pressed: true, .. }
                            if typed_by(*key) == Some(c))
                    });
                    if let Some(at) = key {
                        i.events.remove(at);
                    }
                }
            }
            found
        });
        let Some(pressed) = pressed else { break };
        // (Not egui's too: an arrow moved the focus on from a button Tab
        // had gone to, as well as the selection. The keys are the tree's.)
        ui.memory_mut(|m| {
            m.move_focus(egui::FocusDirection::None);
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
        let rows = shown_rows(g, &view.collapsed);
        // (A selection in a folded branch: from Root.)
        let at = rows.iter().position(|r| r.row == view.selected).unwrap_or(0);
        let page = if view.page == 0 { 10 } else { view.page };
        let step = match pressed {
            Pressed::Key(k) => {
                view.typed.clear();
                tree_step(&rows, at, k, page)
            }
            Pressed::Typed(t) if t == "*" => {
                view.typed.clear();
                let here = rows[at];
                if here.line.is_none() || here.children {
                    let open = branch_lines(g, here.line);
                    view.collapsed.retain(|l| !open.contains(l));
                }
                TreeStep::None
            }
            Pressed::Typed(t) => {
                // Letters typed in quick succession go by all of them, as
                // in Windows' tree: a pause starts over.
                let texts: Vec<String> = rows.iter().map(|r| row_text(game, g, r, names)).collect();
                let mut to = None;
                for c in t.chars() {
                    if now - view.typed_at > TYPE_AHEAD {
                        view.typed.clear();
                    }
                    view.typed.push(c);
                    view.typed_at = now;
                    to = type_ahead(&texts, to.unwrap_or(at), &view.typed).or(to);
                }
                to.map_or(TreeStep::None, TreeStep::To)
            }
        };
        match step {
            TreeStep::To(to) => {
                view.selected = rows[to].row;
                view.nudge = true;
            }
            TreeStep::Fold(line, true) => {
                view.collapsed.remove(&line);
            }
            TreeStep::Fold(line, false) => {
                view.collapsed.insert(line);
            }
            TreeStep::None => {}
        }
        ui.ctx().request_repaint();
    }
}

/// A row's text as the tree draws it, as a letter typed finds it (Windows'
/// tree goes by its rows' text, and Aurora's matches `[OWNER]`: watched):
/// Root's, or [`row_label`].
fn row_text(game: Option<&mg_rules::GameData>, g: &Gff, row: &Shown, names: bool) -> String {
    let Some(r) = row.row else { return "Root".to_string() };
    let Some(link) = links(g, r.parent).get(r.pos) else { return String::new() };
    row_label(game, g, r.parent.child_kind(), link, names)
}

/// The text of the row of a `kind` line's `link`: an NPC line's speaker
/// first (`names`: `[OWNER]` or its tag), then the line's (its first line);
/// `[CONTINUE]` for a player's line with none, and ` [END DIALOGUE]` after
/// one that ends the conversation.
fn row_label(
    game: Option<&mg_rules::GameData>,
    g: &Gff,
    kind: Kind,
    link: &Struct,
    names: bool,
) -> String {
    let index = link_index(link);
    let linked = is_link(link);
    let Some(n) = node(g, kind, index) else { return String::new() };
    let speaker = decode(n.string("Speaker").unwrap_or_default());
    let first = line_text(game, n).lines().next().unwrap_or_default().to_string();
    let mut label = match kind {
        Kind::Entry if !names => first,
        Kind::Entry if speaker.is_empty() => format!("[OWNER] - {first}"),
        Kind::Entry => format!("[{speaker}] - {first}"),
        Kind::Reply if first.is_empty() && !linked => "[CONTINUE]".to_string(),
        Kind::Reply => first,
    };
    if kind == Kind::Reply && !linked && links(g, Parent::Node(kind, index)).is_empty() {
        label.push_str(" [END DIALOGUE]");
    }
    label
}

/// Selects the row where line `(kind, index)` is owned (not a link to
/// it), its parents opened out, and has it scrolled to: a link's Go to
/// Linked (and a double click on it, as Aurora's), a bookmark.
pub(crate) fn go_to_line(view: &mut DialogView, g: &Gff, kind: Kind, index: u32) {
    let Some((parent, pos)) = mg_module::dialog::owner(g, kind, index) else { return };
    go_to_row(view, g, Row { parent, pos });
}

/// Selects a row (a link under its parent), its parents opened out, and
/// has it scrolled to: what a search's result or a reference names. They
/// selected it where it was, under a folded line and out of sight.
pub(crate) fn go_to_row(view: &mut DialogView, g: &Gff, row: Row) {
    let Row { parent, pos } = row;
    let mut up = parent;
    let mut seen = HashSet::new();
    while let Parent::Node(k, i) = up {
        if !seen.insert((k, i)) {
            break;
        }
        view.collapsed.remove(&(k, i));
        match mg_module::dialog::owner(g, k, i) {
            Some((p, _)) => up = p,
            None => break,
        }
    }
    view.selected = Some(Row { parent, pos });
    view.reveal = true;
}

/// A choice of a line's right-click menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    Add,
    Copy,
    Cut,
    Paste,
    PasteLink,
    Delete,
    /// Toggle Bookmark (M5's W4): a bookmark put on the line, or taken off.
    Bookmark,
}

/// A line the Input Text popup is asking for (Options › Conversation
/// Editor, "Show popup when creating a new text entry").
#[derive(Debug, Clone, PartialEq)]
pub struct NewLine {
    pub parent: Parent,
    pub text: String,
    /// Whether its text has been selected for typing over.
    pub shown: bool,
}

/// A row dropped on a line (or the root): moved, or linked with Ctrl.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Drop {
    from: Row,
    onto: Parent,
    link: bool,
    /// Above this child of `onto` (dropped on a line of its own kind),
    /// not at the end.
    before: Option<usize>,
}

/// Find / Replace in conversations.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DialogSearch {
    pub find: String,
    pub replace: String,
    /// Match Case and Whole Word, as the script editor's Find Text has them
    /// (its `backwards` is not offered here).
    pub options: crate::script_tools::SearchOptions,
    /// All conversations in the module, not only this one.
    pub all_files: bool,
    /// Conversation, line, and its text.
    pub results: Vec<(ResKey, Kind, u32, String)>,
    /// Asked for by a key: the Find What field takes the keyboard.
    focus: bool,
    /// Asked for by a key: find again.
    again: bool,
}

/// What Copy and Cut put aside (shared by all conversations, like Aurora's
/// Scrap). It names lines of the conversation it was copied from by their
/// numbers (the copied line, the branch's links out), and follows them as
/// that conversation's bookmarks do ([`follow_outside`]).
#[derive(Debug, Clone, PartialEq)]
pub struct DialogClip {
    pub from: ResKey,
    pub branch: Branch,
    /// The copied line, for Paste As Link; none once it is gone (cut, or
    /// deleted since).
    pub target: Option<(Kind, u32)>,
}

/// What is kept by line number outside a conversation's editor follows
/// its lines whether or not the editor is drawn: String Edit of a line,
/// open over a conversation's tab behind another, wrote its OK by the old
/// number after an Undo elsewhere renumbered the lines (each frame of
/// the window, before its OK is read; nothing is read where the
/// conversation has not changed since its view last looked).
pub(crate) fn follow_unseen(app: &mut Moonglow) {
    let Some(key) = app.loc_edit.as_ref().map(|w| w.target.key) else { return };
    if key.restype != ResType::DLG {
        return;
    }
    let Some(mut view) = app.dialog_views.remove(&key) else { return };
    // (What is typed in the line chosen's fields follows too, as it would
    // as the editor begins: the renumbering is taken here once.)
    let changed = match app.ws.as_mut() {
        Some(ws) if view.seen.is_some_and(|(r, _)| r != ws.revision()) => {
            let revision = ws.revision();
            ws.doc(&key).ok().map(|g| (g.clone(), revision))
        }
        _ => None,
    };
    if let Some((g, revision)) = changed {
        follow_view(app, None, key, &g, revision, &mut view);
    }
    app.dialog_views.insert(key, view);
}

/// Has what is kept by a line's number outside conversation `key`'s view
/// follow its lines across the renumberings the view followed
/// ([`DialogView::follow`]): the conversation clipboard, where it was
/// copied here (its line for Paste As Link, and its branch's links to
/// lines outside it), and an open String Edit of a line's text, which
/// closes when its line is gone. Each wrote to whatever line had the
/// number once an earlier line was deleted, or an Undo put one back.
fn follow_outside(app: &mut Moonglow, key: ResKey, followed: &[Renumbering]) {
    for map in followed.iter().filter(|m| !m.is_identity()) {
        if let Some(clip) = app.dialog_clip.as_mut().filter(|c| c.from == key) {
            clip.target = clip.target.and_then(|(k, i)| Some((k, map.line(k, i)?)));
            clip.branch.renumber(map);
        }
        let edited = app.loc_edit.as_mut().filter(|w| w.target.key == key);
        let line = edited.as_ref().and_then(|w| match w.target.path.0.first() {
            Some(mg_edit::Step::Item(list, i)) => {
                let kind = [Kind::Entry, Kind::Reply].into_iter().find(|k| k.list() == list)?;
                Some((kind, *i))
            }
            _ => None,
        });
        if let (Some(w), Some((kind, i))) = (edited, line) {
            match u32::try_from(i).ok().and_then(|i| map.line(kind, i)) {
                Some(n) => w.target.path.0[0] = mg_edit::Step::Item(kind.list().into(), n as usize),
                None => app.loc_edit = None,
            }
        }
    }
}

/// The ids of the fields the Data panel shows for conversation `key`'s
/// row `row` showing the line `line`, by which what is typed in them is
/// kept until the keyboard leaves them (`widgets::commit_text`,
/// `resref_field`): the text, the speaker's tag, the comment and the
/// link's, the sound, the condition's and the action's scripts; and the
/// two scripts' parameters being typed (kept apart, in the context's
/// data).
fn row_fields(key: ResKey, row: Row, (kind, index): (Kind, u32)) -> ([egui::Id; 7], [egui::Id; 2]) {
    let (link, own) = (link_path(row).to_string(), node_path(kind, index).to_string());
    let typed = [
        egui::Id::new(("dlg-text", key, kind, index)),
        egui::Id::new(("dlg-speaker", key, index)),
        egui::Id::new(("dlg-comment", key, kind, index, "Comment")),
        egui::Id::new(("dlg-comment", key, kind, index, "LinkComment")),
        egui::Id::new(("dlg-sound", key, kind, index)),
        egui::Id::new(("dlg-script", key, "cond", link.clone())),
        egui::Id::new(("dlg-script", key, "act", own.clone())),
    ];
    let params = [
        egui::Id::new(("dlg-params-buf", key, "cond", link)),
        egui::Id::new(("dlg-params-buf", key, "act", own)),
    ];
    (typed, params)
}

/// What is typed in the line chosen's fields follows it when the lines are
/// numbered anew under it (a delete, its undo, a change made elsewhere):
/// its fields' ids are its row's and line's numbers ([`row_fields`]), so
/// what was typed, the text's cursor and the keyboard's focus move to its
/// new ones (`after`), and typing goes on there; where it is gone (no
/// `after`), what was typed goes with it. Never set on the line that took
/// its number (the Node panel's ids follow its line). Only the
/// line chosen's fields are drawn, so only theirs hold what is typed.
fn follow_typed(
    app: &mut Moonglow,
    ctx: Option<&egui::Context>,
    key: ResKey,
    before: Option<(Row, (Kind, u32))>,
    after: Option<(Row, (Kind, u32))>,
) {
    let Some((row, line)) = before else { return };
    let (typed, params) = row_fields(key, row, line);
    let to = after.map(|(row, line)| row_fields(key, row, line));
    let focused = ctx.and_then(|c| c.memory(|m| m.focused()));
    // (All taken first, then put back: a line may move to a number another
    // field of it had.)
    let state = |id: egui::Id| ctx.and_then(|c| egui::text_edit::TextEditState::load(c, id));
    let moved: Vec<_> = typed.iter().map(|id| (app.buffers.remove(id), state(*id))).collect();
    // (The parameters being typed are kept in the context's data: without
    // the context, String Edit following while the editor is behind, none
    // is being typed, and the field drops what it kept there unset when it
    // is drawn again without the keyboard.)
    type Pairs = Vec<(String, String)>;
    let pairs: Option<Vec<Option<Pairs>>> = ctx.map(|c| {
        c.data_mut(|d| {
            let mut removed = Vec::with_capacity(params.len());
            for id in &params {
                removed.push(d.remove_temp::<Pairs>(*id));
            }
            removed
        })
    });
    let Some((typed_to, params_to)) = to else { return };
    for ((from, (text, state)), to) in typed.iter().zip(moved).zip(typed_to) {
        if let Some(text) = text {
            app.buffers.insert(to, text);
        }
        if let (Some(state), Some(ctx)) = (state, ctx) {
            state.store(ctx, to);
        }
        if let Some(ctx) = ctx.filter(|_| focused == Some(*from)) {
            ctx.memory_mut(|m| m.request_focus(to));
        }
    }
    if let (Some(pairs), Some(ctx)) = (pairs, ctx) {
        for (pairs, to) in pairs.into_iter().zip(params_to) {
            if let Some(pairs) = pairs {
                ctx.data_mut(|d| d.insert_temp(to, pairs));
            }
        }
    }
}

/// A line's own text: in the language edited (Options › Language), else,
/// where it has none in it, in another (English first).
fn text(n: &Struct) -> String {
    let Some(ls) = n.locstring("Text") else { return String::new() };
    crate::text::shown_text(ls)
}

/// A line's text as shown: its own (in the language edited), else its talk-table string
/// (the original campaign's lines, and many modules', are all in the game's
/// talk table).
fn line_text(game: Option<&mg_rules::GameData>, n: &Struct) -> String {
    let own = text(n);
    if !own.is_empty() {
        return own;
    }
    n.locstring("Text")
        .filter(|ls| !ls.strref.is_none())
        .and_then(|ls| game?.string(ls.strref))
        .unwrap_or_default()
}

/// The GFF path of a link.
fn link_path(row: Row) -> GffPath {
    match row.parent {
        Parent::Root => GffPath::root().item("StartingList", row.pos),
        Parent::Node(k, i) => GffPath::root().item(k.list(), i as usize).item(k.links(), row.pos),
    }
}

fn node_path(kind: Kind, index: u32) -> GffPath {
    GffPath::root().item(kind.list(), index as usize)
}

fn set(key: ResKey, what: &str, path: GffPath, label: &str, value: Value) -> Action {
    Action::Apply(Command::new(
        what,
        vec![Edit::SetField { key, path, label: label.to_string(), value: Some(value) }],
    ))
}

fn replace(key: ResKey, what: &str, g: &Gff) -> Action {
    Action::Apply(Command::new(what, written(key, g).into_iter().collect()))
}

/// The edit that keeps conversation `key`'s count of words (`NumWords`,
/// which the status line shows and the file keeps) with a line's text
/// changed from `was` to `now`: the count stayed as it was when a line
/// was typed over.
fn words_edit(
    app: &mut Moonglow,
    key: ResKey,
    was: &mg_core::LocString,
    now: &mg_core::LocString,
) -> Option<Edit> {
    let g = app.ws.as_mut()?.doc(&key).ok()?;
    let words = word_count(g).saturating_sub(words_of(was)) + words_of(now);
    Some(Edit::SetField {
        key,
        path: GffPath::root(),
        label: "NumWords".into(),
        value: Some(Value::Dword(words)),
    })
}

/// The edit that makes conversation `key` what `g` is; none where `g`
/// can't be written (the conversation stays as it is). With `data:
/// g.to_bytes().ok()` a write that failed would have been the edit that
/// deletes the conversation.
fn written(key: ResKey, g: &Gff) -> Option<Edit> {
    match g.to_bytes() {
        Ok(data) => Some(Edit::SetResource { key, data: Some(data) }),
        Err(e) => {
            crate::trace::note(format!("{key} could not be written, and is left as it is: {e}"));
            None
        }
    }
}

/// The key/value pairs of a parameter list, in the game's codepage (as
/// [`params_value`] writes them).
fn params(s: &Struct, label: &str) -> Vec<(String, String)> {
    mg_module::dialog::params_in(s, label, crate::text::game_codepage())
}

/// A parameter list, in the game's codepage (as [`params`] reads it).
fn params_value(pairs: &[(String, String)]) -> Value {
    mg_module::dialog::params_value_in(pairs, crate::text::game_codepage())
}

/// The conversation editor's header's name (`mg_uikit::header`; a screen
/// reader's and the tests' group).
pub const CONVERSATION_HEADER: &str = "Conversation header";

/// The name of the Right Dock's section of a conversation's Data and
/// Search, before their tabs (the Conversation board's `.docktab.title`).
pub(crate) const NODE: &str = "Node";

/// The editor's text at its own size (Options › Conversation Editor) on
/// `ui`: every kind of text in it, each as much larger as its lines are;
/// and in the lists and menus opened from it, which take the window's
/// style, until [`usual_text`] puts the window's back. The window's style
/// as it was, where it was changed.
fn larger_text(app: &Moonglow, ui: &mut Ui) -> Option<std::sync::Arc<egui::Style>> {
    let size = app.settings.dialog_font_size?;
    let usual = ui.ctx().global_style();
    let body_size = egui::TextStyle::Body.resolve(ui.style()).size;
    // (Kept by the window's style itself, held: by its address alone,
    // another style made at the same address took the old one's.)
    type Kept = ((u8, std::sync::Arc<egui::Style>, u32), std::sync::Arc<egui::Style>);
    let key = (size, usual.clone(), body_size.to_bits());
    let id = egui::Id::new("dlg-larger-style");
    let same = |was: &(u8, std::sync::Arc<egui::Style>, u32)| {
        was.0 == key.0 && std::sync::Arc::ptr_eq(&was.1, &key.1) && was.2 == key.2
    };
    let scaled: std::sync::Arc<egui::Style> = match ui.ctx().data(|d| d.get_temp::<Kept>(id)) {
        Some((was, style)) if same(&was) => style,
        _ => {
            let by = f32::from(size.clamp(6, 48)) / body_size;
            let mut style = (**ui.style()).clone();
            for font in style.text_styles.values_mut() {
                font.size *= by;
            }
            style.spacing.interact_size.y *= by;
            style.spacing.icon_width *= by;
            let style = std::sync::Arc::new(style);
            ui.ctx().data_mut(|d| d.insert_temp(id, (key, style.clone())));
            style
        }
    };
    ui.set_style(scaled.clone());
    ui.ctx().set_global_style(scaled);
    Some(usual)
}

/// The window's style back after [`larger_text`].
fn usual_text(ui: &Ui, usual: Option<std::sync::Arc<egui::Style>>) {
    if let Some(usual) = usual {
        ui.ctx().set_global_style(usual);
    }
}

/// Whether conversation `key` is only looked at: one of the game's,
/// opened to read (`Workspace::is_viewed`), or held by another builder
/// (`blueprint::looked_at`). Its header's commands that change it and its
/// panels' fields are then read-only, and Close alone ends it.
pub(crate) fn looked_at(app: &Moonglow, key: &ResKey) -> bool {
    let (held, viewed) = crate::blueprint::looked_at(app, key);
    held.is_some() || viewed
}

/// The conversation editor (M4's U14, the Conversation board): its header
/// (OK and Cancel at its end, `blueprint::ok_cancel`'s meaning) over its
/// tree. Its line's fields, its Search and its own settings are the Right
/// Dock's Node and Conversation panels, or its window's right pane where
/// it floats ([`node_data`], [`node_search`], [`conversation_panel`]),
/// drawn before it: what they asked for this frame is set here, after the
/// editor's Cancel starts noting, so that Cancel sets it back too.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let usual = larger_text(app, ui);
    let targets = [(key, mg_edit::GffPath::root())];
    let pending = app.actions.len();
    if let Some(view) = app.dialog_views.get_mut(&key) {
        app.actions.append(&mut view.from_panels);
    }
    let pressed = editor(app, ui, key);
    let tab = crate::Tab::Dialog(key);
    crate::blueprint::window_drawn(app, pending, &tab, &targets, pressed);
    usual_text(ui, usual);
    // (Its window's Outliner unfolded from its strip, over the tree: drawn
    // after the editor, whose view is back in its place by then, at the
    // window's text size, as the Outliner is.)
    crate::outliner::over(app, ui, crate::outliner::Owner::Dialog(key));
}

/// The conversation `key` as it is now, and its view with what is kept
/// by line number followed to its lines ([`follow_view`]): as the editor
/// and each of its panels begin, whichever is drawn first in the frame (a
/// panel, in the Right Dock, before the editor). The view is taken from
/// `Moonglow::dialog_views`, to be put back. The conversation is copied
/// once a revision of the workspace (`DialogView::copy`): the editor and
/// its panels drawn in a frame share the copy, and a frame where nothing
/// changed copies nothing.
fn begin(
    app: &mut Moonglow,
    ctx: &egui::Context,
    key: ResKey,
) -> Result<(std::sync::Arc<Gff>, DialogView), String> {
    let Some(ws) = &mut app.ws else { return Err(String::new()) };
    let revision = ws.revision();
    // (Taken for the frame and put back at its end, not copied.)
    let mut view = app.dialog_views.remove(&key).unwrap_or_default();
    let g = match view.copy.take().filter(|(r, _)| *r == revision) {
        Some((_, g)) => g,
        None => match ws.doc(&key) {
            Ok(g) => std::sync::Arc::new(g.clone()),
            Err(e) => {
                app.dialog_views.insert(key, view);
                return Err(e.to_string());
            }
        },
    };
    view.copy = Some((revision, g.clone()));
    follow_view(app, Some(ctx), key, &g, revision, &mut view);
    Ok((g, view))
}

/// The bookmarks, folded lines and selection of `view` follow their lines,
/// if a delete (or its undo, a change made elsewhere) numbered them anew
/// since it was last looked at ([`DialogView::follow`]), and what is kept
/// by number outside it ([`follow_outside`]) and what is typed in the
/// line chosen's fields ([`follow_typed`]) with them; a selection the
/// conversation no longer has (undone) is let go. Wherever a renumbering
/// is followed (as the editor or a panel begins, or for String Edit while
/// the editor is behind, [`follow_unseen`]): the view takes each but once.
fn follow_view(
    app: &mut Moonglow,
    ctx: Option<&egui::Context>,
    key: ResKey,
    g: &Gff,
    revision: u64,
    view: &mut DialogView,
) {
    let before = view.selected.zip(view.selected_line);
    let followed = view.follow(key, g, revision);
    follow_outside(app, key, &followed);
    view.selected = view.selected.filter(|r| r.pos < links(g, r.parent).len());
    if followed.is_empty() {
        return;
    }
    let after = view.selected.and_then(|r| {
        let line = links(g, r.parent).get(r.pos).map(|l| (r.parent.child_kind(), link_index(l)));
        Some(r).zip(line)
    });
    // (The line drawn last, as followed, is the selection's line now.)
    view.selected_line = after.map(|(_, line)| line);
    if after != before {
        follow_typed(app, ctx, key, before, after);
        // (Save Set's name, kept by the place it was typed at.)
        view.param_set_name = None;
    }
}

/// What the header asked for that the editor does ([`header`]).
#[derive(Default)]
struct HeaderAsked {
    add: bool,
    copy: bool,
    cut: bool,
    paste: bool,
    paste_link: bool,
    delete: bool,
    up: bool,
    down: bool,
    expand: bool,
    collapse: bool,
    export: Option<mg_module::dialog_io::Format>,
    import: bool,
    test: bool,
    /// OK, Cancel or Close, at the header's end.
    pressed: Option<crate::blueprint::Pressed>,
}

/// What the header's commands may do now ([`header`]).
struct HeaderCan {
    add: bool,
    selected: bool,
    paste: bool,
    paste_link: bool,
    up: bool,
    down: bool,
    /// The conversation is only looked at ([`looked_at`]).
    viewed: bool,
}

/// The conversation editor's header (M4's U14, the Conversation board's
/// `.editor-header`): at the left Add with its word, then Copy, Cut,
/// Paste, Paste As Link and Delete, Move Up and Move Down, Expand All and
/// Collapse All as icons; at the right the five highlights (one joined
/// group of icon toggles), the Scripts toggle with its word, a line,
/// Export and its formats, Import Lines… and Test; then OK and Cancel in
/// the end slot (the proposal's ruling 10 of section 15.2), Close alone
/// for a conversation only looked at. What has no room loses its words,
/// then goes into More (`mg_uikit::header`). The highlights and Scripts it
/// sets itself; the rest it asks the editor to do.
fn header(
    app: &mut Moonglow,
    ui: &mut Ui,
    highlight: &mut [bool; 5],
    can: HeaderCan,
) -> HeaderAsked {
    use crate::icons::editor as icon;
    use mg_uikit::header::{EditorHeader, Item, Joined};
    let ctx = ui.ctx().clone();
    let keymap = &app.keymap;
    let add_tip = keymap.titled("Add a line under the selection", crate::keys::Cmd::AddLine, &ctx);
    let delete_tip =
        keymap.titled("Delete the line and its branch", crate::keys::Cmd::DeleteLine, &ctx);
    let edits = !can.viewed;
    let mut scripts = !app.settings.dialog_hide_scripts;
    let mut export = None;
    let [comments, actions, quests, animations, sounds] = highlight;
    let highlights = vec![
        Joined::toggle(icon::COMMENTS, "Comments", comments).tip("Highlight lines with comments"),
        Joined::toggle(icon::ACTIONS, "Actions", actions).tip("Highlight lines with actions"),
        Joined::toggle(icon::QUESTS, "Quests", quests).tip("Highlight lines with quests"),
        Joined::toggle(icon::ANIMATIONS, "Animations", animations)
            .tip("Highlight lines with animations"),
        Joined::toggle(icon::SOUNDS, "Sounds", sounds).tip("Highlight lines with sounds"),
    ];
    let formats = |ui: &mut Ui| {
        mg_uikit::menu_rows(ui);
        for f in mg_module::dialog_io::Format::ALL {
            if ui.button(format!("{}…", f.name())).clicked() {
                export = Some(f);
                ui.close();
            }
        }
    };
    let tips = crate::blueprint::OK_CANCEL_TIPS;
    let close_tip = can.viewed.then_some("Close the conversation");
    let header = EditorHeader::new(CONVERSATION_HEADER)
        .left(Item::button(icon::ADD, "Add").primary().enabled(edits && can.add).tip(add_tip))
        .left(
            Item::icon(icon::COPY, "Copy")
                .enabled(can.selected)
                .tip("Copy the line and its branch"),
        )
        .left(
            Item::icon(icon::CUT, "Cut")
                .enabled(edits && can.selected)
                .tip("Cut the line and its branch"),
        )
        .left(
            Item::icon(icon::PASTE, "Paste")
                .enabled(edits && can.paste)
                .tip("Paste the lines copied under the selection"),
        )
        .left(
            Item::icon(icon::PASTE_LINK, "Paste As Link")
                .enabled(edits && can.paste_link)
                .tip("Link the selected line and the line copied"),
        )
        .left(Item::icon(icon::DELETE, "Delete").enabled(edits && can.selected).tip(delete_tip))
        .left(Item::separator())
        .left(
            Item::icon(mg_uikit::Icon::ArrowUp, "Move Up")
                .enabled(edits && can.up)
                .tip("Move the line up among its parent's (Alt+Up)"),
        )
        .left(
            Item::icon(mg_uikit::Icon::ArrowDown, "Move Down")
                .enabled(edits && can.down)
                .tip("Move the line down among its parent's (Alt+Down)"),
        )
        .left(Item::separator())
        .left(Item::icon(icon::EXPAND_ALL, "Expand All").tip("Show every line under its parent"))
        .left(
            Item::icon(icon::COLLAPSE_ALL, "Collapse All")
                .tip("Fold every line with lines under it"),
        )
        .right(Item::joined("Highlight", highlights))
        .right(
            Item::toggle(Some(icon::SCRIPTS), "Scripts", &mut scripts)
                .tip("Show each line's condition, action, journal update and sound"),
        )
        .right(Item::separator())
        .right(
            Item::menu(Some(icon::EXPORT), "Export", formats)
                .words_icon(true)
                .tip("Export the lines as plain text, CSV, Twine or Ink"),
        )
        .right(
            Item::button(icon::IMPORT, "Import Lines…")
                .enabled(edits)
                .tip("Read back the text, speakers and comments of a CSV export"),
        )
        .right(Item::button(mg_uikit::Icon::Test, "Test").tip(TEST_TIP));
    let out = crate::widgets::header_with_ok_cancel(ui, header, close_tip, tips);
    if scripts == app.settings.dialog_hide_scripts {
        app.settings.dialog_hide_scripts = !scripts;
    }
    HeaderAsked {
        add: out.clicked("Add"),
        copy: out.clicked("Copy"),
        cut: out.clicked("Cut"),
        paste: out.clicked("Paste"),
        paste_link: out.clicked("Paste As Link"),
        delete: out.clicked("Delete"),
        up: out.clicked("Move Up"),
        down: out.clicked("Move Down"),
        expand: out.clicked("Expand All"),
        collapse: out.clicked("Collapse All"),
        export,
        import: out.clicked("Import Lines…"),
        test: out.clicked("Test"),
        pressed: out.end,
    }
}

/// The editor: its header and its tree; OK, Cancel or Close pressed in
/// the header's end slot.
fn editor(app: &mut Moonglow, ui: &mut Ui, key: ResKey) -> Option<crate::blueprint::Pressed> {
    let (g, mut view) = match begin(app, ui.ctx(), key) {
        Ok(begun) => begun,
        Err(e) if e.is_empty() => return None,
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
            return None;
        }
    };
    let viewed = looked_at(app, &key);
    let folds = view.collapsed.len();
    // Opened or brought to the front: the keys are its tree's.
    if app.dialog_keys == Some(key) {
        app.dialog_keys = None;
        take_keys(ui.ctx(), key);
    }
    // The arrow keys and the others of the tree, once a click in the
    // editor or its opening has given it the keys; not while a field is
    // typed in, nor while a list of choices has the keyboard (reached
    // with Tab: its arrows are its own).
    let keys_here = has_keys(ui.ctx(), key);
    let typing = ui.ctx().text_edit_focused();
    let choosing = crate::widgets::choice_has_keys(ui.ctx());
    let tree_has_keys = keys_here
        && !typing
        && !choosing
        && view.input.is_none()
        && !egui::Popup::is_any_open(ui.ctx())
        && !mg_uikit::popover::Popover::any_open(ui.ctx());
    if tree_has_keys {
        // Escape hands them back (to the area's view, as the module tree's
        // does), unless it took the keyboard from a field or closes a
        // window over the editor.
        if !view.was_typing
            && crate::widgets::escape_past_dialogs(ui.ctx())
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            crate::palette_view::give_arrows(ui.ctx(), false);
        } else {
            let names = !app.settings.dialog_hide_names;
            tree_keys(ui, app.game.as_deref(), &g, &mut view, &app.keymap, names);
        }
    }
    let mut actions: Vec<Action> = Vec::new();
    let target_of = |r: Row| (r.parent.child_kind(), link_index(&links(&g, r.parent)[r.pos]));

    // Options › Conversation Editor.
    let popup = !app.settings.dialog_no_text_popup;
    let paste_source_first = app.settings.dialog_paste_source_to_dest;
    let drag_source_first = !app.settings.dialog_drag_dest_to_source;
    let tlk = |strref: u32, english: &str| {
        app.game
            .as_ref()
            .and_then(|g| g.string(mg_core::StrRef(strref)))
            .unwrap_or_else(|| english.to_string())
    };
    let placeholder = tlk(10336, "<< Enter text here >>");
    let prompts = [
        tlk(67057, "Enter what the NPC says next:"),
        tlk(67056, "Enter what the player says next:"),
    ];

    // Keys, with the pointer over the editor or its tree having the keys
    // (Aurora's control with the focus has them; a press in its panels
    // gives them to it too), and no text being typed: Options ›
    // Keyboard's, and the platform's Copy, Cut and Paste.
    // (Not with the pointer alone while another list holds the keys, the
    // Outliner's bookmarks or the module tree: the list keeps them, and
    // Delete there never deletes a line here. M4's U15 review.)
    let other_list = crate::palette_view::a_list_has_the_keys(ui.ctx());
    let here = ((ui.ui_contains_pointer() && !other_list) || keys_here) && !typing;
    view.keys_took = (here, tree_has_keys);
    let keymap = &app.keymap;
    let pressed = |c: crate::keys::Cmd| here && ui.input(|i| keymap.pressed(i, c));
    let asked = view.menu.take();
    let (key_add, key_delete) = (
        pressed(crate::keys::Cmd::AddLine) || asked == Some(Asked::Add),
        pressed(crate::keys::Cmd::DeleteLine) || asked == Some(Asked::Delete),
    );
    let (key_find, key_find_next) =
        (pressed(crate::keys::Cmd::DialogFind), pressed(crate::keys::Cmd::DialogFindNext));
    // (Toggle Bookmark, F5 as the script's: M5's W4, the owner's ruling
    // 28. A bookmark changes no line: a conversation only looked at takes
    // one too.)
    let key_bookmark = pressed(crate::keys::Cmd::DialogBookmark) || asked == Some(Asked::Bookmark);
    let (key_copy, key_cut, key_paste) =
        if here { crate::widgets::copy_cut_paste(ui) } else { (false, false, false) };
    let (key_copy, key_cut, key_paste) = (
        key_copy || asked == Some(Asked::Copy),
        key_cut || asked == Some(Asked::Cut),
        key_paste || asked == Some(Asked::Paste),
    );
    // (Alt + the arrow keys: up and down among the lines of its parent.)
    let key_move = |up: bool| {
        let arrow = if up { egui::Key::ArrowUp } else { egui::Key::ArrowDown };
        here && ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, arrow))
    };
    let (key_up, key_down) = (key_move(true), key_move(false));

    // What the header's commands may do.
    let sel = view.selected;
    let add_parent = match sel {
        None => Some(Parent::Root),
        Some(r) if !is_link(&links(&g, r.parent)[r.pos]) => {
            let (k, i) = target_of(r);
            Some(Parent::Node(k, i))
        }
        Some(_) => None,
    };
    let clip = app.dialog_clip.clone();
    let fits = |p: Parent| clip.as_ref().is_some_and(|c| c.branch.kind == p.child_kind());
    let can_paste = add_parent.is_some_and(fits);
    // Paste As Link: by default the selected line gets a link to the
    // copied one (Link Destination To Source); the other way round, the
    // copied line gets a link to the selected one.
    let selected_line = sel.filter(|r| !is_link(&links(&g, r.parent)[r.pos])).map(target_of);
    let copied = clip.as_ref().filter(|c| c.from == key).and_then(|c| c.target);
    let pair = match (selected_line, copied) {
        (Some(dest), Some(copied)) => {
            let (from, to) = if paste_source_first { (copied, dest) } else { (dest, copied) };
            (from.0.child() == to.0).then_some((from, to))
        }
        _ => None,
    };
    let siblings = sel.map_or(0, |r| links(&g, r.parent).len());
    let can = HeaderCan {
        add: add_parent.is_some(),
        selected: sel.is_some(),
        paste: can_paste,
        paste_link: pair.is_some(),
        up: sel.is_some_and(|r| r.pos > 0),
        down: sel.is_some_and(|r| r.pos + 1 < siblings),
        viewed,
    };
    let mut highlight = view.highlight;
    let h = header(app, ui, &mut highlight, can);
    view.highlight = highlight;
    // (Nothing that changes it, where it is only looked at: the commands
    // set the conversation whole, which the workspace does not refuse.)
    let edits = !viewed;

    if edits
        && (h.add || key_add)
        && let Some(p) = add_parent
    {
        if popup {
            view.input = Some(NewLine { parent: p, text: placeholder, shown: false });
        } else {
            // The new line is selected, its text edited in place.
            let mut ng = Gff::clone(&g);
            add_node(&mut ng, p, "");
            view.selected = Some(Row { parent: p, pos: links(&ng, p).len() - 1 });
            if let Parent::Node(k, i) = p {
                view.collapsed.remove(&(k, i));
            }
            actions.push(replace(key, "Add line", &ng));
        }
    }
    let copy = |r: Row| {
        copy_branch(&g, r.parent, r.pos).map(|b| DialogClip {
            from: key,
            branch: b,
            target: Some(target_of(r)),
        })
    };
    if (h.copy || key_copy) && sel.is_some() {
        app.dialog_clip = sel.and_then(copy);
        if app.dialog_clip.is_some() {
            crate::widgets::mark_clipboard(ui.ctx(), "conversation lines");
        }
    }
    let mut delete = false;
    if edits && (h.cut || key_cut) && sel.is_some() {
        app.dialog_clip = sel.and_then(copy);
        if app.dialog_clip.is_some() {
            crate::widgets::mark_clipboard(ui.ctx(), "conversation lines");
        }
        delete = true;
    }
    if edits
        && (h.paste || key_paste)
        && can_paste
        && let (Some(p), Some(c)) = (add_parent, clip.as_ref())
    {
        let mut ng = Gff::clone(&g);
        if paste_branch(&mut ng, p, &c.branch, c.from == key) {
            view.selected = Some(Row { parent: p, pos: links(&ng, p).len() - 1 });
            // (Under a folded line: opened, so that what was pasted
            // shows.)
            if let Parent::Node(k, i) = p {
                view.collapsed.remove(&(k, i));
            }
            actions.push(replace(key, "Paste lines", &ng));
        }
    }
    if edits
        && (h.paste_link || asked == Some(Asked::PasteLink))
        && let Some((from, to)) = pair
    {
        let mut ng = Gff::clone(&g);
        if link_lines(&mut ng, from, to) {
            actions.push(replace(key, "Paste as link", &ng));
        }
    }
    if edits && (h.delete || key_delete) {
        delete = true;
    }
    if delete && let Some(r) = sel {
        let mut ng = Gff::clone(&g);
        let renumbered = remove(&mut ng, r.parent, r.pos);
        view.selected = None;
        // (Followed as the conversation becomes it.)
        if let Some((_, before)) = view.seen {
            view.remember(Change {
                before,
                after: fingerprint(&ng),
                renumbered: Some(renumbered),
                found: false,
                bookmarks: Vec::new(),
                collapsed: Vec::new(),
            });
        }
        actions.push(replace(key, "Delete line", &ng));
    }
    // Up and down among the lines of its parent: the order the game tries
    // them in (Alt + the arrow keys, too).
    for (up, asked) in [(true, h.up || key_up), (false, h.down || key_down)] {
        let can = sel.is_some_and(|r| if up { r.pos > 0 } else { r.pos + 1 < siblings });
        if edits
            && asked
            && can
            && let Some(r) = sel
        {
            let mut ng = Gff::clone(&g);
            if let Some(pos) = shift_link(&mut ng, r.parent, r.pos, up) {
                view.selected = Some(Row { parent: r.parent, pos });
                let what = if up { "Move line up" } else { "Move line down" };
                actions.push(replace(key, what, &ng));
            }
        }
    }
    if h.expand {
        view.collapsed.clear();
    }
    if h.collapse {
        for k in [Kind::Entry, Kind::Reply] {
            for i in 0..mg_module::dialog::nodes(&g, k).len() as u32 {
                view.collapsed.insert((k, i));
            }
        }
    }
    // Export and import (dialog_io): the lines back by CSV.
    if let Some(f) = h.export {
        let suggested = std::path::PathBuf::from(crate::dialog_export::file_name(key, f));
        if let Some(path) =
            app.dialogs.save_file(crate::dialogs::FileKind::Conversation(f), Some(&suggested))
        {
            // (As the module tree's Export as Ink… and the rest write each
            // of several.)
            let text = crate::dialog_export::text_of(&g, key, f);
            match std::fs::write(&path, text) {
                Ok(()) => app.log.info(format!("Exported {key} to {}", path.display())),
                Err(e) => app.log.error(format!("{}: {e}", path.display())),
            }
        }
    }
    if edits
        && h.import
        && let Some(path) = app.dialogs.open_file(
            crate::dialogs::FileKind::Conversation(mg_module::dialog_io::Format::Csv),
            None,
        )
    {
        let mut ng = Gff::clone(&g);
        let read = std::fs::read_to_string(&path).map_err(|e| e.to_string());
        let game = crate::text::game_table();
        match read.and_then(|csv| mg_module::dialog_io::update_from_csv_in(&mut ng, &csv, game)) {
            Ok(0) => app.log.info(format!("{}: no lines changed", path.display())),
            Ok(n) => {
                app.log.info(format!("{}: {n} line(s) changed", path.display()));
                actions.push(replace(key, "Import lines", &ng));
            }
            Err(e) => app.log.error(format!("{}: {e}", path.display())),
        }
    }
    if h.test {
        view.test = Some(Vec::new());
    }
    // (The line chosen as the frame began, as the other commands take it.)
    if key_bookmark
        && let Some(r) = sel
        && let Some(l) = links(&g, r.parent).get(r.pos)
    {
        toggle_bookmark(&mut view, (r.parent.child_kind(), link_index(l)));
    }
    // Find and Replace, and Find Next (Options › Keyboard): the Node
    // section's Search, brought to the front (`search_forward`).
    if key_find || key_find_next {
        view.search.focus = key_find;
        view.search.again = key_find_next;
        app.dialog_search_forward = Some(key);
    }

    // A floating conversation's Outliner folded to a strip at the tree's
    // left, under the header (M4's U15 review: the pane gives way before
    // the header does), where its window has not the room for its pane.
    let owner = crate::outliner::Owner::Dialog(key);
    if app.outliner.strips_now.contains(&owner) {
        crate::outliner::strip(app, ui, owner);
    }
    // The tree, in the Options' colours.
    let hl = view.highlight;
    let mut dropped: Option<Drop> = None;
    let mut look =
        Look::new(&app.settings, ui.visuals().dark_mode, mg_uikit::tokens(ui.ctx()).text_3);
    look.bookmark_key = app.keymap.label(crate::keys::Cmd::DialogBookmark, ui.ctx());
    // (Each line cut at the tree's right edge, the owner's ruling 29: the
    // tree scrolls up and down only. What was laid out the frame before is
    // taken again where nothing changed: `cut_words`.)
    view.cuts.1 = std::mem::take(&mut view.cuts.0);
    let lines = egui::ScrollArea::vertical().id_salt(("dlg-tree", key)).auto_shrink([false, false]);
    let flat = flat_rows(&mut view, &g);
    let shown = lines.show(ui, |ui| {
        mg_uikit::list_rows(ui);
        view.rows_right = ui.max_rect().right();
        let width = ui.available_width() - RIGHT_ROOM;
        look.least = least_line(width, egui::TextStyle::Body.resolve(ui.style()).size);
        look.levels = tree_levels(ui, width, look.least);
        // Ctrl+Up and Ctrl+Down: a row up or down, the selection staying.
        if view.scroll != 0 {
            let pitch = mg_uikit::row_pitch(ui);
            ui.scroll_with_delta(egui::vec2(0.0, -(view.scroll as f32) * pitch));
            view.scroll = 0;
        }
        let fill_at = ui.painter().add(egui::Shape::Noop);
        let root = ui.selectable_label(view.selected.is_none(), RichText::new("Root").strong());
        if view.selected.is_none() {
            fill_row(ui, fill_at, root.rect);
        }
        if root.clicked() {
            view.selected = None;
        }
        if view.selected.is_none() && std::mem::take(&mut view.nudge) {
            root.scroll_to_me(None);
        }
        let ours = |p: std::sync::Arc<(ResKey, Row)>| (p.0 == key).then_some(p.1);
        if let Some(from) = root.dnd_release_payload::<(ResKey, Row)>().and_then(ours) {
            dropped = Some(Drop { from, onto: Parent::Root, link: link_key(ui), before: None });
        }
        let a11y = ui.ctx().accesskit_node_builder(root.id, |_| ()).is_some();
        if a11y {
            for &fr in flat.iter() {
                draw_flat_row(
                    ui,
                    app.game.as_deref(),
                    key,
                    &g,
                    fr,
                    &mut view,
                    hl,
                    &look,
                    &mut dropped,
                );
            }
        } else {
            let pad = ui.spacing().button_padding;
            let btn_font = egui::TextStyle::Button.resolve(ui.style());
            let row_h = (ui.fonts_mut(|f| f.row_height(&btn_font)) + 2.0 * pad.y)
                .max(ui.spacing().interact_size.y)
                .max(mg_uikit::metrics::ICON_BUTTON);
            let spacing_y = ui.spacing().item_spacing.y;
            let step_y = row_h + spacing_y;
            let start_y = ui.cursor().top();
            let first_vis = (((ui.clip_rect().top() - step_y - start_y) / step_y).floor().max(0.0)
                as usize)
                .min(flat.len());
            let last_vis = (((ui.clip_rect().bottom() + step_y - start_y) / step_y).ceil().max(0.0)
                as usize)
                .min(flat.len());
            let reveal_idx = if view.reveal || view.nudge {
                view.selected.and_then(|sel| flat.iter().position(|fr| fr.row == sel))
            } else {
                None
            };
            let skip = |ui: &mut Ui, n: usize| {
                if n > 0 {
                    ui.allocate_space(egui::vec2(
                        1.0,
                        (n as f32 * row_h + (n - 1) as f32 * spacing_y).max(0.0),
                    ));
                }
            };
            let draw_one = |ui: &mut Ui,
                            fr: FlatRow,
                            view: &mut DialogView,
                            dropped: &mut Option<Drop>| {
                let id_salt = match fr.row.parent {
                    Parent::Root => (0u64, fr.row.pos),
                    Parent::Node(Kind::Entry, i) => (1 + u64::from(i), fr.row.pos),
                    Parent::Node(Kind::Reply, i) => ((1u64 << 32) | u64::from(i), fr.row.pos),
                };
                ui.push_id(id_salt, |ui| {
                    draw_flat_row(ui, app.game.as_deref(), key, &g, fr, view, hl, &look, dropped);
                });
            };
            match reveal_idx {
                Some(idx) if idx < first_vis => {
                    skip(ui, idx);
                    draw_one(ui, flat[idx], &mut view, &mut dropped);
                    skip(ui, first_vis - idx - 1);
                    for &fr in &flat[first_vis..last_vis] {
                        draw_one(ui, fr, &mut view, &mut dropped);
                    }
                    skip(ui, flat.len() - last_vis);
                }
                Some(idx) if idx >= last_vis => {
                    skip(ui, first_vis);
                    for &fr in &flat[first_vis..last_vis] {
                        draw_one(ui, fr, &mut view, &mut dropped);
                    }
                    skip(ui, idx - last_vis);
                    draw_one(ui, flat[idx], &mut view, &mut dropped);
                    skip(ui, flat.len() - idx - 1);
                }
                _ => {
                    skip(ui, first_vis);
                    for &fr in &flat[first_vis..last_vis] {
                        draw_one(ui, fr, &mut view, &mut dropped);
                    }
                    skip(ui, flat.len() - last_vis);
                }
            }
        }
    });

    let pitch = mg_uikit::row_metrics(ui).1;
    view.page = ((shown.inner_rect.height() / pitch) as usize).saturating_sub(1).max(1);
    // A line folded over the selection (its − button, the keypad's −,
    // Collapse All) is selected in its place, as in Windows' tree control:
    // the selection stays in sight, and the keys go on from there.
    if view.collapsed.len() > folds {
        view.selected = in_sight(&g, &view.collapsed, view.selected);
    }
    if keys_here {
        crate::palette_view::keys_outline(ui, shown.inner_rect, crate::palette_view::Keys::Dialog);
    }
    // (A row dragged before the lines changed under it is no row now.)
    let dropped = dropped.filter(|d| d.from.pos < links(&g, d.from.parent).len());
    // A drag moves the line (with its branch) under the line it is dropped
    // on, or above it if that is a line of its own kind (the player's on
    // the player's); with Ctrl it links them instead, by default the dragged line to
    // the other (Link Source To Destination).
    // (Not where it is only looked at: a conversation set whole is not one
    // the workspace refuses.)
    if edits && let Some(d) = dropped {
        let mut ng = Gff::clone(&g);
        let source = target_of(d.from);
        let done = match d.onto {
            Parent::Node(k, i) if d.link => {
                let (from, to) =
                    if drag_source_first { (source, (k, i)) } else { ((k, i), source) };
                link_lines(&mut ng, from, to)
            }
            _ if d.link => false,
            onto => {
                let at = move_link_to(&mut ng, d.from.parent, d.from.pos, onto, d.before);
                if let Some(pos) = at {
                    view.selected = Some(Row { parent: onto, pos });
                    // (Dropped under a folded line: opened, to show it.)
                    if let Parent::Node(k, i) = onto {
                        view.collapsed.remove(&(k, i));
                    }
                }
                at.is_some()
            }
        };
        if done {
            actions.push(replace(key, if d.link { "Link lines" } else { "Move line" }, &ng));
        }
    }
    // The Input Text popup: OK adds the line (the parent stays selected, as
    // in Aurora), Cancel adds nothing.
    if let Some(mut input) = view.input.take() {
        let prompt = &prompts[usize::from(input.parent.child_kind() == Kind::Reply)];
        let (mut ok, mut cancel) = (false, false);
        let window =
            crate::widgets::modal(ui.ctx(), "Input Text").id(egui::Id::new(("dlg-input", key)));
        let footed = window.show_foot(ui.ctx(), |ui| {
            ui.set_width(340.0);
            ui.label(prompt.as_str());
            let id = egui::Id::new(("dlg-input-text", key));
            let field = egui::TextEdit::multiline(&mut input.text).desired_rows(6);
            let out = crate::widgets::text_area(ui, id, field);
            if !input.shown {
                // The placeholder is selected, so typing replaces it.
                let mut state = out.state.clone();
                let all = input.text.chars().count();
                state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(all),
                )));
                state.store(ui.ctx(), id);
                out.response.request_focus();
                input.shown = true;
            }
            (
                (),
                crate::widgets::Foot::new().main("OK", true).tip("Accept changes").cancel_discard(),
            )
        });
        if let Some(footed) = &footed {
            ok = footed.pressed.main();
            cancel = footed.pressed.cancelled();
        }
        if ok {
            let mut ng = Gff::clone(&g);
            let kind = input.parent.child_kind();
            let i = add_node(&mut ng, input.parent, "");
            // (With plain line ends, as Aurora writes a line's text.)
            let text = crate::text::with_loc_line_ends(input.text.trim_end(), "");
            let t = with_edited_text(Default::default(), &text);
            if let Some(n) = ng.root.list_mut(kind.list()).and_then(|l| l.get_mut(i as usize)) {
                n.set("Text", Value::LocString(t.clone()));
            }
            let words = word_count(&ng);
            ng.root.set("NumWords", Value::Dword(words));
            if let Parent::Node(k, i) = input.parent {
                view.collapsed.remove(&(k, i));
            }
            // (Its text set as typed, after: put in the module's talk table
            // first, where Options › Language says so.)
            let text = Edit::SetField {
                key,
                path: node_path(kind, i),
                label: "Text".into(),
                value: Some(Value::LocString(t)),
            };
            if let Some(made) = written(key, &ng) {
                actions.push(Action::ApplyTyped(Command::new("Add line", vec![made, text])));
            }
        } else if !cancel {
            view.input = Some(input);
        }
    }

    // A click in the editor gives its tree the keys: the arrows, and
    // Delete and the others, are this conversation's, no longer the
    // module tree's (whose Delete asked to delete the whole conversation:
    // Sacha's report).
    if ui.ui_contains_pointer() && ui.input(|i| i.pointer.any_pressed()) {
        take_keys(ui.ctx(), key);
    }
    // (Add Set › Manage Sets…, beside a script's parameters.)
    let dir = app.param_set_dir.clone();
    let sets = crate::widgets::PARAMETER_SETS;
    crate::widgets::sets_window(app, ui.ctx(), &mut view.param_sets, &sets, dir);
    view.was_typing = ui.ctx().text_edit_focused();
    view.selected_line = view.selected.and_then(|r| {
        links(&g, r.parent).get(r.pos).map(|l| (r.parent.child_kind(), link_index(l)))
    });
    follow_panels(app, ui.ctx(), key, &view);
    app.dialog_views.insert(key, view);
    app.actions.extend(actions);
    h.pressed
}

/// A tool of the Node section's bar (the Conversation board's, at its
/// end): Bookmark and Test ([`node_tools`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeTool {
    Bookmark,
    Test,
}

/// The Node section's Bookmark's tip.
pub(crate) const BOOKMARK_TIP: &str = "Bookmark the line chosen, or take its bookmark off";

/// [`BOOKMARK_TIP`] with Toggle Bookmark's key after it (Options ›
/// Keyboard), where it has one.
pub(crate) fn bookmark_tip(app: &Moonglow, ctx: &egui::Context) -> String {
    app.keymap.titled(BOOKMARK_TIP, crate::keys::Cmd::DialogBookmark, ctx)
}

/// The conversation in front's commands the Command Palette offers, each
/// with whether it can run now: Toggle Bookmark, where a line is chosen
/// (M5's W4, the owner's ruling 28). None where no conversation is in
/// front.
pub(crate) fn palette_commands(app: &Moonglow) -> Vec<(crate::keys::Cmd, bool)> {
    let Some(Tab::Dialog(key)) = app.front_tab() else { return Vec::new() };
    let chosen = app.dialog_views.get(&key).is_some_and(|v| v.selected.is_some());
    vec![(crate::keys::Cmd::DialogBookmark, chosen)]
}

/// Runs a command of [`palette_commands`] on the conversation in front:
/// done as its editor is drawn next, as a row's right-click menu's.
pub(crate) fn run_palette_command(app: &mut Moonglow, cmd: crate::keys::Cmd) {
    let Some(Tab::Dialog(key)) = app.front_tab() else { return };
    if cmd == crate::keys::Cmd::DialogBookmark
        && let Some(view) = app.dialog_views.get_mut(&key)
    {
        view.menu = Some(Asked::Bookmark);
    }
}

/// Test's tip, on the header and on the Node section's bar: one name,
/// "Test", on both (the coordinator's ruling at U14's review: the board's
/// "Test: click through the conversation" reads as a name and a tip).
const TEST_TIP: &str = "Click through the conversation.";

/// The conversation an editor's panel draws for: its tab's.
fn dialog_of(editor: Option<&Tab>) -> Option<ResKey> {
    match editor {
        Some(Tab::Dialog(key)) => Some(*key),
        _ => None,
    }
}

/// The name of a floating editor's window's Outliner's strip (its fold
/// strip's dock, for a screen reader and the tests): a conversation's
/// (M4's U15) and a table's (U16), `outliner::strip`.
pub const OUTLINER_STRIP: &str = "Window's Outliner";

/// The width of the Node section's tools ([`node_tools`]).
pub(crate) fn node_tools_width(ui: &Ui) -> f32 {
    2.0 * mg_uikit::IconButton::new(mg_uikit::Icon::Bookmark, "").icon_only(true).width(ui)
}

/// Whether the Node section's Bookmark can be pressed for `editor`'s
/// conversation: a line is chosen. (Noted with the pass for the editor's
/// redraw: [`follow_panels`].)
pub(crate) fn node_tools_enabled(app: &Moonglow, editor: Option<&Tab>, pass: u64) -> bool {
    let Some(view) = dialog_of(editor).and_then(|k| app.dialog_views.get(&k)) else {
        return false;
    };
    let chosen = view.selected.is_some();
    view.bar_drawn.set(Some((pass, chosen)));
    chosen
}

/// The Node section's tools at its bar's end (the Conversation board's
/// `.panel-tabs .tools`): Bookmark (where a line is chosen, `bookmark`;
/// `tip`, [`bookmark_tip`]) and Test; the one clicked.
pub(crate) fn node_tools(ui: &mut Ui, bookmark: bool, tip: &str) -> Option<NodeTool> {
    let icon = |icon, name: &str| mg_uikit::IconButton::new(icon, name).icon_only(true);
    let marked = ui
        .add_enabled(bookmark, icon(mg_uikit::Icon::Bookmark, "Bookmark"))
        .on_hover_text(tip)
        .on_disabled_hover_text(tip);
    let test = ui.add(icon(mg_uikit::Icon::Test, "Test")).on_hover_text(TEST_TIP);
    if marked.clicked() {
        Some(NodeTool::Bookmark)
    } else if test.clicked() {
        Some(NodeTool::Test)
    } else {
        None
    }
}

/// What a Node section's tool does for `editor`'s conversation: Bookmark
/// puts a bookmark on the line chosen, or takes it off; Test opens the
/// Conversation Test.
pub(crate) fn node_tool(app: &mut Moonglow, editor: Option<&Tab>, tool: NodeTool) {
    let Some(key) = dialog_of(editor) else { return };
    let target = match &mut app.ws {
        Some(ws) => ws.doc(&key).ok().and_then(|g| {
            let r = app.dialog_views.get(&key)?.selected?;
            let link = links(g, r.parent).get(r.pos)?;
            Some((r.parent.child_kind(), link_index(link)))
        }),
        None => None,
    };
    let Some(view) = app.dialog_views.get_mut(&key) else { return };
    match tool {
        NodeTool::Bookmark => {
            if let Some(t) = target {
                toggle_bookmark(view, t);
            }
        }
        NodeTool::Test => view.test = Some(Vec::new()),
    }
}

/// A bookmark put on `line`, at the end of `view`'s, or taken off it: the
/// Node section's Bookmark, Toggle Bookmark's key, a row's right-click
/// menu and the Command Palette's row alike.
fn toggle_bookmark(view: &mut DialogView, line: (Kind, u32)) {
    match view.bookmarks.iter().position(|b| *b == line) {
        Some(i) => _ = view.bookmarks.remove(i),
        None => view.bookmarks.push(line),
    }
}

/// What a conversation's panels asked for in a frame its editor was not
/// drawn after them (its tab closed meanwhile, its window folded): set as
/// the frame ends, with the frame's other actions, before anything can
/// number its lines anew under it; never kept for a later frame, where it
/// could land on another line. Its Cancel does not set it back: no window
/// drew it. Called as each frame's actions are run.
pub(crate) fn flush_panels(app: &mut Moonglow) {
    for view in app.dialog_views.values_mut() {
        if !view.from_panels.is_empty() {
            app.actions.append(&mut view.from_panels);
        }
    }
}

/// Find (Ctrl+F) or Find Next (F3) pressed in a conversation
/// (`Moonglow::dialog_search_forward`): its Search brought to the front,
/// acted on as the frame begins, before the docks are drawn (the editor,
/// drawn inside the dock area, cannot see the active editor, as a
/// script's Help, `shell::help_forward`). Where it floats, its window's
/// right pane shows it (`Moonglow::window_rights`, for each of the
/// window's tabs: the pane is drawn for the one in front), the main
/// window's Right Dock left as it was; docked, its group takes the focus
/// where another group had it (the keys fired over it), and Search is the
/// Right Dock's panel, the dock unfolded where the builder had folded it.
/// Nothing kept in the settings.
pub(crate) fn search_forward(app: &mut Moonglow) {
    let Some(key) = app.dialog_search_forward.take() else { return };
    let search = crate::shell::Panel::NodeSearch;
    let tab = Tab::Dialog(key);
    let Some(at) = app.dock.find_tab(&tab) else { return };
    if !at.surface.is_main() {
        let tabs: Vec<Tab> = (app.dock.iter_leaves())
            .filter(|(p, _)| p.surface == at.surface)
            .flat_map(|(_, leaf)| leaf.tabs.iter().cloned())
            .collect();
        for t in tabs {
            app.window_rights.insert(t, search);
        }
        return;
    }
    // (Docked in a group without the focus, the pointer over it: its
    // group takes the focus, as a click there gives it, and the Right
    // Dock is its.)
    if app.active_editor() != Some(&tab) {
        app.show_tab(tab.clone());
    }
    if app.active_editor() == Some(&tab) {
        if crate::shell::right_folded(app) {
            app.show_dock_panel(search);
        } else {
            crate::shell::focus_right(app, search);
        }
    } else if let Some(view) = app.dialog_views.get_mut(&key) {
        // (Nowhere to show it: nothing left asked, to take the keyboard
        // later.)
        view.search.focus = false;
        view.search.again = false;
    }
}

/// After the editor `view` is drawn: where one of its panels drawn in
/// this pass (before the editor) showed another line chosen than the one
/// it ends with (a click in the tree, its keys), or the Node section's
/// Bookmark another state, or its Search is to come to the front, the
/// next frame is asked for, once; a panel not drawn asks for nothing (the
/// endless redraw of M4's U12 review must not come back through them).
fn follow_panels(app: &Moonglow, ctx: &egui::Context, key: ResKey, view: &DialogView) {
    let pass = ctx.cumulative_pass_nr();
    let data = view.data_drawn.is_some_and(|(p, was)| p == pass && was != view.selected);
    let bar =
        (view.bar_drawn.get()).is_some_and(|(p, was)| p == pass && was != view.selected.is_some());
    if data || bar || app.dialog_search_forward == Some(key) {
        ctx.request_repaint();
    }
}

/// Whether F6 came to the panel of `keys` (`Moonglow::dialog_panel_focus`):
/// taken by that panel alone, so that another drawn before it leaves it.
fn take_focus(app: &mut Moonglow, keys: crate::palette_view::Keys) -> bool {
    let to = app.dialog_panel_focus == Some(keys);
    if to {
        app.dialog_panel_focus = None;
    }
    to
}

/// What [`begin_panel`] gives a conversation's panel: its key, the
/// conversation, its view, the window's style to put back and whether it
/// is only looked at.
type Begun = (ResKey, std::sync::Arc<Gff>, DialogView, Option<std::sync::Arc<egui::Style>>, bool);

/// What a conversation's panel begins with, for `editor`'s conversation:
/// its key, the conversation, its view ([`begin`]), the editor's larger
/// text on `ui` (the window's style to put back, [`usual_text`]), and
/// whether it is only looked at; a press in it gives the conversation's
/// tree the keys (Delete, Find and the rest are the conversation's, as
/// they were from its bottom pane); F6 there (`keys`) outlined.
fn begin_panel(
    app: &mut Moonglow,
    ui: &mut Ui,
    editor: Option<&Tab>,
    keys: crate::palette_view::Keys,
) -> Option<Begun> {
    let key = dialog_of(editor)?;
    let (g, view) = begin(app, ui.ctx(), key).ok()?;
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        take_keys(ui.ctx(), key);
    }
    let usual = larger_text(app, ui);
    let viewed = looked_at(app, &key);
    Some((key, g, view, usual, viewed))
}

/// A conversation's panel ends: what it asked for kept for the editor to
/// set (its Cancel's: [`ui`]), or, where it is only looked at (`viewed`),
/// refused and said in the log; the view put back, the window's style
/// back.
fn end_panel(
    app: &mut Moonglow,
    ui: &Ui,
    (key, viewed): (ResKey, bool),
    mut view: DialogView,
    actions: Vec<Action>,
    usual: Option<std::sync::Arc<egui::Style>>,
) {
    match viewed {
        true if !actions.is_empty() => {
            app.log.warn(format!("{key} is only looked at: nothing in it was changed"));
        }
        true => {}
        false => view.from_panels.extend(actions),
    }
    app.dialog_views.insert(key, view);
    usual_text(ui, usual);
}

/// The room round a conversation's panel's body (the board's `.content`:
/// 8 at its left, top and foot, 4 at its right before its scroll bar's 8).
const PANEL_MARGIN: egui::Margin = egui::Margin { left: 8, right: 12, top: 8, bottom: 8 };

/// The words of the readout over a conversation's Data (the board's
/// `.readout`: 11 points, `text-3`).
fn readout(ui: &mut Ui, words: String) {
    let size = egui::TextStyle::Small.resolve(ui.style()).size;
    ui.add(egui::Label::new(RichText::new(words).size(size).weak()).wrap());
}

/// The conversation's Data (M4's U14, the Right Dock's Node section's
/// first tab, or its window's right pane): the line chosen, as the
/// Conversation board draws it, its sections on one scroll: Text (Speaker
/// tag, the text with its languages button and Token…), Text Appears
/// When… and Actions Taken (each its Script Wizard…, script, parameters
/// and preview), Other Actions (animation, sound, journal) and Comments;
/// over them how many letters and words the line's text shown has (the
/// talk table's where its text is, the proposal's finding 13.2) and the
/// file's. With Root chosen, what Add adds. Its fields' ids are the
/// line's, so a field typed in never writes to another line as the
/// selection moves.
pub(crate) fn node_data(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let keys = crate::palette_view::Keys::NodeData;
    let Some((key, g, mut view, usual, viewed)) = begin_panel(app, ui, editor, keys) else {
        return;
    };
    let focus = take_focus(app, keys);
    view.data_drawn = Some((ui.ctx().cumulative_pass_nr(), view.selected));
    let mut actions = Vec::new();
    let scroll = egui::ScrollArea::vertical().id_salt(("dlg-node-data", key)).auto_shrink(false);
    scroll.show(ui, |ui| {
        egui::Frame::new().inner_margin(PANEL_MARGIN).show(ui, |ui| {
            ui.set_width(ui.available_width());
            crate::blueprint::read_only_if(ui, viewed, |ui| {
                data_body(app, ui, key, &g, &mut view, &mut actions, (focus, viewed));
            });
        });
    });
    end_panel(app, ui, (key, viewed), view, actions, usual);
}

/// [`node_data`]'s sections.
fn data_body(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    g: &Gff,
    view: &mut DialogView,
    actions: &mut Vec<Action>,
    (focus, viewed): (bool, bool),
) {
    let words = g.root.dword("NumWords").unwrap_or(0);
    let Some(r) = view.selected else {
        readout(ui, format!("Root. File: {words} words. Add adds an NPC greeting."));
        return;
    };
    let link = links(g, r.parent)[r.pos].clone();
    let (kind, index) = (r.parent.child_kind(), link_index(&link));
    let Some(n) = node(g, kind, index).cloned() else { return };
    // (The text shown: the talk table's where the line's is there, as
    // its field's note shows it; finding 13.2.)
    let line = line_text(app.game.as_deref(), &n);
    readout(
        ui,
        format!(
            "Line: {} letters, {} words. File: {words} words.{}",
            line.chars().count(),
            line.split_whitespace().count(),
            if is_link(&link) {
                " This row is a link: the line is edited where it is owned."
            } else {
                ""
            }
        ),
    );
    ui.add_space(2.0);
    // (Save Set's name of another line's, forgotten: the places of this
    // line's condition and action, as `script_with_params` makes them.)
    let places = [format!("cond {}", link_path(r)), format!("act {}", node_path(kind, index))];
    if view.param_set_name.as_ref().is_some_and(|(p, _)| !places.contains(p)) {
        view.param_set_name = None;
    }
    panel_heading(ui, "Text", None);
    text_panel(app, ui, key, kind, index, &n, view, actions, focus);
    for (condition, heading) in [(true, "Text Appears When…"), (false, "Actions Taken")] {
        ui.add_space(mg_uikit::form::SECTION_GAP);
        // (Its Script Wizard… at the heading's end, where the panel has
        // room for both; else on a line of its own under it.)
        // (Measured at the editor's text size, as the heading is drawn;
        // off where the conversation is only looked at: it writes a script
        // and sets it.)
        let mut wizard = false;
        let tip = "Write a new script from a few choices";
        let room = heading_width(ui, heading)
            + mg_uikit::metrics::GAP_M
            + button_room(ui, "Script Wizard…");
        let mut button = |ui: &mut Ui| {
            let b = egui::Button::new("Script Wizard…");
            wizard = ui.add_enabled(!viewed, b).on_hover_text(tip).clicked();
        };
        if room <= ui.available_width() {
            panel_heading(ui, heading, Some(&mut button));
        } else {
            panel_heading(ui, heading, None);
            button(ui);
        }
        let (path, s, label) = match condition {
            true => (link_path(r), &link, "Active"),
            false => (node_path(kind, index), &n, "Script"),
        };
        if wizard && !viewed {
            app.open_script_wizard(key, path.clone(), label, condition);
        }
        script_with_params(app, ui, key, path, condition, s, view, actions);
    }
    ui.add_space(mg_uikit::form::SECTION_GAP);
    panel_heading(ui, "Other Actions", None);
    other_tab(app, ui, key, kind, index, &n, actions);
    ui.add_space(mg_uikit::form::SECTION_GAP);
    panel_heading(ui, "Comments", None);
    let (path, label, value) = if is_link(&link) {
        (link_path(r), "LinkComment", decode(link.string("LinkComment").unwrap_or_default()))
    } else {
        (node_path(kind, index), "Comment", decode(n.string("Comment").unwrap_or_default()))
    };
    let id = egui::Id::new(("dlg-comment", key, kind, index, label));
    let width = ui.available_width();
    if let Some(v) = commit_text(app, ui, id, &value, true, width) {
        actions.push(set(key, "Comment", path, label, Value::String(encode(&v))));
    }
}

/// The conversation's Search (M4's U14, the Node section's second tab):
/// Find and Replace in this conversation or the module's, its results a
/// click going to their lines. (Its bookmarks are the Outliner's since
/// M4's U15: [`bookmark_rows`].)
pub(crate) fn node_search(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let keys = crate::palette_view::Keys::NodeSearch;
    let Some((key, g, mut view, usual, viewed)) = begin_panel(app, ui, editor, keys) else {
        return;
    };
    if take_focus(app, keys) {
        view.search.focus = true;
    }
    let mut actions = Vec::new();
    let scroll = egui::ScrollArea::vertical().id_salt(("dlg-node-search", key)).auto_shrink(false);
    scroll.show(ui, |ui| {
        egui::Frame::new().inner_margin(PANEL_MARGIN).show(ui, |ui| {
            ui.set_width(ui.available_width());
            search_pane(app, ui, key, &g, &mut view, &mut actions, viewed);
        });
    });
    end_panel(app, ui, (key, viewed), view, actions, usual);
}

/// Conversation `key`'s bookmarks as the Outliner lists them (M4's U15,
/// the Conversation board's): each line's kind and number, and its row's
/// words, "Entry 40: Welcome, <FullName>. …" (its kind, its number and the
/// first line of its text shown, the talk table's where the line's is
/// there), in the order they were set; followed to their lines first
/// ([`begin`]: across a delete, its undo, a change made elsewhere). None
/// where the conversation cannot be read. (No stamp for the editor's
/// redraw: what changes them after the Outliner is drawn, the Node bar's
/// Bookmark, is a click, after which egui draws a frame anyway; an edit
/// and an outside change ask for their own. M4's U15 review.)
pub(crate) fn bookmark_rows(
    app: &mut Moonglow,
    ctx: &egui::Context,
    key: ResKey,
) -> Option<Vec<((Kind, u32), String)>> {
    let (g, view) = begin(app, ctx, key).ok()?;
    let rows = (view.bookmarks.iter())
        .filter_map(|&(kind, index)| {
            let n = node(&g, kind, index)?;
            let text = line_text(app.game.as_deref(), n);
            let first = text.lines().next().unwrap_or_default();
            Some(((kind, index), format!("{kind:?} {index}: {first}")))
        })
        .collect();
    app.dialog_views.insert(key, view);
    Some(rows)
}

/// A bookmark of conversation `key` chosen in the Outliner (a click on its
/// row, Enter): its line selected in the tree, the lines over it opened
/// out, and scrolled to ([`go_to_line`]), as the old Bookmarks list did.
pub(crate) fn go_to_bookmark(
    app: &mut Moonglow,
    ctx: &egui::Context,
    key: ResKey,
    (kind, index): (Kind, u32),
) {
    let Ok((g, mut view)) = begin(app, ctx, key) else { return };
    go_to_line(&mut view, &g, kind, index);
    app.dialog_views.insert(key, view);
    ctx.request_repaint();
}

/// The Conversation panel's header's detail (the board's "3834 words in
/// the file"): its words, as the file counts them.
pub(crate) fn words_detail(app: &Moonglow, editor: Option<&Tab>) -> Option<String> {
    let words = file_words(app, dialog_of(editor)?)?;
    Some(format!("{words} words in the file"))
}

/// Conversation `key`'s words, as the file counts them (`NumWords`): the
/// Conversation panel's header and the Status Bar's count.
pub(crate) fn file_words(app: &Moonglow, key: ResKey) -> Option<u32> {
    let ws = app.ws.as_ref()?;
    let g = ws.loaded(&key).or_else(|| ws.viewed_doc(&key))?;
    Some(g.root.dword("NumWords").unwrap_or(0))
}

/// The Conversation panel (M4's U14, under the Node section): the
/// conversation's own settings, Normal end, Aborted and Stop camera zoom
/// in (today's Current File page); its word count in its header
/// ([`words_detail`]), or, sharing the Right Dock's one tab bar under
/// 1400, at its own top.
pub(crate) fn conversation_panel(
    app: &mut Moonglow,
    ui: &mut Ui,
    editor: Option<&Tab>,
    titled: bool,
) {
    let keys = crate::palette_view::Keys::Conversation;
    let Some((key, g, view, usual, viewed)) = begin_panel(app, ui, editor, keys) else {
        return;
    };
    let focus = take_focus(app, keys);
    let mut actions = Vec::new();
    let scroll = egui::ScrollArea::vertical().id_salt(("dlg-file-panel", key)).auto_shrink(false);
    let shown = scroll.show(ui, |ui| {
        egui::Frame::new().inner_margin(PANEL_MARGIN).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !titled {
                let words = g.root.dword("NumWords").unwrap_or(0);
                readout(ui, format!("{words} words in the file"));
            }
            crate::blueprint::read_only_if(ui, viewed, |ui| {
                file_tab(app, ui, key, &g, &mut actions, focus);
            });
        });
    });
    // (Its height as drawn, for its section's least height from the next
    // frame, `conversation_least`: a frame more where it changed, once.)
    let high = shown.content_size.y.ceil();
    if titled && (high - app.dialog_conversation_height).abs() > 0.5 {
        app.dialog_conversation_height = high;
        ui.ctx().request_repaint();
    }
    end_panel(app, ui, (key, viewed), view, actions, usual);
}

/// The Conversation panel's least height in a stacked Right Dock (or
/// window's pane) section: its bar and its three rows (Normal end,
/// Aborted, Stop camera zoom in) at the editor's text size, with its
/// margins: nothing of it cut at rest (the coordinator's ruling at U14's
/// review; `shell::section_least`).
pub(crate) fn conversation_least(app: &Moonglow) -> f32 {
    let body = mg_uikit::metrics::BODY_SIZE;
    let by = app.settings.dialog_font_size.map_or(1.0, |s| f32::from(s.clamp(6, 48)) / body);
    let row = mg_uikit::metrics::CONTROL_HEIGHT * by.max(1.0) + 6.0;
    let margins = f32::from(PANEL_MARGIN.top + PANEL_MARGIN.bottom);
    let rows = (mg_uikit::metrics::DOCK_TABS + 3.0 * row + margins + 4.0).ceil();
    // (Or as tall as it was drawn last, where its rows stood their labels
    // over their fields: the narrow dock at the larger text.)
    let drawn = mg_uikit::metrics::DOCK_TABS + app.dialog_conversation_height + 2.0;
    rows.max(drawn.ceil())
}

/// The least width of a field beside its label in a conversation's panel
/// ([`form_row`]): under it the label stands over the field.
const FIELD_LEAST: f32 = 120.0;
/// A field label's width in a conversation's panel (the board's
/// `--lbl-w: 96px`), or its words' where they are wider (the editor's
/// larger text).
const LABEL_WIDTH: f32 = 96.0;

/// How much larger than the kit's body the text of `ui` is (the
/// conversation editor's larger text, Options › Conversation Editor), at
/// least 1.
fn text_scale(ui: &Ui) -> f32 {
    let body = egui::TextStyle::Body.resolve(ui.style()).size;
    (body / mg_uikit::metrics::BODY_SIZE).max(1.0)
}

/// The width a field needs to show 16 characters of a name (a resref, a
/// tag) in the text size of `ui`: 16 of the letters, digits and underscore
/// a name is written in, as wide as they average, with the field's
/// margins and the cursor's room.
fn name_least(ui: &Ui) -> f32 {
    const CHARS: &str = "abcdefghijklmnopqrstuvwxyz0123456789_";
    let font = egui::TextStyle::Body.resolve(ui.style());
    let all = ui.painter().layout_no_wrap(CHARS.to_string(), font, Color32::WHITE).size().x;
    let margin = mg_uikit::controls::FIELD_MARGIN;
    (all / CHARS.len() as f32 * 16.0 + f32::from(margin.left) + f32::from(margin.right) + 2.0)
        .ceil()
}

/// The room `widgets::resref_field` keeps for its `…` button beside the
/// field, and the gaps.
fn picker_room(ui: &Ui) -> f32 {
    // (A square as high as the field, after the gap.)
    ui.spacing().interact_size.y + 2.0 * ui.spacing().item_spacing.x + 2.0
}

/// A row of a resref field (a script's, a sound's) labeled `label`, with
/// `trailing` points kept after its picker for what follows it (Edit,
/// Play): the field as wide as the row leaves it, never narrower than 16
/// characters of a name need ([`name_least`]); where the row beside the
/// label has not that room, the label stands over the row
/// ([`form_row_least`]). `add` lays out the field (`widgets::resref_field`,
/// given its width) and what follows it.
fn resref_row<R>(ui: &mut Ui, label: &str, trailing: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let slack = ui.spacing().item_spacing.x;
    let need = name_least(ui) + picker_room(ui) + trailing + slack;
    form_row_least(ui, label, need, |ui| {
        let room = ui.available_width();
        let field = (room - picker_room(ui) - trailing - slack).max(name_least(ui));
        // (Where even the whole row has not room for what follows the
        // field, at the larger text in the narrow dock, it goes onto the
        // next line: the field keeps its 16 characters.)
        match field + picker_room(ui) + trailing + slack <= room + 0.5 {
            true => crate::widgets::with_resref_width(ui, field, add),
            false => {
                let field = (room - picker_room(ui) - slack).max(name_least(ui));
                ui.horizontal_wrapped(|ui| crate::widgets::with_resref_width(ui, field, add)).inner
            }
        }
    })
}

/// The width of a button reading `words` after the gap before it.
fn button_room(ui: &Ui, words: &str) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text = ui.painter().layout_no_wrap(words.to_string(), font, Color32::WHITE).size().x;
    text + 2.0 * ui.spacing().button_padding.x + ui.spacing().item_spacing.x + 1.0
}

/// A row of a conversation's panel's form (the board's `.node-form`):
/// `label` in a column of [`LABEL_WIDTH`] at the left, `add` beside it;
/// where the panel is too narrow for the field beside it, the label over
/// it (a row that goes onto two lines when short of room).
fn form_row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    // (A field's least room grows with the editor's larger text.)
    let least = FIELD_LEAST * text_scale(ui);
    form_row_least(ui, label, least, add)
}

/// [`form_row`] whose field and what follows it need `least` points
/// beside the label.
fn form_row_least<R>(ui: &mut Ui, label: &str, least: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let words = ui.painter().layout_no_wrap(label.to_string(), font, Color32::WHITE).size().x;
    let width = LABEL_WIDTH.max((words + 8.0).ceil());
    // (The room the field is given beside the label: the row's, less the
    // label's column and the gap the row puts after it. Tested with the
    // gap left out, the field had 4 points less than was tested, and a
    // resref row then put its Edit onto a line of its own: Data taller
    // where wider, which never came to rest beside its scroll bar. M4's
    // U15 review.)
    let beside = ui.available_width() - width - ui.spacing().item_spacing.x;
    if beside < least {
        if !label.is_empty() {
            crate::widgets::field_label(ui, label);
        }
        return ui.horizontal(add).inner;
    }
    ui.horizontal(|ui| {
        let high = ui.spacing().interact_size.y;
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        ui.allocate_ui_with_layout(egui::vec2(width, high), layout, |ui| {
            ui.set_min_width(width);
            crate::widgets::field_label(ui, label);
        });
        add(ui)
    })
    .inner
}

/// A section's heading in a conversation's panel (the board's `.sec-h`:
/// bold, larger than its labels, a `line-soft` rule under it), at the
/// editor's text size as its labels are (panel content, not chrome: the
/// coordinator's ruling at U14's review), `end` at its right end where
/// given (Script Wizard…).
fn panel_heading(ui: &mut Ui, words: &str, end: Option<&mut dyn FnMut(&mut Ui)>) {
    let t = mg_uikit::tokens(ui.ctx());
    let size = mg_uikit::metrics::HEADING_SIZE * text_scale(ui);
    let mut text = RichText::new(words).size(size).color(t.text);
    if ui.fonts(|f| f.families().contains(&mg_uikit::fonts::bold())) {
        text = text.family(mg_uikit::fonts::bold());
    }
    ui.horizontal(|ui| {
        ui.label(text);
        if let Some(end) = end {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| end(ui));
        }
    });
    ui.add_space(mg_uikit::metrics::GAP_S - ui.spacing().item_spacing.y);
    let (rule, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().hline(rule.x_range(), rule.center().y, egui::Stroke::new(1.0, t.line_soft));
    ui.add_space(mg_uikit::metrics::GAP_M - ui.spacing().item_spacing.y);
}

/// The width of a section's heading's words in `ui` ([`panel_heading`]).
fn heading_width(ui: &Ui, words: &str) -> f32 {
    let size = mg_uikit::metrics::HEADING_SIZE * text_scale(ui);
    let font = egui::FontId::new(size, mg_uikit::fonts::bold());
    ui.painter().layout_no_wrap(words.into(), font, Color32::WHITE).size().x
}

fn highlighted(n: &Struct, hl: [bool; 5]) -> bool {
    let has = |label: &str| match n.get(label) {
        Some(Value::String(b)) => !b.is_empty(),
        Some(Value::ResRef(b)) => !b.is_empty(),
        Some(Value::Dword(v)) => *v != 0,
        _ => false,
    };
    (hl[0] && has("Comment"))
        || (hl[1] && has("Script"))
        || (hl[2] && has("Quest"))
        || (hl[3] && has("Animation"))
        || (hl[4] && has("Sound"))
}

/// What a line does besides its text, for the tree: its link's condition,
/// its action, journal update and sound, by name (`if c_has_key`, `do
/// a_give_gold`, `journal q_rats 20`, `sound vs_hello`).
pub(crate) fn markers(link: &Struct, n: &Struct) -> Vec<String> {
    let resref = |s: &Struct, l: &str| s.resref(l).filter(|r| !r.is_empty()).map(|r| r.to_string());
    let mut out = Vec::new();
    if let Some(c) = resref(link, "Active") {
        out.push(format!("if {c}"));
    }
    if let Some(a) = resref(n, "Script") {
        out.push(format!("do {a}"));
    }
    let quest = decode(n.string("Quest").unwrap_or_default());
    if !quest.is_empty() {
        let entry = n.integer("QuestEntry").unwrap_or(0);
        out.push(format!("journal {quest} {entry}"));
    }
    if let Some(s) = resref(n, "Sound") {
        out.push(format!("sound {s}"));
    }
    out
}

/// How lines look (Options › Conversation Editor).
struct Look {
    names: bool,
    /// Each line's condition, action, journal update and sound, named.
    scripts: bool,
    npc: Color32,
    pc: Color32,
    /// A link's lines.
    link: Color32,
    /// The most levels a line stands in by, its words keeping at least
    /// [`Look::least`] beside it in the tree's width ([`tree_levels`]).
    levels: usize,
    /// The least room a line's words keep, where the marks after them or
    /// its depth would leave less ([`LEAST_LINE_EMS`]).
    least: f32,
    /// Toggle Bookmark's key (Options › Keyboard), beside its entry in a
    /// row's right-click menu; empty where it has none.
    bookmark_key: String,
}

/// The NPC's and the player's text colors by default, for the dark theme
/// or the light (darker there, to stand out against white as the dark
/// theme's do against black).
pub(crate) fn default_colors(dark: bool) -> ([u8; 3], [u8; 3]) {
    match dark {
        true => ([245, 110, 114], [110, 168, 245]),
        false => ([180, 30, 30], [25, 85, 200]),
    }
}

impl Look {
    /// `link`: a link's lines' color (the theme's weak text).
    fn new(s: &crate::Settings, dark: bool, link: Color32) -> Look {
        let rgb = |c: Option<[u8; 3]>, [r, g, b]: [u8; 3]| {
            let [r, g, b] = c.unwrap_or([r, g, b]);
            Color32::from_rgb(r, g, b)
        };
        let (npc, pc) = default_colors(dark);
        Look {
            names: !s.dialog_hide_names,
            scripts: !s.dialog_hide_scripts,
            npc: rgb(s.dialog_npc_color, npc),
            pc: rgb(s.dialog_pc_color, pc),
            link,
            levels: usize::MAX,
            least: 0.0,
            bookmark_key: String::new(),
        }
    }
}

/// Whether a drop links (Ctrl, or Cmd on macOS) rather than moves.
fn link_key(ui: &Ui) -> bool {
    ui.input(|i| i.modifiers.ctrl || i.modifiers.mac_cmd)
}

/// How far a line of the conversation tree stands in from the one it is
/// under.
const INDENT: f32 = 16.0;
/// The room a row with no lines under it keeps where another has its
/// arrow.
const ARROW_ROOM: f32 = 18.0;

/// The least room a line's words keep in the tree, in ems of the editor's
/// text (156 points at 13), where a third of the tree's width is less
/// ([`least_line`]).
pub const LEAST_LINE_EMS: f32 = 12.0;

/// The least room a line's words keep in a tree `width` wide whose text is
/// `em` points: [`LEAST_LINE_EMS`] ems, or a third of the width where
/// that is more. The marks after the words are cut to what they leave,
/// and a line deeper than the width allows stands in no further than the
/// level that leaves its words this much, its guides with it (M5's W4: the
/// owner's ruling 29 cuts each line at the tree's right edge, and neither
/// the board nor SPEC says how far a deep line stands in).
pub fn least_line(width: f32, em: f32) -> f32 {
    (LEAST_LINE_EMS * em).max(width / 3.0)
}

/// What follows the words of a player's line that ends the conversation
/// ([`row_label`]): kept whole after them where they are cut.
const END_WORDS: &str = " [END DIALOGUE]";

/// The room a line of the conversation tree leaves at the tree's right
/// edge, the kit's tree rows' padding: beside the scroll bar where it is
/// shown (the kit's scroll bars are solid, never over the rows).
const RIGHT_ROOM: f32 = mg_uikit::tree::ROW_PADDING_RIGHT;

/// The most levels a line of the conversation tree stands in by in a tree
/// `width` wide, its words keeping `least` points beside its arrow: at
/// least one.
fn tree_levels(ui: &Ui, width: f32, least: f32) -> usize {
    let arrow = mg_uikit::metrics::ICON_BUTTON + ui.spacing().item_spacing.x;
    let pad = 2.0 * ui.spacing().button_padding.x;
    let room = width - arrow - pad - least;
    ((room / INDENT).floor().max(1.0)) as usize
}

/// `text` in `font` (`italics`) laid out on one line, cut with an ellipsis
/// at `room` points (whole where `room` is infinite), its color left to
/// whoever paints it: from what `view` keeps of this frame and the last,
/// laid out anew only where the words, the font or the room changed
/// (counted in [`DialogView::cuts_made`]).
fn cut_words(
    ui: &Ui,
    view: &mut DialogView,
    text: &str,
    font: &egui::FontId,
    italics: bool,
    room: f32,
) -> std::sync::Arc<egui::Galley> {
    use std::hash::{Hash as _, Hasher as _};
    let room = if room.is_finite() { room.max(1.0).floor() } else { f32::INFINITY };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (text, font, italics, room.to_bits()).hash(&mut hasher);
    let key = hasher.finish();
    if let Some(galley) = view.cuts.0.get(&key) {
        return galley.clone();
    }
    if let Some(galley) = view.cuts.1.remove(&key) {
        view.cuts.0.insert(key, galley.clone());
        return galley;
    }
    view.cuts_made += 1;
    let format = egui::TextFormat {
        font_id: font.clone(),
        color: Color32::PLACEHOLDER,
        italics,
        ..Default::default()
    };
    let mut job = egui::text::LayoutJob::single_section(text.to_owned(), format);
    job.wrap = egui::text::TextWrapping::truncate_at_width(room);
    let galley = ui.painter().layout_job(job);
    view.cuts.0.insert(key, galley.clone());
    galley
}

/// How a row's line is drawn ([`line_row`]).
struct LineLook<'a> {
    /// The row's whole text ([`row_label`]): its accessible name, and its
    /// tip where its words are cut.
    label: &'a str,
    /// It ends the conversation: [`END_WORDS`] kept whole after its words.
    ends: bool,
    color: Color32,
    italics: bool,
    /// Found by the highlights: drawn behind its words.
    found: Option<Color32>,
    selected: bool,
    /// What follows it: its scripts' marks (each drawn whole or not at
    /// all), or `?` for a condition with the marks hidden; in their font
    /// and color.
    marks: Option<(Vec<String>, egui::FontId, Color32)>,
}

/// A row's line after its arrow (the Conversation board's `.dl-row .ln`
/// and `.mks`): its words cut with an ellipsis where they and the marks
/// after them would run past the tree's right edge ([`RIGHT_ROOM`] short
/// of it), the marks following them. A mark is drawn whole or not at all:
/// the words are cut first, down to `least`, then the marks go from the
/// right, then the words further; and where even an ellipsis of them has
/// no room beside ` [END DIALOGUE]`, the end words alone, cut themselves
/// (the owner's ruling 29, M5's W4 fix rounds). A selectable label, named
/// by the row's whole text, as wide as its words; each mark a label named
/// by its words, drawn or not, in sight or not. Laid out only where it is
/// in sight (a row out of sight keeps its place, its names and what it
/// answers, and lays out nothing). Its response, whether its words were
/// cut, and the marks that had no room.
fn line_row(
    ui: &mut Ui,
    view: &mut DialogView,
    least: f32,
    line: LineLook<'_>,
) -> (egui::Response, bool, Vec<String>) {
    let pad = ui.spacing().button_padding;
    let gap = ui.spacing().item_spacing.x;
    let font = egui::TextStyle::Button.resolve(ui.style());
    let height =
        (ui.fonts_mut(|f| f.row_height(&font)) + 2.0 * pad.y).max(ui.spacing().interact_size.y);
    let room = (ui.available_width() - RIGHT_ROOM).max(1.0);
    let whole = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(room, height));
    let (enabled, selected) = (ui.is_enabled(), line.selected);
    let named = |r: &egui::Response, name: &str| {
        r.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, selected, name)
        });
    };
    let mark_named = |r: &egui::Response, words: &str| {
        r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, enabled, words));
    };
    let marks: &[String] = line.marks.as_ref().map_or(&[], |(m, _, _)| m.as_slice());
    if !ui.is_rect_visible(whole) {
        let size = egui::vec2(2.0 * pad.x, height);
        let (_, r) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        named(&r, line.label);
        for m in marks {
            let (_, marked) = ui.allocate_exact_size(size, egui::Sense::hover());
            mark_named(&marked, m);
        }
        return (r, false, Vec::new());
    }
    let words = match line.ends {
        true => line.label.strip_suffix(END_WORDS).unwrap_or(line.label),
        false => line.label,
    };
    let inf = f32::INFINITY;
    let mut end = (line.ends).then(|| cut_words(ui, view, END_WORDS, &font, line.italics, inf));
    let laid: Vec<_> = match &line.marks {
        Some((m, f, _)) => m.iter().map(|m| cut_words(ui, view, m, f, false, inf)).collect(),
        None => Vec::new(),
    };
    let fixed = 2.0 * pad.x + end.as_ref().map_or(0.0, |g| g.size().x);
    // (The most marks kept, from the left, that leave the words their
    // least room, or their whole width where that is less.)
    let natural = cut_words(ui, view, words, &font, line.italics, inf).size().x;
    let want = natural.min(least).min(room - fixed);
    let marks_w = |k: usize| laid[..k].iter().map(|g| gap + g.size().x).sum::<f32>();
    let mut kept = laid.len();
    while kept > 0 && room - fixed - marks_w(kept) < want {
        kept -= 1;
    }
    let text_room = (room - fixed - marks_w(kept)).max(want).max(1.0);
    let mut cut = cut_words(ui, view, words, &font, line.italics, text_room);
    // (Last of all, where the row has no room for an ellipsis of its words
    // beside ` [END DIALOGUE]`: the end words alone, cut themselves.)
    let ellipsis = cut_words(ui, view, "…", &font, line.italics, inf).size().x;
    let mut elided = cut.elided;
    if line.ends && room - fixed < ellipsis {
        cut = cut_words(ui, view, "", &font, line.italics, inf);
        let alone = (room - 2.0 * pad.x).max(1.0);
        end = Some(cut_words(ui, view, END_WORDS.trim_start(), &font, line.italics, alone));
        elided = true;
    }
    let end_w = end.as_ref().map_or(0.0, |g| g.size().x);
    let size = egui::vec2(2.0 * pad.x + cut.size().x + end_w, height);
    let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let dropped = marks[kept..].to_vec();
    named(&r, line.label);
    // (As egui's selectable label: framed while chosen, under the pointer,
    // held or with the keyboard's focus.)
    let painter = ui.painter();
    if selected || r.widget_state() != egui::widget_style::WidgetState::Inactive {
        let v = ui.style().interact_selectable(&r, selected);
        let framed = rect.expand(v.expansion);
        painter.rect(
            framed,
            v.corner_radius,
            v.weak_bg_fill,
            v.bg_stroke,
            egui::StrokeKind::Inside,
        );
    }
    let at = egui::pos2(rect.left() + pad.x, rect.center().y - cut.size().y / 2.0);
    let text = egui::Rect::from_min_size(at, egui::vec2(cut.size().x + end_w, cut.size().y));
    if let Some(found) = line.found {
        painter.rect_filled(text, 0.0, found);
    }
    painter.galley(at, cut.clone(), line.color);
    if let Some(end) = end {
        painter.galley(egui::pos2(at.x + cut.size().x, at.y), end, line.color);
    }
    let color = line.marks.as_ref().map_or(line.color, |(_, _, c)| *c);
    for (galley, words) in laid.into_iter().zip(marks).take(kept) {
        let (marked, response) =
            ui.allocate_exact_size(egui::vec2(galley.size().x, height), egui::Sense::hover());
        mark_named(&response, words);
        let y = marked.center().y - galley.size().y / 2.0;
        ui.painter().galley(egui::pos2(marked.left(), y), galley, color);
    }
    // (The marks with no room still each a label, named by their words, as
    // a row's out of sight are: one rule wherever the row is.)
    for words in &dropped {
        let (_, response) = ui.allocate_exact_size(egui::vec2(0.0, height), egui::Sense::hover());
        ui.add_space(-ui.spacing().item_spacing.x);
        mark_named(&response, words);
    }
    (r, elided, dropped)
}

/// The indent guides of a row `depth` levels under Root drawn at `row`
/// (the Conversation board's `.gd`): a 1 px line in the theme's `guide`
/// down the middle of the arrow of each line it is under, Root's place
/// too, as high as the row and the room between rows, so that a branch's
/// lines join; lighter on the selected row (`guide_on_fill`) and on the
/// row under the pointer (`guide_hover`). Only where the row is in sight: the tree lays out every
/// open row, and what is out of sight costs nothing more.
fn guides(ui: &Ui, row: egui::Rect, depth: usize, selected: bool) {
    let half = ui.spacing().item_spacing.y / 2.0;
    let span = egui::Rangef::new(row.top() - half, row.bottom() + half);
    if depth == 0 || !ui.is_rect_visible(row.expand2(egui::vec2(0.0, half))) {
        return;
    }
    // (Lighter on the selected row's fill, the board's; on a row under
    // the pointer, its own, so that they stay in sight there too.)
    let t = mg_uikit::tokens(ui.ctx());
    let color = match (selected, ui.rect_contains_pointer(row)) {
        (true, _) => t.guide_on_fill,
        (false, true) => t.guide_hover,
        (false, false) => t.guide,
    };
    let stroke = egui::Stroke::new(1.0, color);
    let painter = ui.painter();
    for level in 0..depth {
        // (Down the middle of the arrow of the line at `level`; on a whole
        // pixel, so that it stays 1 px.)
        let x = row.left() + level as f32 * INDENT + mg_uikit::metrics::ICON_BUTTON / 2.0;
        let x = painter.round_to_pixel_center(x);
        painter.vline(x, span, stroke);
    }
}

/// The selected row's fill (the board's `.dl-row.sel`: `accent-fill` the
/// tree's whole width, from its left edge to what is in sight at its
/// right, the room between rows halved into it), at `at`, under the row.
fn fill_row(ui: &Ui, at: egui::layers::ShapeIdx, row: egui::Rect) {
    let half = ui.spacing().item_spacing.y / 2.0;
    let full = fill_span(ui.clip_rect(), ui.max_rect(), row, half);
    let fill = mg_uikit::tokens(ui.ctx()).accent_fill;
    ui.painter().set(at, egui::Shape::rect_filled(full, 0.0, fill));
}

/// Where [`fill_row`] fills: the tree's width (`max`, the rows' room)
/// from its left edge to what of it is in sight (`clip`), `half` the
/// room between rows on either side of `row`. (The tree's vertical
/// scroll area clips up and down only: its clip rect is as wide as the
/// window, and the fill ran on under the window's right pane.)
fn fill_span(clip: egui::Rect, max: egui::Rect, row: egui::Rect, half: f32) -> egui::Rect {
    let right = clip.right().min(max.right());
    let x = egui::Rangef::new(max.left(), right);
    egui::Rect::from_x_y_ranges(x, egui::Rangef::new(row.top() - half, row.bottom() + half))
}

/// Draws one flattened row of conversation `key`'s tree.
#[allow(clippy::too_many_arguments)]
fn draw_flat_row(
    ui: &mut Ui,
    game: Option<&mg_rules::GameData>,
    key: ResKey,
    g: &Gff,
    fr: FlatRow,
    view: &mut DialogView,
    hl: [bool; 5],
    look: &Look,
    dropped: &mut Option<Drop>,
) {
    let row = fr.row;
    let kind = fr.kind;
    let index = fr.index;
    let linked = fr.linked;
    let children = fr.children;
    let open = fr.open;
    let Some(l) = links(g, row.parent).get(row.pos) else { return };
    let Some(n) = node(g, kind, index) else { return };
    let label = row_label(game, g, kind, l, look.names);
    let cond = l.resref("Active").filter(|r| !r.is_empty());
    // (The row chosen: its words in the theme's `on-fill` on the
    // selection's fill, where a speaker's color would not read.)
    let color = if view.selected == Some(row) {
        mg_uikit::tokens(ui.ctx()).on_fill
    } else if linked {
        look.link
    } else if kind == Kind::Entry {
        look.npc
    } else {
        look.pc
    };
    // (Found: the theme's `select`, faint behind the words.)
    let found = highlighted(n, hl).then(|| mg_uikit::tokens(ui.ctx()).select.gamma_multiply(0.3));
    let chosen = view.selected == Some(row);
    // (Its scripts' marks, after its words: on the selected row's fill
    // `on-fill`, the board's; a condition's `?` where they are hidden.)
    let marks = match look.scripts {
        true => Some(markers(l, n)).filter(|m| !m.is_empty()).map(|m| {
            let color = match chosen {
                true => mg_uikit::tokens(ui.ctx()).on_fill.gamma_multiply(0.85),
                false => ui.visuals().weak_text_color(),
            };
            (m, egui::TextStyle::Small.resolve(ui.style()), color)
        }),
        false => cond.is_some().then(|| {
            let font = egui::TextStyle::Body.resolve(ui.style());
            let color = if chosen {
                mg_uikit::tokens(ui.ctx()).on_fill.gamma_multiply(0.85)
            } else {
                ui.visuals().weak_text_color()
            };
            (vec!["?".to_string()], font, color)
        }),
    };
    // (The selected row filled the whole width of the tree, the board's
    // `.dl-row.sel`, under what the row draws.)
    let fill_at = ui.painter().add(egui::Shape::Noop);
    // (No further in than leaves its words their least room.)
    let level = usize::from(fr.depth).min(look.levels);
    let row_shown = ui.horizontal(|ui| {
        ui.add_space(level as f32 * INDENT);
        if children {
            // (Named with its line, said open or not: a screen reader's
            // "Collapse [OWNER] - Hello", as the NUI layers' arrows.)
            let (icon, verb, tip) = match open {
                true => (mg_uikit::Icon::ChevronDown, "Collapse", "Hide the lines under it"),
                false => (mg_uikit::Icon::ChevronRight, "Expand", "Show the lines under it"),
            };
            let arrow = crate::widgets::icon_only(icon, verb)
                .name(format!("{verb} {label}"))
                .expanded(open);
            if ui.add(arrow).on_hover_text(tip).clicked() {
                if open {
                    view.collapsed.insert((kind, index));
                } else {
                    view.collapsed.remove(&(kind, index));
                }
            }
        } else {
            ui.add_space(ARROW_ROOM);
        }
        let ends = kind == Kind::Reply && !linked && !children;
        let line = LineLook {
            label: &label,
            ends,
            color,
            italics: linked,
            found,
            selected: chosen,
            marks,
        };
        let (r, cut, no_room) = line_row(ui, view, look.least, line);
        // (The whole line where its words are cut, as the kit's cut
        // words have it, and the marks with no room; its condition
        // after it.)
        let r = match (cut, no_room.is_empty()) {
            (false, true) => r,
            (true, true) => r.on_hover_text(label.as_str()),
            (_, false) => r.on_hover_text(format!("{label}\n{}", no_room.join("  "))),
        };
        // (With its conversation: a row dropped on another's tree
        // is not one of its rows.)
        r.dnd_set_drag_payload((key, row));
        let ours = |p: std::sync::Arc<(ResKey, Row)>| (p.0 == key).then_some(p.1);
        if let Some(from) = r.dnd_release_payload::<(ResKey, Row)>().and_then(ours)
            && from != row
        {
            let link = link_key(ui);
            // On a line of its own kind: above it, under its parent.
            *dropped = Some(if !link && from.parent.child_kind() == kind {
                Drop { from, onto: row.parent, link, before: Some(row.pos) }
            } else {
                Drop { from, onto: Parent::Node(kind, index), link, before: None }
            });
        }
        // Where a line held over this one would go: a line above it,
        // or a frame around it.
        if let Some(from) = r.dnd_hover_payload::<(ResKey, Row)>().and_then(ours)
            && from != row
        {
            let stroke = egui::Stroke::new(2.0, mg_uikit::tokens(ui.ctx()).accent);
            if !link_key(ui) && from.parent.child_kind() == kind {
                ui.painter().hline(r.rect.x_range(), r.rect.top(), stroke);
            } else {
                ui.painter().rect_stroke(r.rect, 2.0, stroke, egui::StrokeKind::Inside);
            }
        }
        mg_uikit::context_menu(&r, |ui| {
            view.selected = Some(row);
            if linked
                && ui
                    .button("Go to Linked")
                    .on_hover_text("The line this row links to, where it is owned")
                    .clicked()
            {
                go_to_line(view, g, kind, index);
                ui.close();
            }
            for (label, asked) in [
                ("Add", Asked::Add),
                ("Copy", Asked::Copy),
                ("Cut", Asked::Cut),
                ("Paste", Asked::Paste),
                ("Paste As Link", Asked::PasteLink),
                ("Delete", Asked::Delete),
            ] {
                if ui.button(label).clicked() {
                    view.menu = Some(asked);
                    ui.close();
                }
            }
            ui.separator();
            let mut entry = egui::Button::new("Bookmark");
            if !look.bookmark_key.is_empty() {
                let keys = mg_uikit::MenuKeys::new(ui, look.bookmark_key.clone());
                entry = entry.shortcut_text(keys.atom());
            }
            if ui.add(entry).on_hover_text(BOOKMARK_TIP).clicked() {
                view.menu = Some(Asked::Bookmark);
                ui.close();
            }
        });
        let r = match cond {
            Some(c) => r.on_hover_text(format!("Appears when {c} returns TRUE")),
            None => r,
        };
        if r.clicked() {
            view.selected = Some(row);
        }
        // A double click on a link goes to the line it links to, as
        // in Aurora.
        if linked && r.double_clicked() {
            go_to_line(view, g, kind, index);
        }
        if view.reveal && view.selected == Some(row) {
            r.scroll_to_me(Some(egui::Align::Center));
            view.reveal = false;
        }
        if view.nudge && view.selected == Some(row) {
            r.scroll_to_me(None);
            view.nudge = false;
        }
    });
    if chosen {
        fill_row(ui, fill_at, row_shown.response.rect);
    }
    guides(ui, row_shown.response.rect, level, chosen);
}

/// The tags of the creatures placed in the module's areas, sorted, each
/// once (the Speaker Tag list).
fn creature_tags(app: &mut Moonglow, ctx: &egui::Context) -> std::sync::Arc<Vec<String>> {
    let Some(ws) = app.ws.as_mut() else { return Default::default() };
    // (Kept until the module changes: the arrow keys ask each frame the
    // box has the keys, and every area's objects were read each time.
    // A revision is no other workspace's.)
    let id = egui::Id::new("dlg-creature-tags");
    let revision = ws.revision();
    type Kept = (u64, std::sync::Arc<Vec<String>>);
    if let Some((r, tags)) = ctx.data(|d| d.get_temp::<Kept>(id))
        && r == revision
    {
        return tags;
    }
    let areas = mg_module::areas::every(&ws.module);
    let mut tags = std::collections::BTreeSet::new();
    for a in areas {
        let Ok(g) = ws.doc(&ResKey::new(a, mg_core::ResType::GIT)) else { continue };
        for c in g.root.list("Creature List").unwrap_or(&[]) {
            if let Some(t) = c.string("Tag").filter(|t| !t.is_empty()) {
                tags.insert(decode(t));
            }
        }
    }
    let tags: std::sync::Arc<Vec<String>> = std::sync::Arc::new(tags.into_iter().collect());
    ctx.data_mut(|d| d.insert_temp::<Kept>(id, (revision, tags.clone())));
    tags
}

#[allow(clippy::too_many_arguments)]
fn text_panel(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    kind: Kind,
    index: u32,
    n: &Struct,
    view: &mut DialogView,
    actions: &mut Vec<Action>,
    focus: bool,
) {
    let path = node_path(kind, index);
    // Within its panel, however narrow: a row too wide for it goes onto
    // two lines.
    let width = ui.available_width();
    ui.set_max_width(width);
    if kind == Kind::Entry {
        let speaker = decode(n.string("Speaker").unwrap_or_default());
        // (One combo, the board's: the field and its list's chevron joined,
        // the field never narrower than a tag of 16 characters needs.)
        let arrow = 24.0 + ui.spacing().icon_width + 2.0 * ui.spacing().button_padding.x;
        let least = name_least(ui) + arrow;
        form_row_least(ui, "Speaker tag", least, |ui| {
            let id = egui::Id::new(("dlg-speaker", key, index));
            if focus {
                ui.memory_mut(|m| m.request_focus(id));
            }
            ui.spacing_mut().item_spacing.x = 0.0;
            let width = (ui.available_width() - arrow - 2.0).max(name_least(ui));
            let mut chosen = commit_text(app, ui, id, &speaker, false, width);
            // The tags of the creatures placed in the module's areas (read
            // only while the list is open: it takes every area's objects).
            let r = crate::widgets::dropdown(("dlg-speaker-tags", key, index))
                .selected_text("")
                .width(24.0)
                .show_ui(ui, |ui| {
                    mg_uikit::list_rows(ui);
                    for t in creature_tags(app, ui.ctx()).iter() {
                        if ui.selectable_label(*t == speaker, t).clicked() {
                            chosen = Some(t.clone());
                        }
                    }
                })
                .response
                .on_hover_text("The tag of a creature placed in the module");
            // (The arrow keys step through the tags, read only while the
            // list has the keys.)
            let mut tag = speaker.clone();
            if (r.clicked() || r.has_focus())
                && crate::widgets::arrow_step(ui, &r, &creature_tags(app, ui.ctx()), &mut tag)
            {
                chosen = Some(tag);
            }
            if let Some(v) = chosen {
                actions.push(set(
                    key,
                    "Speaker",
                    path.clone(),
                    "Speaker",
                    Value::String(encode(&v)),
                ));
            }
        });
        form_row(ui, "", |ui| {
            ui.add(
                egui::Label::new(RichText::new("(empty: the conversation's owner)").weak()).wrap(),
            )
        });
    }
    let ls = n.locstring("Text").cloned().unwrap_or_default();
    form_row(ui, "Text", |ui| {
        if crate::widgets::languages_button(ui) {
            app.loc_edit = Some(LocStringEdit::new(
                FieldTarget::new(key, path.clone(), "Text"),
                "Line text",
                &ls,
            ));
        }
        if ui.button("Token…").on_hover_text("Insert a token such as <FirstName>").clicked() {
            view.token_picker = true;
        }
    });
    let english = ls
        .text(crate::text::edit_language(), mg_core::Gender::Male)
        .map(|t| t.into_owned())
        .unwrap_or_default();
    let id = egui::Id::new(("dlg-text", key, kind, index));
    // (F6 to the panel: its first field, a player's line's text.)
    if focus && kind != Kind::Entry {
        ui.memory_mut(|m| m.request_focus(id));
    }
    // Text from the talk table shows under the field; text typed in the
    // field is the line's own, said instead.
    let from_tlk = (english.is_empty() && !ls.strref.is_none())
        .then(|| app.game.as_deref().and_then(|g| g.string(ls.strref)))
        .flatten();
    // Without text in the language edited or a talk-table string: its
    // text in another language (English first), marked as not its own.
    let other =
        (english.is_empty() && ls.strref.is_none()).then(|| crate::text::elsewhere(&ls)).flatten();
    let shown = other
        .as_ref()
        .map(|(_, text)| text.clone())
        .or_else(|| from_tlk.clone())
        .unwrap_or(english);
    // (The board's memo, two lines high, growing with what it holds.)
    let memo = |app: &mut Moonglow, ui: &mut Ui| {
        let width = ui.available_width();
        ui.scope(|ui| {
            if from_tlk.is_some() {
                ui.visuals_mut().override_text_color = Some(crate::blueprint::talk_table_color(ui));
            }
            crate::widgets::commit_loc_text_in(
                &mut app.buffers,
                ui,
                id,
                &shown,
                Some(2),
                width,
                None,
            )
        })
        .inner
    };
    let typed = match &other {
        Some((language, _)) => crate::widgets::borrowed_field(
            ui,
            language.name().unwrap_or("another language"),
            "This line's text is in another language: it has none in the language edited \
             (Options › Language). Typing here gives it text in that language",
            |ui| memo(app, ui),
        ),
        None => memo(app, ui),
    };
    if let Some(v) = typed {
        // (Put in the module's talk table first, where Options › Language
        // says so.)
        let now = with_edited_text(ls.clone(), &v);
        let words = words_edit(app, key, &ls, &now);
        let edit = Edit::SetField {
            key,
            path: path.clone(),
            label: "Text".into(),
            value: Some(now.into_value()),
        };
        let edits = std::iter::once(edit).chain(words).collect();
        actions.push(Action::ApplyTyped(Command::new("Line text", edits)));
    }
    if let Some(t) = from_tlk {
        let note = format!("From the talk table (string {}): {t}", ls.strref.0);
        ui.add(egui::Label::new(RichText::new(note).weak()).wrap());
    }
    if view.token_picker {
        // (Those of the language edited: Options › General.)
        let tokens = crate::widgets::language_tokens(app, crate::text::edit_language());
        let (mut close, mut with_token) = (false, None);
        let ctx = ui.ctx().clone();
        let window = crate::widgets::modal(&ctx, "Select Token");
        let window = crate::widgets::resizable(&ctx, window, egui::vec2(260.0, 340.0))
            .min_size([180.0, 160.0]);
        let footed = window.show_foot(&ctx, |ui| {
            let height = ui.available_height().max(60.0);
            let list = egui::ScrollArea::vertical().auto_shrink(false).max_height(height);
            list.show(ui, |ui| {
                mg_uikit::list_rows(ui);
                for t in &tokens {
                    if ui.selectable_label(false, t).clicked() {
                        with_token = Some(crate::widgets::with_token(ui.ctx(), id, &shown, t));
                        close = true;
                    }
                }
            });
            ((), crate::widgets::Foot::new().cancel())
        });
        if footed.is_some_and(|f| f.pressed.cancelled()) {
            close = true;
        }
        if let Some(v) = with_token {
            let now = with_edited_text(ls.clone(), &v);
            let words = words_edit(app, key, &ls, &now);
            let edit = Edit::SetField {
                key,
                path: path.clone(),
                label: "Text".into(),
                value: Some(now.into_value()),
            };
            let edits = std::iter::once(edit).chain(words).collect();
            actions.push(Action::Apply(Command::new("Insert token", edits)));
        }
        if close {
            view.token_picker = false;
        }
    }
}

/// A script field with its parameters and a preview of its source: a
/// link's condition (`Active`, `ConditionParams`), else a line's action
/// (`Script`, `ActionParams`). Beside the parameters, parameter sets
/// (`param_sets`): added (script and parameters as one step), saved,
/// saved to and added from files, and copied and pasted as a set's text
/// (to another line, conversation or module, or a message).
#[allow(clippy::too_many_arguments)]
fn script_with_params(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    path: GffPath,
    condition: bool,
    s: &Struct,
    view: &mut DialogView,
    actions: &mut Vec<Action>,
) {
    use crate::FileKind;
    use crate::param_sets::{self, ParamSet};
    use crate::widgets::{PARAMETER_SETS, SetChoice};
    let (label, list, salt) = match condition {
        true => ("Active", "ConditionParams", "cond"),
        false => ("Script", "ActionParams", "act"),
    };
    let current = s.resref(label).unwrap_or(ResRef::EMPTY);
    // The script as it is this frame (one typed is set as the frame ends).
    let mut script_now = current;
    let edit = button_room(ui, "Edit");
    resref_row(ui, "Script", edit, |ui| {
        let id = egui::Id::new(("dlg-script", key, salt, path.to_string()));
        let types = [ResType::NSS, ResType::NCS];
        let picked = resref_field(app, ui, id, current, "Select a script", &types);
        if let Some(v) = picked {
            actions.push(set(key, "Script", path.clone(), label, Value::resref(v)));
            script_now = v;
        }
        if let Some(name) = crate::widgets::edit_script_button(app, ui, id, current) {
            actions.push(Action::EditScript { name, condition });
        }
    });
    // (As wide as they are, not as the page: the page stretches what is
    // put in it, which squeezed the parameters' fields to a few letters.)
    let own_width = egui::Layout::top_down(egui::Align::Min);
    // The pairs being edited live in a buffer until a field loses focus or a
    // row is added or removed; with no field focused the file's pairs show.
    let stored = params(s, list);
    let buf_id = egui::Id::new(("dlg-params-buf", key, salt, path.to_string()));
    let mut pairs: Vec<(String, String)> =
        ui.ctx().data_mut(|d| d.get_temp(buf_id)).unwrap_or_else(|| stored.clone());
    // Parameter sets, kept in Moonglow's data folder, and in files anywhere:
    // the sets chosen this frame, added after the rows (to what was typed
    // in them this frame too).
    let dir = app.param_set_dir.clone();
    let place = format!("{salt} {path}");
    let mut chosen: Vec<ParamSet> = Vec::new();
    let mut from_file = false;
    let mut to_file = false;
    // Paste Set's clipboard, come this frame: added as a saved set is.
    let paste_id = egui::Id::new(("dlg-param-paste", key, salt, path.to_string()));
    let mut pasted = false;
    if let Some(text) = crate::widgets::asked_paste(ui.ctx(), paste_id) {
        match param_sets::from_text(&text) {
            Ok(set) => {
                chosen.push(set);
                pasted = true;
            }
            Err(e) => app.log.error(format!(
                "Paste Set: the clipboard doesn't hold a parameter set ({e}); nothing was changed"
            )),
        }
    }
    let named = |pairs: &[(String, String)]| pairs.iter().any(|(n, _)| !n.trim().is_empty());
    let something = named(&pairs) || !script_now.is_empty();
    let set_of = |pairs: &[(String, String)], script: ResRef| ParamSet {
        script: Some(script).filter(|s| !s.is_empty()),
        parameters: pairs.iter().filter(|(n, _)| !n.trim().is_empty()).cloned().collect(),
    };
    // Parameters: Add Parameter, Add Set and the sets' other commands in a
    // menu (the board's `+`, Add Set and `…`).
    let mut add = false;
    form_row(ui, "Parameters", |ui| {
        let tip = "Add a parameter (EE: read with GetScriptParam)";
        add = crate::widgets::icon_button(ui, mg_uikit::Icon::Plus, "Add Parameter", tip).clicked();
        // (The board's Add Set and its ⌄: a button that opens a menu.)
        let popup = egui::Id::new(("dlg-add-set", key, salt, path.to_string()));
        let open = egui::Popup::is_id_open(ui.ctx(), popup);
        let tip = "Add a saved set's parameters and script. Same names take its values.";
        // (At the editor's text size, as its neighbors: panel content, the
        // coordinator's ruling at U14's review.)
        let size = egui::TextStyle::Button.resolve(ui.style()).size;
        let add_set = mg_uikit::KitButton::new(None, "Add Set").opens(open).tip(tip);
        let add_set = ui.add(add_set.text_size(size));
        let menu = egui::Popup::menu(&add_set).id(popup).style(mg_uikit::menu_style);
        menu.show(|ui| {
            mg_uikit::menu_rows(ui);
            match crate::widgets::sets_menu(ui, &mut app.log, &PARAMETER_SETS, dir.as_deref()) {
                Some(SetChoice::Add(set)) => chosen.push(set),
                Some(SetChoice::FromFile) => from_file = true,
                Some(SetChoice::Manage) => {
                    view.param_sets.get_or_insert_default();
                }
                None => {}
            }
        });
        let naming = view.param_set_name.as_ref().is_some_and(|(p, _)| *p == place);
        let more = mg_uikit::IconButton::new(mg_uikit::Icon::MoreHorizontal, SET_COMMANDS)
            .icon_only(true)
            .menu(ui, |ui| {
                mg_uikit::menu_rows(ui);
                let save = !naming && dir.is_some() && something;
                if ui
                    .add_enabled(save, egui::Button::new("Save Set…"))
                    .on_hover_text("Keep the script and these parameters under a name")
                    .on_disabled_hover_text("Keep the script and these parameters under a name")
                    .clicked()
                {
                    view.param_set_name = Some((place.clone(), String::new()));
                    ui.close();
                }
                if ui
                    .add_enabled(something, egui::Button::new("Copy Set"))
                    .on_hover_text("Copy the script and these parameters, for Paste Set")
                    .clicked()
                {
                    ui.ctx().copy_text(param_sets::to_text(&set_of(&pairs, script_now)));
                    ui.close();
                }
                if ui
                    .button("Paste Set")
                    .on_hover_text("Add the set on the clipboard. Same names take its values.")
                    .clicked()
                {
                    crate::widgets::ask_paste(ui.ctx(), paste_id);
                    ui.close();
                }
                if ui
                    .add_enabled(something, egui::Button::new("Save to File…"))
                    .on_hover_text("Save the script and these parameters as a file, to share")
                    .clicked()
                {
                    to_file = true;
                    ui.close();
                }
            });
        more.response.on_hover_text("Save Set…, Copy Set, Paste Set and Save to File…");
    });
    // Save Set's name, typed in place (that of another line is forgotten
    // as the line's Data is drawn, `data_body`).
    if let Some((_, name)) = view.param_set_name.as_mut().filter(|(p, _)| *p == place) {
        let (save, cancel) = crate::widgets::set_name_row(ui, name);
        crate::widgets::replaced_note(ui, &param_sets::SHELF, dir.as_deref(), name);
        if save && let Some(dir) = &dir {
            match param_sets::save(dir, name, &set_of(&pairs, script_now)) {
                Ok(p) => app.log.info(format!("Parameter set saved: {}", p.display())),
                Err(e) => app.log.error(format!("Save Set: {e}")),
            }
        }
        if save || cancel {
            view.param_set_name = None;
        }
    }
    let mut commit = false;
    let mut focused = false;
    let mut remove_at = None;
    // A name's field and a value's, sharing the page's width: the value
    // the wider.
    // (Within the panel: the remove button and the gaps out of its width.)
    let room = (ui.available_width() - 56.0).max(120.0);
    let name_width = (room * 0.4).clamp(50.0, 260.0);
    let value_width = (room - name_width).clamp(60.0, 520.0);
    ui.with_layout(own_width, |ui| {
        // (A table of names and values: rows 22 apart, a line between.)
        mg_uikit::with_table_rows(ui, |ui| {
            for (i, (k, v)) in pairs.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    // (First in its row: at the row's end it was past the
                    // page's edge where the page is narrow.)
                    let tip = "Remove the parameter";
                    let named = match k.is_empty() {
                        true => format!("Remove Parameter {}", i + 1),
                        false => format!("Remove Parameter {k}"),
                    };
                    if crate::widgets::icon_button(ui, mg_uikit::Icon::Minus, &named, tip).clicked()
                    {
                        remove_at = Some(i);
                    }
                    let name = mg_uikit::text_field(k).desired_width(name_width);
                    let rk = ui.add(name.hint_text("name"));
                    let value = mg_uikit::text_field(v).desired_width(value_width);
                    let rv = ui.add(value.hint_text("value"));
                    crate::widgets::leaves_at_a_press(ui.ctx(), &rk);
                    crate::widgets::leaves_at_a_press(ui.ctx(), &rv);
                    commit |= rk.lost_focus() || rv.lost_focus();
                    focused |= crate::widgets::has_keys(&rk) || crate::widgets::has_keys(&rv);
                });
            }
        });
    });
    if let Some(i) = remove_at {
        pairs.remove(i);
        commit = true;
    }
    if add {
        pairs.push((String::new(), String::new()));
        commit = true;
    }
    // (Several files at once, in the order chosen: a later file's
    // parameter of a name an earlier one has takes its value.)
    if from_file {
        for file in app.dialogs.open_files(FileKind::ParameterSet, None) {
            match param_sets::load_file(&file) {
                Ok(set) => chosen.push(set),
                Err(e) => app.log.error(format!("Add from File: {e}")),
            }
        }
    }
    if to_file {
        let suggested = match script_now.is_empty() {
            true => "parameters".to_string(),
            false => script_now.to_string(),
        };
        let suggested = std::path::PathBuf::from(param_sets::file_name(&suggested));
        if let Some(file) = app.dialogs.save_file(FileKind::ParameterSet, Some(&suggested)) {
            match param_sets::save_file(&file, &set_of(&pairs, script_now)) {
                Ok(p) => app.log.info(format!("Parameter set saved: {}", p.display())),
                Err(e) => app.log.error(format!("Save to File: {e}")),
            }
        }
    }
    if !chosen.is_empty() {
        // The sets added: their parameters merged into those shown (with
        // what was typed in them), the last set's script in the Script
        // field, as one step of Undo. The buffer goes: what was typed is
        // in the step, and the step is what shows.
        let mut merged = pairs.clone();
        let mut script = None;
        for set in &chosen {
            param_sets::merge(&mut merged, &set.parameters);
            script = set.script.filter(|s| !s.is_empty()).or(script);
        }
        let mut edits = Vec::new();
        if let Some(script) = script.filter(|s| *s != script_now) {
            edits.push(Edit::SetField {
                key,
                path: path.clone(),
                label: label.into(),
                value: Some(Value::resref(script)),
            });
        }
        if merged != stored {
            edits.push(Edit::SetField {
                key,
                path: path.clone(),
                label: list.into(),
                value: Some(params_value(&merged)),
            });
        }
        if !edits.is_empty() {
            let step = if pasted { "Paste parameter set" } else { "Add parameter set" };
            actions.push(Action::Apply(Command::new(step, edits)));
        }
        ui.ctx().data_mut(|d| d.remove_temp::<Vec<(String, String)>>(buf_id));
    } else if commit {
        ui.ctx().data_mut(|d| d.remove_temp::<Vec<(String, String)>>(buf_id));
        if pairs != stored {
            actions.push(set(key, "Script parameters", path.clone(), list, params_value(&pairs)));
        }
    } else if focused {
        ui.ctx().data_mut(|d| d.insert_temp(buf_id, pairs));
    } else {
        ui.ctx().data_mut(|d| d.remove_temp::<Vec<(String, String)>>(buf_id));
    }
    if !current.is_empty() {
        let preview = script_preview_cached(app, ui.ctx(), salt, current);
        // (The board's `.preview`: its visible lines, on the inset ground,
        // scrolled for the rest.)
        form_row(ui, "Script preview", |ui| match preview {
            Some((src, ranges)) => {
                let line_h = ui.text_style_height(&egui::TextStyle::Monospace);
                let lines = 4.0 * line_h;
                let inset = mg_uikit::tokens(ui.ctx()).inset;
                let bar = ui.spacing().scroll.bar_width + ui.spacing().scroll.bar_inner_margin;
                egui::Frame::new().fill(inset).inner_margin(4).show(ui, |ui| {
                    let down = egui::ScrollArea::vertical().id_salt(("dlg-preview", salt));
                    down.max_height(lines).auto_shrink([false, true]).show_rows(
                        ui,
                        line_h,
                        ranges.len(),
                        |ui, range| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            let mut numbers = String::new();
                            let mut visible = String::new();
                            for (pos, idx) in range.enumerate() {
                                if pos > 0 {
                                    numbers.push('\n');
                                    visible.push('\n');
                                }
                                use std::fmt::Write as _;
                                let _ = write!(&mut numbers, "{}", idx + 1);
                                visible.push_str(&src[ranges[idx].clone()]);
                            }
                            ui.horizontal_top(|ui| {
                                let gutter = RichText::new(numbers).monospace().weak();
                                ui.add(egui::Label::new(gutter).selectable(false));
                                let across = egui::ScrollArea::horizontal()
                                    .id_salt(("dlg-preview-code", salt))
                                    .auto_shrink([false, true]);
                                across.show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        let code = RichText::new(visible).monospace();
                                        ui.add(egui::Label::new(code).extend());
                                        ui.add_space(bar);
                                    });
                                });
                            });
                        },
                    );
                });
            }
            None => {
                ui.weak("No source for this script.");
            }
        });
    }
}

fn split_line_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let bytes = text.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            let end = if i > start && bytes[i - 1] == b'\r' { i - 1 } else { i };
            out.push(start..end);
            start = i + 1;
        }
    }
    if start < text.len() {
        let end = if text.ends_with('\r') { text.len() - 1 } else { text.len() };
        out.push(start..end);
    }
    if out.is_empty() {
        out.push(0..0);
    }
    out
}

fn fast_bytes_hash(bytes: &[u8]) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

type ScriptPreview = (std::sync::Arc<String>, std::sync::Arc<Vec<std::ops::Range<usize>>>);

fn script_preview_cached(
    app: &Moonglow,
    ctx: &egui::Context,
    salt: &str,
    current: ResRef,
) -> Option<ScriptPreview> {
    let script_key = ResKey::new(current, ResType::NSS);
    let open_sig =
        app.scripts.get(&script_key).map(|b| (b.text.len(), fast_bytes_hash(b.text.as_bytes())));
    let ws_key = app.ws.as_ref().map(|w| (w.identity(), w.revision()));
    let mark = (script_key, ws_key, app.load_order, open_sig);
    crate::blueprint::kept_for_key(ctx, egui::Id::new(("dlg-preview-src", salt)), &mark, || {
        app.script_source(&script_key).map(|src| {
            let ranges = std::sync::Arc::new(split_line_ranges(&src));
            (std::sync::Arc::new(src), ranges)
        })
    })
}

fn journal_summary_kept(app: &mut Moonglow, ctx: &egui::Context) -> std::sync::Arc<JournalSummary> {
    // (Its texts are in the editing language, read through the game's
    // codepage: kept for those and the load order too.)
    let ws_key = app.ws.as_ref().map(|w| (w.identity(), w.revision()));
    let key = (ws_key, app.game_names(), crate::text::edit_language().0, crate::text::game_table());
    crate::blueprint::kept_for_key(ctx, egui::Id::new("dlg-journal-summary"), &key, || {
        let cats = app
            .ws
            .as_mut()
            .and_then(|w| w.doc(&ResKey::parse("module", ResType::JRL)?).ok())
            .map(|g| g.root.items(&jrl::CATEGORIES))
            .unwrap_or(&[]);
        let mut tags = Vec::with_capacity(cats.len() + 1);
        tags.push(String::new());
        let mut entries = Vec::with_capacity(cats.len());
        for c in cats {
            tags.push(decode(c.read(&jrl::categories::TAG).as_bytes()));
            let list = mg_module::journal::entries(c)
                .iter()
                .map(|e| {
                    let id = e.read(&jrl::categories::entry_list::ID);
                    let t = e.read(&jrl::categories::entry_list::TEXT);
                    (id, crate::text::shown_text(&t))
                })
                .collect();
            entries.push(list);
        }
        std::sync::Arc::new(JournalSummary { tags, entries })
    })
}

#[derive(Clone, Default)]
struct JournalSummary {
    tags: Vec<String>,
    entries: Vec<Vec<(u32, String)>>,
}

/// The name of the menu of a script's parameter sets' commands beside Add
/// Set (the board's `…`): Save Set…, Copy Set, Paste Set, Save to File….
pub const SET_COMMANDS: &str = "Set Commands";

/// A line's Other Actions (the board's section): the animation played,
/// the sound played (and Play), the journal's category and entry.
fn other_tab(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    kind: Kind,
    index: u32,
    n: &Struct,
    actions: &mut Vec<Action>,
) {
    let path = node_path(kind, index);
    // (A list as wide as the row's room, its own frame within it.)
    let wide = |ui: &Ui| (ui.available_width() - ui.spacing().item_spacing.x - 2.0).max(60.0);
    form_row(ui, "Play animation", |ui| {
        let anim = n.dword("Animation").unwrap_or(0);
        let shown = ANIMATIONS
            .iter()
            .find(|a| a.1 == anim)
            .map_or_else(|| anim.to_string(), |a| a.0.to_string());
        let mut pick = anim;
        let values = ANIMATIONS.map(|a| a.1);
        crate::widgets::dropdown(("dlg-anim", key, kind, index))
            .selected_text(shown)
            .width(wide(ui))
            .show(ui, &mut pick, &values, |v| {
                ANIMATIONS.iter().find(|a| a.1 == *v).map_or("", |a| a.0)
            });
        if pick != anim {
            actions.push(set(key, "Animation", path.clone(), "Animation", Value::Dword(pick)));
        }
    });
    let play = button_room(ui, "Play");
    resref_row(ui, "Play sound", play, |ui| {
        let sound = n.resref("Sound").unwrap_or(ResRef::EMPTY);
        let id = egui::Id::new(("dlg-sound", key, kind, index));
        let picked = resref_field(app, ui, id, sound, "Select a sound", &[ResType::WAV]);
        if let Some(v) = picked {
            actions.push(set(key, "Sound", path.clone(), "Sound", Value::resref(v)));
        }
        if ui.add_enabled(!sound.is_empty(), egui::Button::new("Play")).clicked() {
            app.play_sound(crate::audio::Channel::Preview, sound, 1.0, false);
        }
    });
    // Journal categories and entries of the module, cached per workspace revision.
    let journal = journal_summary_kept(app, ui.ctx());
    let quest = decode(n.string("Quest").unwrap_or_default());
    form_row(ui, "Journal", |ui| {
        let mut pick = quest.clone();
        crate::widgets::dropdown(("dlg-quest", key, kind, index))
            .selected_text(if quest.is_empty() { "(none)".into() } else { quest.clone() })
            .width(wide(ui))
            .show(ui, &mut pick, &journal.tags, |t| {
                if t.is_empty() { "(none)".to_string() } else { t.clone() }
            });
        if pick.is_empty() && !quest.is_empty() {
            actions.push(set(key, "Journal", path.clone(), "Quest", Value::String(Vec::new())));
        } else if pick != quest
            && let Some(cat_idx) = journal.tags[1..].iter().position(|t| *t == pick)
        {
            let mut edits = vec![Edit::SetField {
                key,
                path: path.clone(),
                label: "Quest".into(),
                value: Some(Value::String(encode(&pick))),
            }];
            let first = journal.entries.get(cat_idx).and_then(|e| e.first()).map(|e| e.0);
            edits.push(Edit::SetField {
                key,
                path: path.clone(),
                label: "QuestEntry".into(),
                value: Some(Value::Dword(first.unwrap_or(0))),
            });
            actions.push(Action::Apply(Command::new("Journal", edits)));
        }
    });
    if !quest.is_empty() {
        form_row(ui, "Journal entry", |ui| {
            let entry = n.dword("QuestEntry").unwrap_or(0);
            let entries: &[(u32, String)] = journal.tags[1..]
                .iter()
                .position(|t| *t == quest)
                .and_then(|cat_idx| journal.entries.get(cat_idx))
                .map_or(&[], Vec::as_slice);
            let ids: Vec<u32> = entries.iter().map(|(id, _)| *id).collect();
            let mut pick = entry;
            crate::widgets::dropdown(("dlg-quest-entry", key, kind, index))
                .selected_text(entry.to_string())
                .width(wide(ui))
                .show_with(
                    ui,
                    &mut pick,
                    || ids.clone(),
                    |ui, pick| {
                        for (id, t) in entries {
                            ui.selectable_value(pick, *id, format!("{id}: {t}"));
                        }
                    },
                );
            if pick != entry {
                let id = Value::Dword(pick);
                actions.push(set(key, "Journal entry", path.clone(), "QuestEntry", id));
            }
        });
    }
}

/// The conversation's own settings (the Conversation panel's form):
/// the scripts run at its normal end and when it is aborted, and Stop
/// camera zoom in. `focus`: F6 gives Normal end the keyboard's focus.
fn file_tab(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    g: &Gff,
    actions: &mut Vec<Action>,
    focus: bool,
) {
    for (label, field) in [("Normal end", "EndConversation"), ("Aborted", "EndConverAbort")] {
        resref_row(ui, label, 0.0, |ui| {
            let current = g.root.resref(field).unwrap_or(ResRef::EMPTY);
            let id = egui::Id::new(("dlg-end", key, field));
            if focus && field == "EndConversation" {
                ui.memory_mut(|m| m.request_focus(id));
            }
            let types = [ResType::NSS, ResType::NCS];
            let picked = resref_field(app, ui, id, current, "Select a script", &types);
            if let Some(v) = picked {
                actions.push(set(key, label, GffPath::root(), field, Value::resref(v)));
            }
        });
    }
    let mut zoom = g.root.byte("PreventZoomIn").unwrap_or(0) != 0;
    // (Under the fields where it fits there; else from the panel's edge.)
    let words = "Stop camera zoom in";
    let font = egui::TextStyle::Body.resolve(ui.style());
    let wide = ui.painter().layout_no_wrap(words.into(), font, Color32::WHITE).size().x
        + ui.spacing().icon_width
        + ui.spacing().icon_spacing;
    let changed = match ui.available_width() - LABEL_WIDTH >= wide {
        true => form_row(ui, "", |ui| ui.check_box(&mut zoom, words).changed()),
        false => ui.check_box(&mut zoom, words).changed(),
    };
    {
        if changed {
            actions.push(set(
                key,
                "Camera zoom",
                GffPath::root(),
                "PreventZoomIn",
                Value::Byte(u8::from(zoom)),
            ));
        }
    }
}

/// A new, empty conversation's bytes.
pub(crate) fn new_file() -> Vec<u8> {
    new_dialog().to_bytes().unwrap_or_default()
}

/// The New Conversation window.
pub(crate) fn windows(app: &mut Moonglow, ui: &mut Ui) {
    use crate::widgets::{NameWindow, Named};
    let Some(mut name) = app.new_dialog.clone() else { return };
    let window = NameWindow {
        new: true,
        title: "New Conversation",
        label: "Name (up to 16 characters)",
        restype: ResType::DLG,
        taken: "The module already has a conversation of this name.",
        replaces: false,
        button: "Create",
    };
    let module = app.ws.as_ref().map(|w| &w.module);
    app.new_dialog = match window.show(ui.ctx(), module, &mut name) {
        Named::Open => Some(name),
        Named::Cancelled => {
            app.new_dialog_for = None;
            None
        }
        Named::Chosen(k) => {
            let mut edits = vec![Edit::SetResource { key: k, data: Some(new_file()) }];
            // (Made from an object's Conversation field: it is given the
            // conversation in the same step.)
            if let Some((targets, label)) = app.new_dialog_for.take() {
                edits.extend(targets.into_iter().map(|(key, path)| Edit::SetField {
                    key,
                    path,
                    label: label.clone(),
                    value: Some(Value::resref(k.resref)),
                }));
            }
            app.actions.push(Action::Apply(Command::new(format!("New conversation {k}"), edits)));
            app.actions.push(Action::OpenTab(Tab::Dialog(k)));
            None
        }
    };
}

fn matches(text: &str, find: &str, s: &DialogSearch) -> bool {
    crate::script_tools::find(text, find, 0, &s.options).is_some()
}

/// The Search pane: find in this conversation or all of them, replace in
/// this one.
fn search_pane(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    g: &Gff,
    view: &mut DialogView,
    actions: &mut Vec<Action>,
    viewed: bool,
) {
    let s = &mut view.search;
    form_row(ui, "Find What", |ui| {
        let width = ui.available_width();
        let field = ui.add(mg_uikit::text_field(&mut s.find).desired_width(width));
        crate::widgets::autofocus(ui, &field);
        if std::mem::take(&mut s.focus) {
            field.request_focus();
        }
        // Find Next (F3) from the field too, as in Aurora (Sacha's report):
        // it types nothing, so the field gives it up.
        let keys = &app.keymap;
        if field.has_focus() && ui.input_mut(|i| keys.consume(i, crate::keys::Cmd::DialogFindNext))
        {
            s.again = true;
        }
    });
    form_row(ui, "Replace With", |ui| {
        let width = ui.available_width();
        ui.add(mg_uikit::text_field(&mut s.replace).desired_width(width));
    });
    ui.horizontal_wrapped(|ui| {
        ui.check_box(&mut s.options.match_case, "Match Case");
        ui.check_box(&mut s.options.whole_word, "Match Whole Word Only");
        ui.check_box(&mut s.all_files, "All Files in Module");
    });
    let mut find = false;
    let mut replace_all = false;
    ui.horizontal(|ui| {
        find = ui.add_enabled(!s.find.is_empty(), egui::Button::new("Find")).clicked();
        find |= std::mem::take(&mut s.again) && !s.find.is_empty();
        replace_all = ui
            .add_enabled(
                !viewed && !s.find.is_empty() && !s.all_files,
                egui::Button::new("Replace All"),
            )
            .clicked();
    });
    if find {
        let keys: Vec<ResKey> = if s.all_files {
            let mut k: Vec<ResKey> = app
                .ws
                .as_ref()
                .map(|w| w.module.keys_of(ResType::DLG).copied().collect())
                .unwrap_or_default();
            k.sort();
            k
        } else {
            vec![key]
        };
        let mut results = Vec::new();
        for k in keys {
            // (Another conversation as an editor has it, else read from
            // the module for this look and let go: each was made a
            // document of the workspace and copied whole.)
            let read;
            let d: &Gff = if k == key {
                g
            } else if let Some(open) = app.ws.as_ref().and_then(|w| w.loaded(&k)) {
                open
            } else {
                match app.ws.as_ref().and_then(|w| w.module.gff(&k)) {
                    Some(Ok(other)) => {
                        read = other;
                        &read
                    }
                    _ => continue,
                }
            };
            for kind in [Kind::Entry, Kind::Reply] {
                for (i, n) in mg_module::dialog::nodes(d, kind).iter().enumerate() {
                    let t = line_text(app.game.as_deref(), n);
                    if matches(&t, &s.find, s) {
                        results.push((k, kind, i as u32, t));
                    }
                }
            }
        }
        s.results = results;
    }
    if replace_all {
        let mut ng = Gff::clone(g);
        let mut n = 0;
        for kind in [Kind::Entry, Kind::Reply] {
            if let Some(list) = ng.root.list_mut(kind.list()) {
                for line in list {
                    let t = text(line);
                    let (new, count) =
                        crate::script_tools::replace_all(&t, &s.find, &s.replace, &s.options);
                    if count > 0 {
                        let ls = line.locstring("Text").cloned().unwrap_or_default();
                        line.set("Text", with_edited_text(ls, &new).into_value());
                        n += count;
                    }
                }
            }
        }
        if n > 0 {
            // (With the count of its words as it is now.)
            let words = word_count(&ng);
            ng.root.set("NumWords", Value::Dword(words));
            actions.push(replace(key, "Replace text", &ng));
        }
        app.log.info(format!("Replaced {n} in {key}"));
    }
    ui.separator();
    // The results in sight are laid out, a line each (a line's first,
    // its whole text on hover), the others taking a row's room: a search
    // for "the" over a world's conversations is tens of thousands, and
    // each was copied and drawn with its whole text every frame.
    mg_uikit::list_rows(ui);
    let row = mg_uikit::row_pitch(ui);
    let row_h = mg_uikit::row_height(ui);
    let spacing_y = ui.spacing().item_spacing.y;
    let total = s.results.len();
    let start_y = ui.cursor().top();
    let first_vis =
        (((ui.clip_rect().top() - row - start_y) / row).floor().max(0.0) as usize).min(total);
    let last_vis =
        (((ui.clip_rect().bottom() + row - start_y) / row).ceil().max(0.0) as usize).min(total);
    if first_vis > 0 {
        ui.allocate_space(egui::vec2(
            1.0,
            (first_vis as f32 * row_h + (first_vis - 1) as f32 * spacing_y).max(0.0),
        ));
    }
    let mut clicked = None;
    for (at, (k, kind, index, t)) in s.results[first_vis..last_vis].iter().enumerate() {
        let at = first_vis + at;
        ui.push_id(at, |ui| {
            let line = t.lines().next().unwrap_or_default();
            let label = if *k == key {
                format!("{kind:?} {index}: {line}")
            } else {
                format!("{k}: {kind:?} {index}: {line}")
            };
            let found = ui.add(egui::Button::selectable(false, label).truncate());
            let found = if line.len() < t.len() { found.on_hover_text(t) } else { found };
            if found.clicked() {
                clicked = Some(at);
            }
        });
    }
    let trailing = total - last_vis;
    if trailing > 0 {
        ui.allocate_space(egui::vec2(
            1.0,
            (trailing as f32 * row_h + (trailing - 1) as f32 * spacing_y).max(0.0),
        ));
    }
    let clicked = clicked.and_then(|at| s.results.get(at)).map(|r| (r.0, r.1, r.2));
    if let Some((k, kind, index)) = clicked {
        if k == key {
            // (Its parents opened out, and scrolled to.)
            go_to_line(view, g, kind, index);
        } else {
            actions.push(Action::OpenTab(Tab::Dialog(k)));
        }
    }
}

impl Moonglow {
    /// Brings conversation `key`'s Search to the front as Find (Ctrl+F)
    /// does, from the next frame ([`search_forward`]). For the window's
    /// tests.
    #[doc(hidden)]
    pub fn show_conversation_search(&mut self, key: ResKey) {
        self.dialog_search_forward = Some(key);
    }

    /// Reads a Twine or Ink story as a new conversation of the module,
    /// named after the file (made unique), and opens it; its name.
    pub fn import_conversation(&mut self, path: &std::path::Path) -> Option<ResKey> {
        use mg_module::dialog_io::Format;
        let source = match std::fs::read_to_string(path) {
            Ok(s) => crate::text::for_windows_1252(&s),
            Err(e) => {
                self.log.error(format!("{}: {e}", path.display()));
                return None;
            }
        };
        let read = Format::of(path)
            .and_then(|f| f.read(&source))
            .unwrap_or_else(|| Err("not a Twine (.twee) or Ink (.ink) story".to_string()));
        let g = match read {
            Ok(g) => g,
            Err(e) => {
                self.log.error(format!("{}: {e}", path.display()));
                return None;
            }
        };
        let ws = self.ws.as_mut()?;
        let stem = path.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default();
        let stem: String = mg_core::ResRef::name_of(&stem).chars().take(16).collect();
        let stem = if stem.is_empty() { "conversation".to_string() } else { stem };
        let free =
            |name: &str| ResKey::parse(name, ResType::DLG).filter(|k| !ws.module.contains(k));
        // (With the builder's code, as the team's pattern puts it; never
        // twice.)
        let naming = ws.module.project_file.naming_for(ResType::DLG);
        let key = free(&naming.fit(&stem, "")).or_else(|| {
            let r = naming.numbered(&stem, 1, &|s| free(s).is_none())?;
            Some(ResKey::new(r, ResType::DLG))
        })?;
        let lines = mg_module::dialog::nodes(&g, Kind::Entry).len()
            + mg_module::dialog::nodes(&g, Kind::Reply).len();
        let Some(made) = written(key, &g) else {
            self.log.error(format!("{} could not be made a conversation", path.display()));
            return None;
        };
        self.actions.push(Action::Apply(mg_edit::Command::new(
            format!("Import conversation {}", key.resref),
            vec![made],
        )));
        self.actions.push(Action::OpenTab(Tab::Dialog(key)));
        self.log.info(format!("Imported {} as {key}: {lines} lines", path.display()));
        Some(key)
    }
}

/// A link's condition script, if it has one.
fn condition(link: &Struct) -> Option<String> {
    link.resref("Active").filter(|r| !r.is_empty()).map(|r| r.to_string())
}

/// Whether a link's condition is taken to pass.
fn passes(link: &Struct, assume: &std::collections::BTreeMap<String, bool>) -> bool {
    condition(link).is_none_or(|c| assume.get(&c).copied().unwrap_or(true))
}

/// The NPC line the game says among `parent`'s: the first whose condition
/// passes.
pub(crate) fn npc_line(
    g: &Gff,
    parent: Parent,
    assume: &std::collections::BTreeMap<String, bool>,
) -> Option<u32> {
    links(g, parent).iter().find(|l| passes(l, assume)).map(link_index)
}

/// A condition's name as a toggle: TRUE or FALSE, clicked to switch.
fn condition_toggle(ui: &mut Ui, c: &str, assume: &mut std::collections::BTreeMap<String, bool>) {
    let on = assume.get(c).copied().unwrap_or(true);
    let text = format!("if {c}: {}", if on { "TRUE" } else { "FALSE" });
    if ui
        .small_button(text)
        .on_hover_text("What this condition is taken to return. Click to switch.")
        .clicked()
    {
        assume.insert(c.to_string(), !on);
    }
}

/// A line's actions and journal update (its markers but the condition).
fn effects(link: &Struct, n: &Struct) -> String {
    markers(link, n).into_iter().filter(|m| !m.starts_with("if ")).collect::<Vec<_>>().join("  ")
}

/// Test mode: the conversation played as the game plays it. The NPC says
/// the first of its lines whose condition passes; the player is offered
/// the replies whose conditions pass. Conditions are taken to return TRUE
/// until switched, and the actions and journal updates of each line are
/// shown.
pub(crate) fn test_window(app: &mut Moonglow, ui: &mut Ui) {
    let open: Vec<ResKey> =
        app.dialog_views.iter().filter(|(_, v)| v.test.is_some()).map(|(k, _)| *k).collect();
    for key in open {
        let Some(g) = app.ws.as_mut().and_then(|w| w.doc(&key).ok().cloned()) else { continue };
        let view = app.dialog_views.get_mut(&key).expect("listed");
        let game = app.game.as_deref();
        let mut path = view.test.clone().unwrap_or_default();
        let mut assume = view.assume.clone();
        let title = format!("Conversation Test: {key}");
        let mut close = false;
        let ctx = ui.ctx().clone();
        let window = crate::widgets::modal(&ctx, title);
        let window = crate::widgets::resizable(&ctx, window, egui::vec2(480.0, 420.0))
            .min_size([300.0, 200.0]);
        let footed = window.show_foot(&ctx, |ui| {
            // What was said so far, taking the room the window has over the
            // turn under it (that turn's height the frame before).
            let under_id = ui.id().with("test-under-history");
            let under_was: Option<f32> = ui.data(|d| d.get_temp(under_id));
            let row = ui.spacing().interact_size.y;
            let height = (ui.available_height() - under_was.unwrap_or(8.0 * row)).max(3.0 * row);
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .max_height(height)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for (k, i) in &path {
                        if let Some(n) = node(&g, *k, *i) {
                            let who = if *k == Kind::Entry { "NPC" } else { "You" };
                            ui.weak(format!("{who}: {}", line_text(game, n)));
                        }
                    }
                });
            // (The cursor, not the ui's rectangle: the body is as tall as
            // the room the footer leaves.)
            let from = ui.cursor().top();
            ui.scope(|ui| turn(ui, &g, game, &mut path, &mut assume));
            let under = ui.cursor().top() - from;
            if under_was.is_none_or(|was| (was - under).abs() > 0.5) {
                ui.data_mut(|d| d.insert_temp(under_id, under));
                ui.ctx().request_repaint();
            }
            // (Back: a turn, the reply and the NPC line before it.)
            ((), crate::widgets::Foot::new().button("< Back", !path.is_empty()).leave("Done"))
        });
        if let Some(footed) = &footed {
            if footed.pressed.clicked("< Back") {
                path.pop();
                path.pop();
            }
            close = footed.pressed.cancelled();
        }
        view.test = if close { None } else { Some(path) };
        view.assume = assume;
    }
}

/// Conversation Test's turn: the NPC's line (those passed over shown), the
/// player's replies to it (Back and Done are the footer's).
fn turn(
    ui: &mut Ui,
    g: &Gff,
    game: Option<&mg_rules::GameData>,
    path: &mut Vec<(Kind, u32)>,
    assume: &mut std::collections::BTreeMap<String, bool>,
) {
    ui.separator();
    // The NPC's turn: from the root, or after the last reply.
    let parent = match path.last() {
        None => Parent::Root,
        Some(&(k, i)) => Parent::Node(k, i),
    };
    let candidates = links(g, parent).to_vec();
    let spoken = npc_line(g, parent, assume);
    // The NPC lines before the one said were passed over.
    for l in &candidates {
        let i = link_index(l);
        let Some(n) = node(g, Kind::Entry, i) else { continue };
        ui.horizontal(|ui| {
            if Some(i) == spoken {
                ui.label(RichText::new(format!("NPC: {}", line_text(game, n))).strong());
            } else {
                ui.weak(format!("(not said) {}", line_text(game, n)));
            }
            if let Some(c) = condition(l) {
                condition_toggle(ui, &c, assume);
            }
        });
        if Some(i) == spoken {
            let e = effects(l, n);
            if !e.is_empty() {
                ui.weak(e);
            }
            break;
        }
    }
    let Some(entry) = spoken else {
        ui.weak(if candidates.is_empty() {
            "[END DIALOGUE]"
        } else {
            "[END DIALOGUE: no NPC line's condition passes]"
        });
        return;
    };
    ui.separator();
    // The player's replies.
    let replies = links(g, Parent::Node(Kind::Entry, entry)).to_vec();
    if replies.is_empty() {
        ui.weak("[END DIALOGUE]");
    }
    // Numbered as the game numbers the ones it offers.
    let mut number = 0;
    for l in &replies {
        let i = link_index(l);
        let Some(n) = node(g, Kind::Reply, i) else { continue };
        let t = line_text(game, n);
        let t = if t.is_empty() { "[CONTINUE]".to_string() } else { t };
        let shown = passes(l, assume);
        number += usize::from(shown);
        let label = if shown { format!("{number}. {t}") } else { t };
        ui.horizontal(|ui| {
            if shown {
                if ui.button(&label).clicked() {
                    path.push((Kind::Entry, entry));
                    path.push((Kind::Reply, i));
                }
            } else {
                ui.weak(format!("(hidden) {label}"));
            }
            if let Some(c) = condition(l) {
                condition_toggle(ui, &c, assume);
            }
            let e = effects(l, n);
            if !e.is_empty() {
                ui.weak(e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Key;
    use mg_module::dialog::{add_link, new_link, node_mut};

    /// The Journal summary a node's Data panel shows is in the editing
    /// language as it is now: kept for the module's revision alone, it
    /// stayed in the language it was first made in (the review's D7).
    #[test]
    fn the_journal_summary_follows_the_editing_language() {
        use mg_core::{Gender, Language, LocString};
        let mut journal = mg_module::journal::new_journal();
        let mut cat = mg_module::journal::new_category(&[]);
        let mut entry = mg_module::journal::new_entry(&[]);
        let mut text = LocString::from_text(Language::ENGLISH, Gender::Male, b"Hello".to_vec());
        text.set(Language::FRENCH, Gender::Male, b"Bonjour".to_vec());
        entry.write(&jrl::categories::entry_list::TEXT, text);
        cat.items_mut(&jrl::categories::ENTRY_LIST).push(entry);
        journal.root.items_mut(&jrl::CATEGORIES).push(cat);
        let mut m = mg_module::Module::new();
        m.set_gff(ResKey::parse("module", ResType::JRL).unwrap(), &journal).unwrap();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.ws = Some(mg_edit::Workspace::new(m));
        let ctx = egui::Context::default();
        let was = crate::text::edit_language();
        crate::text::set_edit_language(Language::ENGLISH);
        let english = journal_summary_kept(&mut app, &ctx).entries[0][0].1.clone();
        crate::text::set_edit_language(Language::FRENCH);
        let french = journal_summary_kept(&mut app, &ctx).entries[0][0].1.clone();
        crate::text::set_edit_language(was);
        assert_eq!((english.as_str(), french.as_str()), ("Hello", "Bonjour"));
    }

    /// The selected row's fill ends at the tree's right edge: a vertical
    /// scroll area clips up and down only, its clip rect as wide as the
    /// window, and the fill ran on under the window's right pane (Node,
    /// Data, Search) in the README's picture (M5 P1).
    #[test]
    fn the_selected_row_s_fill_stays_in_the_tree() {
        let r = |x0: f32, y0: f32, x1: f32, y1: f32| {
            egui::Rect::from_min_max((x0, y0).into(), (x1, y1).into())
        };
        let (clip, max) = (r(251.0, 135.0, 1561.0, 788.0), r(517.0, 135.0, 1275.0, 788.0));
        let fill = fill_span(clip, max, r(517.0, 135.0, 561.2, 157.0), 0.0);
        assert_eq!(fill, r(517.0, 135.0, 1275.0, 157.0));
        // Scrolled half out of sight at the right (a window off the
        // screen): to what is in sight.
        let fill =
            fill_span(r(251.0, 135.0, 900.0, 788.0), max, r(517.0, 135.0, 561.2, 157.0), 2.0);
        assert_eq!(fill, r(517.0, 133.0, 900.0, 159.0));
    }

    /// What a conversation's panel asked for in a frame its editor was not
    /// drawn after it is set with that frame's actions, never kept for a
    /// later frame, where the lines may have been numbered anew under it
    /// (M4's U14 review, item 8).
    #[test]
    fn a_panel_s_edit_its_editor_did_not_take_is_set_with_its_frame() {
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        let key = ResKey::parse("talk", ResType::DLG).unwrap();
        let mut view = DialogView::default();
        view.from_panels.push(Action::Undo);
        app.dialog_views.insert(key, view);
        flush_panels(&mut app);
        assert!(matches!(app.actions.as_slice(), [Action::Undo]), "set with the frame");
        assert!(app.dialog_views[&key].from_panels.is_empty(), "nothing kept");
    }

    /// The light theme's text colors stand out against its white as the
    /// web asks of text (a contrast of 4.5 or more), and more than the dark
    /// theme's would there.
    #[test]
    fn the_light_theme_s_text_colors_stand_out_against_white() {
        fn luminance([r, g, b]: [u8; 3]) -> f64 {
            let c = |v: u8| {
                let v = f64::from(v) / 255.0;
                if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
            };
            0.2126 * c(r) + 0.7152 * c(g) + 0.0722 * c(b)
        }
        let against_white = |c: [u8; 3]| 1.05 / (luminance(c) + 0.05);
        let contrast_on = |fg: [u8; 3], bg: Color32| {
            let l1 = luminance(fg);
            let l2 = luminance([bg.r(), bg.g(), bg.b()]);
            (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05)
        };
        let (light, dark) = (default_colors(false), default_colors(true));
        for (l, d) in [(light.0, dark.0), (light.1, dark.1)] {
            assert!(against_white(l) >= 4.5, "{l:?}: {}", against_white(l));
            assert!(against_white(l) > against_white(d));
            for bg in [
                mg_uikit::Tokens::DARK.inset,
                mg_uikit::Tokens::DARK.panel,
                mg_uikit::Tokens::DARK.hover_row,
            ] {
                assert!(contrast_on(d, bg) >= 4.5, "{d:?} on {bg:?}: {}", contrast_on(d, bg));
            }
        }
    }

    /// A conversation's fingerprint, kept with a module's tabs, is fixed:
    /// the same as the records written with Rust's `DefaultHasher` have
    /// (none is lost by the change), and the same whatever the toolchain.
    #[test]
    fn a_conversation_s_fingerprint_is_fixed() {
        use std::hash::{Hash, Hasher};
        let g = sample();
        let bytes = g.to_bytes().unwrap();
        let mut before = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut before);
        assert_eq!(fingerprint(&g), before.finish(), "as the records written before have it");
        let mut fixed = siphasher::sip::SipHasher13::new_with_keys(0, 0);
        bytes.hash(&mut fixed);
        assert_eq!(fingerprint(&g), fixed.finish());
        assert_eq!(fingerprint(&new_dialog()), 15_852_168_482_793_792_934, "pinned");
    }

    /// Root, a greeting with a reply that has a line under it, and a link.
    fn sample() -> Gff {
        let mut g = new_dialog();
        let hello = add_node(&mut g, Parent::Root, "Hello.");
        let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who?");
        add_node(&mut g, Parent::Node(Kind::Reply, who), "Me.");
        let go = add_node(&mut g, Parent::Root, "Go.");
        add_link(&mut g, Parent::Node(Kind::Entry, go), who);
        g
    }

    #[test]
    fn the_rows_are_those_drawn() {
        let g = sample();
        let rows = shown_rows(&g, &HashSet::new());
        let lines: Vec<_> = rows.iter().map(|r| (r.line, r.up, r.children)).collect();
        let (e, r) = (Kind::Entry, Kind::Reply);
        assert_eq!(
            lines,
            [
                (None, None, true),
                (Some((e, 0)), Some(0), true),
                (Some((r, 0)), Some(1), true),
                (Some((e, 1)), Some(2), false),
                (Some((e, 2)), Some(0), true),
                // (The link: nothing under it.)
                (Some((r, 0)), Some(4), false),
            ]
        );
        // A folded line's are not drawn.
        let folded = HashSet::from([(Kind::Reply, 0)]);
        assert_eq!(shown_rows(&g, &folded).len(), 5);
    }

    /// A malformed file in which two lines own each next one (not as a
    /// link), 40 deep: each line's are drawn under it once, not 2^40 rows.
    #[test]
    fn lines_owned_twice_are_drawn_once() {
        let mut g = new_dialog();
        let mut parents = vec![Parent::Root];
        for depth in 0..40 {
            let first = add_node(&mut g, parents[0], &format!("a{depth}"));
            let second = add_node(&mut g, parents[0], &format!("b{depth}"));
            let kind = parents[0].child_kind();
            let under = [Parent::Node(kind, first), Parent::Node(kind, second)];
            // Both own the next line: the second as its own, not a link.
            let next = add_node(&mut g, under[0], &format!("c{depth}"));
            let (list, owned) = match kind {
                Kind::Entry => ("EntryList", "RepliesList"),
                Kind::Reply => ("ReplyList", "EntriesList"),
            };
            let second_node = &mut g.root.list_mut(list).unwrap()[second as usize];
            second_node.list_mut(owned).unwrap().push(new_link(next, under[1], false));
            parents = vec![Parent::Node(under[0].child_kind(), next)];
        }
        let rows = shown_rows(&g, &HashSet::new());
        assert!(rows.len() < 200, "{} rows", rows.len());
    }

    #[test]
    fn the_keys_go_as_in_a_windows_tree() {
        let g = sample();
        let rows = shown_rows(&g, &HashSet::new());
        let step = |at, key| tree_step(&rows, at, key, 3);
        assert_eq!(step(0, Key::ArrowUp), TreeStep::None);
        assert_eq!(step(0, Key::ArrowDown), TreeStep::To(1));
        assert_eq!(step(5, Key::ArrowDown), TreeStep::None);
        assert_eq!(step(3, Key::Home), TreeStep::To(0));
        assert_eq!(step(1, Key::End), TreeStep::To(5));
        assert_eq!(step(1, Key::PageDown), TreeStep::To(4));
        assert_eq!(step(4, Key::PageDown), TreeStep::To(5));
        assert_eq!(step(4, Key::PageUp), TreeStep::To(1));
        // Right: into an open line; nothing on a line with none under it.
        assert_eq!(step(0, Key::ArrowRight), TreeStep::To(1));
        assert_eq!(step(2, Key::ArrowRight), TreeStep::To(3));
        assert_eq!(step(3, Key::ArrowRight), TreeStep::None);
        assert_eq!(step(5, Key::ArrowRight), TreeStep::None);
        // Left: folds an open line; up from one with nothing under it.
        assert_eq!(step(2, Key::ArrowLeft), TreeStep::Fold((Kind::Reply, 0), false));
        assert_eq!(step(3, Key::ArrowLeft), TreeStep::To(2));
        assert_eq!(step(5, Key::ArrowLeft), TreeStep::To(4));
        assert_eq!(step(0, Key::ArrowLeft), TreeStep::None, "Root");
        // Folded: Right opens it, Left goes up.
        let folded = shown_rows(&g, &HashSet::from([(Kind::Reply, 0)]));
        assert_eq!(
            tree_step(&folded, 2, Key::ArrowRight, 3),
            TreeStep::Fold((Kind::Reply, 0), true)
        );
        assert_eq!(tree_step(&folded, 2, Key::ArrowLeft, 3), TreeStep::To(1));
        assert_eq!(tree_step(&[], 0, Key::ArrowDown, 3), TreeStep::None);
    }

    /// Backspace goes to the line a row is under; the keypad's + and −
    /// open and fold a line, going nowhere, and do nothing to a line
    /// already so or with nothing under it.
    #[test]
    fn backspace_and_the_keypad_s_plus_and_minus() {
        let g = sample();
        let rows = shown_rows(&g, &HashSet::new());
        let step = |rows: &[Shown], at, key| tree_step(rows, at, key, 3);
        assert_eq!(step(&rows, 3, Key::Backspace), TreeStep::To(2));
        assert_eq!(step(&rows, 1, Key::Backspace), TreeStep::To(0), "Root");
        assert_eq!(step(&rows, 0, Key::Backspace), TreeStep::None);
        assert_eq!(step(&rows, 5, Key::Backspace), TreeStep::To(4), "from a link");
        assert_eq!(step(&rows, 2, Key::Minus), TreeStep::Fold((Kind::Reply, 0), false));
        assert_eq!(step(&rows, 2, Key::Plus), TreeStep::None, "open already");
        assert_eq!(step(&rows, 3, Key::Minus), TreeStep::None, "nothing under it");
        assert_eq!(step(&rows, 5, Key::Plus), TreeStep::None, "a link");
        assert_eq!(step(&rows, 0, Key::Minus), TreeStep::None, "Root");
        let folded = shown_rows(&g, &HashSet::from([(Kind::Reply, 0)]));
        assert_eq!(step(&folded, 2, Key::Plus), TreeStep::Fold((Kind::Reply, 0), true));
        assert_eq!(step(&folded, 2, Key::Minus), TreeStep::None, "folded already");
    }

    /// A letter goes to the next row beginning with it, in either case,
    /// round the end and back to itself.
    #[test]
    fn a_letter_goes_to_the_next_row_beginning_with_it() {
        let texts: Vec<String> =
            ["Root", "Hello.", "who?", "Me.", "Go.", "Who?"].map(String::from).to_vec();
        assert_eq!(seek(&texts, 0, 'w'), Some(2));
        assert_eq!(seek(&texts, 2, 'W'), Some(5));
        assert_eq!(seek(&texts, 5, 'w'), Some(2), "round the end");
        assert_eq!(seek(&texts, 1, 'h'), Some(1), "itself, the only one");
        assert_eq!(seek(&texts, 3, 'r'), Some(0), "Root");
        assert_eq!(seek(&texts, 3, 'x'), None);
        assert_eq!(seek(&[], 0, 'x'), None);
        assert_eq!(seek(&["Élan".to_string(), "x".into()], 1, 'é'), Some(0));
    }

    /// Letters typed in quick succession: all of them begin the row gone
    /// to, from the selected one on; the same letter again goes round the
    /// rows beginning with it.
    #[test]
    fn letters_typed_quickly_go_by_all_of_them() {
        let texts: Vec<String> =
            ["Root", "[OWNER] - Hello.", "Goodbye.", "Gold?", "go on."].map(String::from).to_vec();
        assert_eq!(type_ahead(&texts, 0, "g"), Some(2));
        assert_eq!(type_ahead(&texts, 2, "go"), Some(2), "itself, first");
        assert_eq!(type_ahead(&texts, 2, "gol"), Some(3));
        assert_eq!(type_ahead(&texts, 2, "GO "), Some(4));
        assert_eq!(type_ahead(&texts, 3, "goo"), Some(2), "round the end");
        assert_eq!(type_ahead(&texts, 2, "gg"), Some(3), "the same letter: the next");
        assert_eq!(type_ahead(&texts, 4, "ggg"), Some(2));
        assert_eq!(type_ahead(&texts, 0, "[owner] - h"), Some(1));
        assert_eq!(type_ahead(&texts, 0, "gx"), None);
        assert_eq!(type_ahead(&texts, 0, ""), None);
    }

    /// The keypad's * opens a line's branch, folded deeper down too, not
    /// lines of other branches; from Root, every line.
    #[test]
    fn the_keypad_s_star_opens_a_branch() {
        let g = sample();
        let hello = branch_lines(&g, Some((Kind::Entry, 0)));
        let all = [(Kind::Entry, 0), (Kind::Reply, 0), (Kind::Entry, 1)];
        assert_eq!(hello, HashSet::from(all));
        // (Go's link is not a line of its own.)
        assert_eq!(branch_lines(&g, Some((Kind::Entry, 2))), HashSet::from([(Kind::Entry, 2)]));
        assert_eq!(branch_lines(&g, None).len(), 4);
    }

    /// A line folded over the selection takes it; folded higher up, the
    /// line folded there; a selection drawn keeps its place.
    #[test]
    fn the_selection_goes_to_the_line_folded_over_it() {
        let g = sample();
        let me = Some(Row { parent: Parent::Node(Kind::Reply, 0), pos: 0 });
        let who = Some(Row { parent: Parent::Node(Kind::Entry, 0), pos: 0 });
        let hello = Some(Row { parent: Parent::Root, pos: 0 });
        assert_eq!(in_sight(&g, &HashSet::new(), me), me);
        assert_eq!(in_sight(&g, &HashSet::from([(Kind::Reply, 0)]), me), who);
        let both = HashSet::from([(Kind::Reply, 0), (Kind::Entry, 0)]);
        assert_eq!(in_sight(&g, &both, me), hello);
        assert_eq!(in_sight(&g, &both, None), None);
        assert_eq!(in_sight(&g, &both, hello), hello);
    }

    /// The bookmarks and folded lines follow their lines across a delete
    /// that numbers them anew, its undo and its redo, also when the editor
    /// sees only where a few changes led; a bookmark on a line deleted
    /// goes, and comes back with it.
    #[test]
    fn bookmarks_and_folds_follow_their_lines() {
        let key = ResKey::parse("talk", ResType::DLG).unwrap();
        let (e, r) = (Kind::Entry, Kind::Reply);
        let before = sample();
        // (What the workspace makes of the bytes the editor gives it.)
        let read = |g: &Gff| Gff::read(&g.to_bytes().unwrap()).unwrap();
        let mut view = DialogView::default();
        view.follow(key, &before, 1);
        // "Go." (entry 2) folded, bookmarked after "Me." (entry 1);
        // "Who?" (reply 0) folded; Go's link selected.
        view.bookmarks = vec![(e, 1), (e, 2)];
        view.collapsed = HashSet::from([(e, 2), (r, 0)]);
        view.selected = Some(Row { parent: Parent::Node(e, 2), pos: 0 });
        view.selected_line = Some((r, 0));
        // "Hello." deleted, as the editor does it: Go is entry 0 now.
        let mut after = before.clone();
        let renumbered = remove(&mut after, Parent::Root, 0);
        let (was, now) = (fingerprint(&before), fingerprint(&after));
        assert_eq!(fingerprint(&read(&after)), now, "the same once read back");
        view.remember(Change {
            before: was,
            after: now,
            renumbered: Some(renumbered),
            found: false,
            bookmarks: Vec::new(),
            collapsed: Vec::new(),
        });
        view.selected = Some(Row { parent: Parent::Node(e, 2), pos: 0 });
        view.follow(key, &read(&after), 2);
        assert_eq!(view.bookmarks, [(e, 0)]);
        assert_eq!(view.collapsed, HashSet::from([(e, 0)]));
        assert_eq!(view.selected, None, "the link went with Who?");
        // Undone: back as they were, the deleted line's bookmark first.
        view.follow(key, &read(&before), 3);
        assert_eq!(view.bookmarks, [(e, 1), (e, 2)]);
        assert_eq!(view.collapsed, HashSet::from([(e, 2), (r, 0)]));
        // A text changed, seen; then undone twice unseen and redone
        // twice: the delete is followed through it.
        let mut text = before.clone();
        mg_module::dialog::node_mut(&mut text, e, 2)
            .unwrap()
            .set("Comment", Value::String(b"note".to_vec()));
        view.follow(key, &read(&text), 4);
        assert_eq!(view.bookmarks, [(e, 1), (e, 2)], "nothing numbered anew");
        view.follow(key, &read(&after), 7);
        assert_eq!(view.bookmarks, [(e, 0)]);
        // A change it doesn't know: the lines are found again in it.
        let mut other = after.clone();
        add_node(&mut other, Parent::Root, "New.");
        view.follow(key, &read(&other), 8);
        assert_eq!(view.bookmarks, [(e, 0)]);
        let mut elsewhere = new_dialog();
        add_node(&mut elsewhere, Parent::Root, "New.");
        add_node(&mut elsewhere, Parent::Root, "Go.");
        view.follow(key, &read(&elsewhere), 9);
        assert_eq!(view.bookmarks, [(e, 1)], "Go, numbered anew elsewhere");
        assert_eq!(view.collapsed, HashSet::from([(e, 1)]));
        // Taken back: as they were.
        view.follow(key, &read(&other), 10);
        assert_eq!(view.bookmarks, [(e, 0)]);
        // (Asked again at the same revision: not hashed again.)
        view.bookmarks.clear();
        view.follow(key, &before, 10);
        assert!(view.bookmarks.is_empty());
    }

    /// Two deletes that leave the same conversation (either of two lines
    /// alike): undo and redo go by the one done last.
    #[test]
    fn of_two_deletes_alike_the_last_is_followed() {
        let key = ResKey::parse("talk", ResType::DLG).unwrap();
        let e = Kind::Entry;
        let read = |g: &Gff| Gff::read(&g.to_bytes().unwrap()).unwrap();
        let mut before = new_dialog();
        add_node(&mut before, Parent::Root, "Same.");
        add_node(&mut before, Parent::Root, "Same.");
        let mut view = DialogView::default();
        view.follow(key, &before, 1);
        view.bookmarks = vec![(e, 1)];
        let delete = |view: &mut DialogView, pos: usize| {
            let mut after = before.clone();
            let renumbered = remove(&mut after, Parent::Root, pos);
            view.remember(Change {
                before: fingerprint(&before),
                after: fingerprint(&after),
                renumbered: Some(renumbered),
                found: false,
                bookmarks: Vec::new(),
                collapsed: Vec::new(),
            });
            after
        };
        // The first deleted: the bookmarked line is the only one now.
        let after = delete(&mut view, 0);
        view.follow(key, &read(&after), 2);
        assert_eq!(view.bookmarks, [(e, 0)]);
        view.follow(key, &read(&before), 3);
        assert_eq!(view.bookmarks, [(e, 1)]);
        // The second deleted (the same bytes after): its bookmark goes.
        let again = delete(&mut view, 1);
        assert_eq!(fingerprint(&again), fingerprint(&after));
        view.follow(key, &read(&again), 4);
        assert!(view.bookmarks.is_empty());
        view.follow(key, &read(&before), 5);
        assert_eq!(view.bookmarks, [(e, 1)]);
    }

    /// A file whose line owns the line it is under (no editor makes one)
    /// is drawn once round, and Right goes nowhere from where it stops.
    #[test]
    fn a_line_under_itself_is_drawn_once() {
        let mut g = new_dialog();
        let hello = add_node(&mut g, Parent::Root, "Hello.");
        let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who?");
        let back = new_link(hello, Parent::Node(Kind::Reply, who), false);
        let n = node_mut(&mut g, Kind::Reply, who).unwrap();
        n.set(Kind::Reply.links(), Value::List(vec![back]));
        let rows = shown_rows(&g, &HashSet::new());
        assert_eq!(rows.len(), 4, "Root, Hello, Who, Hello again");
        assert!(rows[3].children);
        assert_eq!(tree_step(&rows, 3, Key::ArrowRight, 10), TreeStep::None);
        assert_eq!(
            tree_step(&rows, 3, Key::ArrowLeft, 10),
            TreeStep::Fold((Kind::Entry, 0), false)
        );
    }
}
