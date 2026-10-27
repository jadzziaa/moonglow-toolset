//! The Conversation Editor (`.dlg`), laid out like Aurora's: the tree of
//! NPC lines (red, `[OWNER]` or the speaker's tag) and PC lines (blue), links
//! in grey; Add, Copy, Cut, Paste, Paste As Link, Delete, Expand/Collapse
//! All; the selected line's speaker and text; and the tabs Text Appears
//! When…, Actions Taken, Other Actions, Comments and Current File. Each
//! change is one undoable command.

use std::collections::HashSet;

use egui::{Color32, RichText, Ui};
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Gff, Struct, Value};
use mg_module::dialog::{
    ANIMATIONS, Branch, Kind, Parent, Renumbering, add_node, copy_branch, is_link, link_index,
    link_lines, links, move_link_to, new_dialog, node, paste_branch, remove, shift_link,
    word_count,
};
use mg_resman::ResKey;
use mg_schema::{GffValue, StructExt, jrl};

use crate::text::{decode, encode, with_english};
use crate::widgets::{FieldTarget, LocStringEdit, commit_text, resref_field};
use crate::{Action, Moonglow, Tab};

/// A selected row: a link under a parent (or nothing: Root).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub parent: Parent,
    pub pos: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataTab {
    #[default]
    Condition,
    Action,
    Other,
    Comments,
    File,
}

/// The editor's state for one conversation.
#[derive(Debug, Clone, Default)]
pub struct DialogView {
    pub selected: Option<Row>,
    pub collapsed: HashSet<(Kind, u32)>,
    pub tab: DataTab,
    /// Highlight lines with a comment, an action, a quest, an animation or a
    /// sound (Aurora's filter buttons).
    pub highlight: [bool; 5],
    pub token_picker: bool,
    /// Aurora's bottom tabs: Data, Bookmarks, Search.
    pub bottom: Bottom,
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
    /// as the toolbar's button of that name does it.
    pub menu: Option<Asked>,
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
}

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

/// A conversation's fingerprint: a hash of its bytes.
pub(crate) fn fingerprint(g: &Gff) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    g.to_bytes().unwrap_or_default().hash(&mut h);
    h.finish()
}

impl DialogView {
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
    fn follow(&mut self, key: ResKey, g: &Gff, revision: u64) {
        if self.seen.is_some_and(|(r, _)| r == revision) {
            return;
        }
        let now = fingerprint(g);
        let before = self.last.replace(g.clone());
        let Some((_, was)) = self.seen.replace((revision, now)) else { return };
        if was == now {
            return;
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
        }
        if !renumbered {
            return;
        }
        self.selected = selection.and_then(|(parent, shown)| {
            let (_, index) = shown?;
            let pos = links(g, parent).iter().position(|l| link_index(l) == index)?;
            Some(Row { parent, pos })
        });
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

/// The rows of the tree from top to bottom as [`tree`] draws them: Root,
/// then each line whose parents are open.
pub fn shown_rows(g: &Gff, collapsed: &HashSet<(Kind, u32)>) -> Vec<Shown> {
    fn walk(
        g: &Gff,
        parent: Parent,
        up: usize,
        collapsed: &HashSet<(Kind, u32)>,
        path: &mut HashSet<(Kind, u32)>,
        out: &mut Vec<Shown>,
    ) {
        let kind = parent.child_kind();
        for (pos, l) in links(g, parent).iter().enumerate() {
            let index = link_index(l);
            if node(g, kind, index).is_none() {
                continue;
            }
            let children = !is_link(l) && !links(g, Parent::Node(kind, index)).is_empty();
            let open = !collapsed.contains(&(kind, index));
            out.push(Shown {
                row: Some(Row { parent, pos }),
                line: Some((kind, index)),
                up: Some(up),
                children,
                open,
            });
            let here = out.len() - 1;
            // (A line's own lines are drawn under it once: in a file where
            // several lines own one, not again under each, which grows as
            // two to the power of how deep such lines go.)
            if children && open && path.insert((kind, index)) {
                walk(g, Parent::Node(kind, index), here, collapsed, path, out);
            }
        }
    }
    let children = !links(g, Parent::Root).is_empty();
    let mut out = vec![Shown { row: None, line: None, up: None, children, open: true }];
    walk(g, Parent::Root, 0, collapsed, &mut HashSet::new(), &mut out);
    out
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Bottom {
    #[default]
    Data,
    Bookmarks,
    Search,
}

/// Find / Replace in conversations.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DialogSearch {
    pub find: String,
    pub replace: String,
    pub match_case: bool,
    pub whole_word: bool,
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
/// Scrap).
#[derive(Debug, Clone, PartialEq)]
pub struct DialogClip {
    pub from: ResKey,
    pub branch: Branch,
    /// The copied line, for Paste As Link.
    pub target: (Kind, u32),
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
    Action::Apply(Command::new(what, vec![Edit::SetResource { key, data: g.to_bytes().ok() }]))
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

/// The conversation editor, over its OK and Cancel
/// (`blueprint::ok_cancel`).
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    // The editor's text at its own size (Options › Conversation Editor):
    // every kind of text in it, each as much larger as its lines are; and
    // in the lists and menus opened from it, which take the window's
    // style, for as long as the editor is drawn.
    let usual = ui.ctx().global_style();
    if let Some(size) = app.settings.dialog_font_size {
        let by = f32::from(size.clamp(6, 48)) / egui::TextStyle::Body.resolve(ui.style()).size;
        let mut style = (**ui.style()).clone();
        for font in style.text_styles.values_mut() {
            font.size *= by;
        }
        style.spacing.interact_size.y *= by;
        style.spacing.icon_width *= by;
        ui.set_style(style.clone());
        ui.ctx().set_global_style(style);
    }
    let targets = [(key, mg_edit::GffPath::root())];
    let pressed = app.ws.is_some().then(|| crate::blueprint::ok_cancel(ui, &targets[0], false));
    let pending = app.actions.len();
    editor(app, ui, key);
    let tab = crate::Tab::Dialog(key);
    crate::blueprint::window_drawn(app, pending, &tab, &targets, pressed.flatten());
    if app.settings.dialog_font_size.is_some() {
        ui.ctx().set_global_style(usual);
    }
}

fn editor(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let Some(ws) = &mut app.ws else { return };
    let g = match ws.doc(&key) {
        Ok(g) => g.clone(),
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return;
        }
    };
    let revision = ws.revision();
    // (Taken for the frame and put back at its end, not copied.)
    let mut view = app.dialog_views.remove(&key).unwrap_or_default();
    // The bookmarks, folded lines and selection follow their lines, if a
    // delete (or its undo) numbered them anew since the last frame.
    view.follow(key, &g, revision);
    let folds = view.collapsed.len();
    // A selection the conversation no longer has (undo) is dropped.
    view.selected = view.selected.filter(|r| r.pos < links(&g, r.parent).len());
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
    if keys_here
        && !typing
        && !choosing
        && view.input.is_none()
        && !egui::Popup::is_any_open(ui.ctx())
    {
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
    // (Aurora's control with the focus has them), and no text being typed:
    // Options › Keyboard's, and the platform's Copy, Cut and Paste.
    let here = (ui.ui_contains_pointer() || keys_here) && !typing;
    let keymap = &app.keymap;
    let pressed = |c: crate::keys::Cmd| here && ui.input(|i| keymap.pressed(i, c));
    let asked = view.menu.take();
    let (key_add, key_delete) = (
        pressed(crate::keys::Cmd::AddLine) || asked == Some(Asked::Add),
        pressed(crate::keys::Cmd::DeleteLine) || asked == Some(Asked::Delete),
    );
    let (key_find, key_find_next) =
        (pressed(crate::keys::Cmd::DialogFind), pressed(crate::keys::Cmd::DialogFindNext));
    let (key_copy, key_cut, key_paste) =
        if here { crate::widgets::copy_cut_paste(ui) } else { (false, false, false) };
    let (key_copy, key_cut, key_paste) = (
        key_copy || asked == Some(Asked::Copy),
        key_cut || asked == Some(Asked::Cut),
        key_paste || asked == Some(Asked::Paste),
    );
    let add_hint =
        keymap.titled("Add a line under the selection", crate::keys::Cmd::AddLine, ui.ctx());

    // Toolbar.
    ui.horizontal_wrapped(|ui| {
        let sel = view.selected;
        let add_parent = match sel {
            None => Some(Parent::Root),
            Some(r) if !is_link(&links(&g, r.parent)[r.pos]) => {
                let (k, i) = target_of(r);
                Some(Parent::Node(k, i))
            }
            Some(_) => None,
        };
        if (ui
            .add_enabled(add_parent.is_some(), egui::Button::new("Add"))
            .on_hover_text(add_hint)
            .clicked()
            || key_add)
            && let Some(p) = add_parent
        {
            if popup {
                view.input = Some(NewLine { parent: p, text: placeholder.clone(), shown: false });
            } else {
                // The new line is selected, its text edited in place.
                let mut ng = g.clone();
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
                target: target_of(r),
            })
        };
        if ui.add_enabled(sel.is_some(), egui::Button::new("Copy")).clicked()
            || (key_copy && sel.is_some())
        {
            app.dialog_clip = sel.and_then(copy);
            if app.dialog_clip.is_some() {
                crate::widgets::mark_clipboard(ui.ctx(), "conversation lines");
            }
        }
        let mut delete = false;
        if (ui.add_enabled(sel.is_some(), egui::Button::new("Cut")).clicked() || key_cut)
            && sel.is_some()
        {
            app.dialog_clip = sel.and_then(copy);
            if app.dialog_clip.is_some() {
                crate::widgets::mark_clipboard(ui.ctx(), "conversation lines");
            }
            delete = true;
        }
        let paste_parent = add_parent;
        let clip = app.dialog_clip.as_ref();
        let fits = |p: Parent| clip.is_some_and(|c| c.branch.kind == p.child_kind());
        let can_paste = paste_parent.is_some_and(fits);
        if (ui.add_enabled(can_paste, egui::Button::new("Paste")).clicked() || key_paste)
            && can_paste
            && let (Some(p), Some(c)) = (paste_parent, clip)
        {
            let mut ng = g.clone();
            if paste_branch(&mut ng, p, &c.branch, c.from == key) {
                view.selected = Some(Row { parent: p, pos: links(&ng, p).len() - 1 });
                actions.push(replace(key, "Paste lines", &ng));
            }
        }
        // Paste As Link: by default the selected line gets a link to the
        // copied one (Link Destination To Source); the other way round,
        // the copied line gets a link to the selected one.
        let selected_line = sel.filter(|r| !is_link(&links(&g, r.parent)[r.pos])).map(target_of);
        let pair = match (selected_line, clip.filter(|c| c.from == key)) {
            (Some(dest), Some(c)) => {
                let (from, to) =
                    if paste_source_first { (c.target, dest) } else { (dest, c.target) };
                (from.0.child() == to.0).then_some((from, to))
            }
            _ => None,
        };
        if (ui.add_enabled(pair.is_some(), egui::Button::new("Paste As Link")).clicked()
            || asked == Some(Asked::PasteLink))
            && let Some((from, to)) = pair
        {
            let mut ng = g.clone();
            if link_lines(&mut ng, from, to) {
                actions.push(replace(key, "Paste as link", &ng));
            }
        }
        if ui.add_enabled(sel.is_some(), egui::Button::new("Delete")).clicked() || key_delete {
            delete = true;
        }
        if delete && let Some(r) = sel {
            let mut ng = g.clone();
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
        // Up and down among the lines of its parent: the order the game
        // tries them in (Alt + the arrow keys, too).
        let siblings = sel.map_or(0, |r| links(&g, r.parent).len());
        for (label, up, arrow, hint) in [
            ("⏶", true, egui::Key::ArrowUp, "Move the line up among its parent's (Alt+Up)"),
            ("⏷", false, egui::Key::ArrowDown, "Move the line down among its parent's (Alt+Down)"),
        ] {
            let can = sel.is_some_and(|r| if up { r.pos > 0 } else { r.pos + 1 < siblings });
            let key_move = here && ui.input_mut(|i| i.consume_key(egui::Modifiers::ALT, arrow));
            let clicked =
                ui.add_enabled(can, egui::Button::new(label)).on_hover_text(hint).clicked();
            if (clicked || key_move)
                && can
                && let Some(r) = sel
            {
                let mut ng = g.clone();
                if let Some(pos) = shift_link(&mut ng, r.parent, r.pos, up) {
                    view.selected = Some(Row { parent: r.parent, pos });
                    let what = if up { "Move line up" } else { "Move line down" };
                    actions.push(replace(key, what, &ng));
                }
            }
        }
        ui.separator();
        if ui.button("Expand All").clicked() {
            view.collapsed.clear();
        }
        if ui.button("Collapse All").clicked() {
            for k in [Kind::Entry, Kind::Reply] {
                for i in 0..mg_module::dialog::nodes(&g, k).len() as u32 {
                    view.collapsed.insert((k, i));
                }
            }
        }
        ui.separator();
        for (i, label) in
            ["Comments", "Actions", "Quests", "Animations", "Sounds"].iter().enumerate()
        {
            ui.toggle_value(&mut view.highlight[i], *label)
                .on_hover_text(format!("Highlight lines with {}", label.to_lowercase()));
        }
        ui.separator();
        // Export and import (dialog_io): the lines back by CSV.
        let mut export = None;
        ui.menu_button("Export", |ui| {
            for f in mg_module::dialog_io::Format::ALL {
                if ui.button(format!("{}…", f.name())).clicked() {
                    export = Some(f);
                    ui.close();
                }
            }
        });
        if let Some(f) = export {
            let suggested =
                std::path::PathBuf::from(format!("{}.{}", key.resref, f.extensions()[0]));
            if let Some(path) =
                app.dialogs.save_file(crate::dialogs::FileKind::Conversation(f), Some(&suggested))
            {
                // (The lines as the module's own table reads them, if any.)
                let text = crate::text::from_windows_1252(&f.write(&g, &key.resref.to_string()));
                match std::fs::write(&path, text) {
                    Ok(()) => app.log.info(format!("Exported {key} to {}", path.display())),
                    Err(e) => app.log.error(format!("{}: {e}", path.display())),
                }
            }
        }
        if ui
            .button("Import Lines…")
            .on_hover_text("Read back the text, speakers and comments of a CSV export")
            .clicked()
            && let Some(path) = app.dialogs.open_file(
                crate::dialogs::FileKind::Conversation(mg_module::dialog_io::Format::Csv),
                None,
            )
        {
            let mut ng = g.clone();
            let read = (std::fs::read_to_string(&path).map_err(|e| e.to_string()))
                .map(|csv| crate::text::for_windows_1252(&csv));
            match read.and_then(|csv| mg_module::dialog_io::update_from_csv(&mut ng, &csv)) {
                Ok(0) => app.log.info(format!("{}: no lines changed", path.display())),
                Ok(n) => {
                    app.log.info(format!("{}: {n} line(s) changed", path.display()));
                    actions.push(replace(key, "Import lines", &ng));
                }
                Err(e) => app.log.error(format!("{}: {e}", path.display())),
            }
        }
        ui.separator();
        let mut scripts = !app.settings.dialog_hide_scripts;
        if ui
            .toggle_value(&mut scripts, "Scripts")
            .on_hover_text("Show each line's condition, action, journal update and sound")
            .changed()
        {
            app.settings.dialog_hide_scripts = !scripts;
        }
    });
    ui.separator();

    // The fields of the selection.
    // (At most seven tenths of the editor: the lines keep their room.)
    let most = (ui.available_height() * 0.7).max(80.0);
    let fields = egui::Panel::bottom(egui::Id::new(("dlg-data", key)))
        .resizable(true)
        .default_size(260.0f32.min(most))
        .max_size(most);
    fields.show(ui, |ui| {
        // As tall as it was dragged to, whatever is in it: a panel is as
        // tall as its content, and the pane went back to it (shorter, for
        // a line with less to show) after it was dragged taller.
        ui.set_min_height(ui.max_rect().height());
        let words = g.root.dword("NumWords").unwrap_or(0);
        // Find and Replace, and Find Next (Options › Keyboard): the
        // Search pane.
        if key_find || key_find_next {
            view.bottom = Bottom::Search;
            view.search.focus = key_find;
            view.search.again = key_find_next;
        }
        ui.horizontal(|ui| {
            ui.selectable_value(&mut view.bottom, Bottom::Data, "Data");
            ui.selectable_value(&mut view.bottom, Bottom::Bookmarks, "Bookmarks");
            ui.selectable_value(&mut view.bottom, Bottom::Search, "Search");
            ui.separator();
            let sel = view.selected.filter(|r| r.pos < links(&g, r.parent).len());
            if ui.add_enabled(sel.is_some(), egui::Button::new("Bookmark").small()).clicked()
                && let Some(r) = sel
            {
                let t = target_of(r);
                if let Some(i) = view.bookmarks.iter().position(|b| *b == t) {
                    view.bookmarks.remove(i);
                } else {
                    view.bookmarks.push(t);
                }
            }
            if ui.button("Test").on_hover_text("Click through the conversation").clicked() {
                view.test = Some(Vec::new());
            }
        });
        ui.separator();
        // (Shorter than what it shows, it scrolls.)
        let body = egui::ScrollArea::vertical().id_salt(("dlg-data-body", key)).auto_shrink(false);
        body.show(ui, |ui| {
            if view.bottom == Bottom::Bookmarks {
                for (kind, index) in view.bookmarks.clone() {
                    let Some(n) = node(&g, kind, index) else { continue };
                    if ui
                        .selectable_label(
                            false,
                            format!("{kind:?} {index}: {}", line_text(app.game.as_deref(), n)),
                        )
                        .clicked()
                    {
                        go_to_line(&mut view, &g, kind, index);
                    }
                }
                return;
            }
            if view.bottom == Bottom::Search {
                search_pane(app, ui, key, &g, &mut view, &mut actions);
                return;
            }
            // A row added this frame appears once the command has run.
            match view.selected.filter(|r| r.pos < links(&g, r.parent).len()) {
                None => {
                    ui.weak(format!("Root. {words} words in the file. Add adds an NPC greeting."));
                    file_tab(app, ui, key, &g, &mut actions);
                }
                Some(r) => {
                    let (kind, index) = target_of(r);
                    let link = links(&g, r.parent)[r.pos].clone();
                    let Some(n) = node(&g, kind, index).cloned() else { return };
                    let line = text(&n);
                    ui.weak(format!(
                        "Line: {} letters, {} words. File: {words} words.{}",
                        line.chars().count(),
                        line.split_whitespace().count(),
                        if is_link(&link) {
                            " This row is a link: the line is edited where it is owned."
                        } else {
                            ""
                        }
                    ));
                    ui.columns(2, |cols| {
                        text_panel(
                            app,
                            &mut cols[0],
                            key,
                            kind,
                            index,
                            &n,
                            &mut view,
                            &mut actions,
                        );
                        let ui = &mut cols[1];
                        ui.horizontal(|ui| {
                            for (t, label) in [
                                (DataTab::Condition, "Text Appears When…"),
                                (DataTab::Action, "Actions Taken"),
                                (DataTab::Other, "Other Actions"),
                                (DataTab::Comments, "Comments"),
                                (DataTab::File, "Current File"),
                            ] {
                                ui.selectable_value(&mut view.tab, t, label);
                            }
                        });
                        ui.separator();
                        egui::ScrollArea::vertical().id_salt(("dlg-tab", key)).show(ui, |ui| {
                            match view.tab {
                                DataTab::Condition => script_with_params(
                                    app,
                                    ui,
                                    key,
                                    link_path(r),
                                    "Active",
                                    "ConditionParams",
                                    &link,
                                    "cond",
                                    &mut actions,
                                ),
                                DataTab::Action => script_with_params(
                                    app,
                                    ui,
                                    key,
                                    node_path(kind, index),
                                    "Script",
                                    "ActionParams",
                                    &n,
                                    "act",
                                    &mut actions,
                                ),
                                DataTab::Other => {
                                    other_tab(app, ui, key, kind, index, &n, &mut actions)
                                }
                                DataTab::Comments => {
                                    let (path, label, value) = if is_link(&link) {
                                        (
                                            link_path(r),
                                            "LinkComment",
                                            decode(link.string("LinkComment").unwrap_or_default()),
                                        )
                                    } else {
                                        (
                                            node_path(kind, index),
                                            "Comment",
                                            decode(n.string("Comment").unwrap_or_default()),
                                        )
                                    };
                                    let id =
                                        egui::Id::new(("dlg-comment", key, kind, index, label));
                                    if let Some(v) =
                                        commit_text(app, ui, id, &value, true, f32::INFINITY)
                                    {
                                        actions.push(set(
                                            key,
                                            "Comment",
                                            path,
                                            label,
                                            Value::String(encode(&v)),
                                        ));
                                    }
                                }
                                DataTab::File => file_tab(app, ui, key, &g, &mut actions),
                            }
                        });
                    });
                }
            }
        });
    });

    // The tree, in the Options' colours.
    let hl = view.highlight;
    let mut dropped: Option<Drop> = None;
    let look = Look::new(&app.settings);
    let lines = egui::ScrollArea::both().id_salt(("dlg-tree", key)).auto_shrink([false, false]);
    let shown = lines.show(ui, |ui| {
        // Ctrl+Up and Ctrl+Down: a row up or down, the selection staying.
        if view.scroll != 0 {
            let pitch = ui.spacing().interact_size.y + ui.spacing().item_spacing.y;
            ui.scroll_with_delta(egui::vec2(0.0, -(view.scroll as f32) * pitch));
            view.scroll = 0;
        }
        let root = ui.selectable_label(view.selected.is_none(), RichText::new("Root").strong());
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
        let mut path = HashSet::new();
        tree(
            ui,
            app.game.as_deref(),
            key,
            &g,
            Parent::Root,
            1,
            &mut view,
            &mut path,
            hl,
            &look,
            &mut dropped,
        );
    });
    let pitch = ui.spacing().interact_size.y + ui.spacing().item_spacing.y;
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
    if let Some(d) = dropped {
        let mut ng = g.clone();
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
        let modal = egui::Modal::new(egui::Id::new(("dlg-input", key))).show(ui.ctx(), |ui| {
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
            ui.horizontal(|ui| {
                ok = ui.button("OK").on_hover_text("Accept changes").clicked()
                    || crate::widgets::enter(ui);
                cancel = crate::widgets::cancel_discard(ui);
            });
        });
        if ok {
            let mut ng = g.clone();
            let kind = input.parent.child_kind();
            let i = add_node(&mut ng, input.parent, "");
            if let Some(n) = ng.root.list_mut(kind.list()).and_then(|l| l.get_mut(i as usize)) {
                // (With plain line ends, as Aurora writes a line's text.)
                let text = crate::text::with_loc_line_ends(input.text.trim_end(), "");
                let t = with_english(Default::default(), &text);
                n.set("Text", Value::LocString(t));
            }
            let words = word_count(&ng);
            ng.root.set("NumWords", Value::Dword(words));
            if let Parent::Node(k, i) = input.parent {
                view.collapsed.remove(&(k, i));
            }
            actions.push(replace(key, "Add line", &ng));
        } else if !cancel && !modal.should_close() {
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
    view.was_typing = ui.ctx().text_edit_focused();
    view.selected_line = view.selected.and_then(|r| {
        links(&g, r.parent).get(r.pos).map(|l| (r.parent.child_kind(), link_index(l)))
    });
    app.dialog_views.insert(key, view);
    app.actions.extend(actions);
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
}

impl Look {
    fn new(s: &crate::Settings) -> Look {
        let rgb =
            |c: Option<[u8; 3]>, d: Color32| c.map_or(d, |[r, g, b]| Color32::from_rgb(r, g, b));
        Look {
            names: !s.dialog_hide_names,
            scripts: !s.dialog_hide_scripts,
            npc: rgb(s.dialog_npc_color, Color32::from_rgb(210, 70, 70)),
            pc: rgb(s.dialog_pc_color, Color32::from_rgb(90, 140, 230)),
        }
    }
}

/// Whether a drop links (Ctrl, or Cmd on macOS) rather than moves.
fn link_key(ui: &Ui) -> bool {
    ui.input(|i| i.modifiers.ctrl || i.modifiers.mac_cmd)
}

/// Draws the rows under a parent of conversation `key`; `path` stops
/// cycles through owning links.
#[allow(clippy::too_many_arguments)]
fn tree(
    ui: &mut Ui,
    game: Option<&mg_rules::GameData>,
    key: ResKey,
    g: &Gff,
    parent: Parent,
    depth: usize,
    view: &mut DialogView,
    path: &mut HashSet<(Kind, u32)>,
    hl: [bool; 5],
    look: &Look,
    dropped: &mut Option<Drop>,
) {
    let kind = parent.child_kind();
    for (pos, l) in links(g, parent).iter().enumerate() {
        let index = link_index(l);
        let Some(n) = node(g, kind, index) else { continue };
        let row = Row { parent, pos };
        let linked = is_link(l);
        let children = !linked && !links(g, Parent::Node(kind, index)).is_empty();
        let open = !view.collapsed.contains(&(kind, index));
        let label = row_label(game, g, kind, l, look.names);
        let cond = l.resref("Active").filter(|r| !r.is_empty());
        let color = if linked {
            Color32::GRAY
        } else if kind == Kind::Entry {
            look.npc
        } else {
            look.pc
        };
        let mut rich = RichText::new(label).color(color);
        if linked {
            rich = rich.italics();
        }
        if highlighted(n, hl) {
            rich = rich.background_color(Color32::from_rgba_unmultiplied(230, 200, 60, 70));
        }
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            if children {
                if ui.small_button(if open { "−" } else { "+" }).clicked() {
                    if open {
                        view.collapsed.insert((kind, index));
                    } else {
                        view.collapsed.remove(&(kind, index));
                    }
                }
            } else {
                ui.add_space(18.0);
            }
            let r =
                ui.selectable_label(view.selected == Some(row), rich).interact(egui::Sense::drag());
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
                    Drop { from, onto: parent, link, before: Some(pos) }
                } else {
                    Drop { from, onto: Parent::Node(kind, index), link, before: None }
                });
            }
            // Where a line held over this one would go: a line above it,
            // or a frame around it.
            if let Some(from) = r.dnd_hover_payload::<(ResKey, Row)>().and_then(ours)
                && from != row
            {
                let stroke = egui::Stroke::new(2.0, ui.visuals().selection.stroke.color);
                if !link_key(ui) && from.parent.child_kind() == kind {
                    ui.painter().hline(r.rect.x_range(), r.rect.top(), stroke);
                } else {
                    ui.painter().rect_stroke(r.rect, 2.0, stroke, egui::StrokeKind::Inside);
                }
            }
            r.context_menu(|ui| {
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
            if look.scripts {
                let marks = markers(l, n);
                if !marks.is_empty() {
                    ui.label(RichText::new(marks.join("  ")).weak().small());
                }
            } else if cond.is_some() {
                ui.weak("?");
            }
        });
        // (Once, as `shown_rows` has it.)
        if children && open && path.insert((kind, index)) {
            let under = Parent::Node(kind, index);
            tree(ui, game, key, g, under, depth + 1, view, path, hl, look, dropped);
        }
    }
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
    let areas = ws.module.areas().unwrap_or_default();
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
) {
    let path = node_path(kind, index);
    // Within its column, however narrow the window: a row too wide for it
    // wraps, rather than push the text under the script's side.
    let width = ui.available_width();
    ui.set_max_width(width);
    if kind == Kind::Entry {
        ui.horizontal_wrapped(|ui| {
            ui.label("Speaker Tag");
            let speaker = decode(n.string("Speaker").unwrap_or_default());
            let id = egui::Id::new(("dlg-speaker", key, index));
            let mut chosen = commit_text(app, ui, id, &speaker, false, 160.0);
            // The tags of the creatures placed in the module's areas (read
            // only while the list is open: it takes every area's objects).
            let r = crate::widgets::dropdown(("dlg-speaker-tags", key, index))
                .selected_text("")
                .width(24.0)
                .show_ui(ui, |ui| {
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
            ui.weak("(empty: the conversation's owner)");
        });
    }
    let ls = n.locstring("Text").cloned().unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label("Text");
        if ui.small_button("…").on_hover_text("Edit text in multiple languages").clicked() {
            app.loc_edit = Some(LocStringEdit::new(
                FieldTarget::new(key, path.clone(), "Text"),
                "Line text",
                &ls,
            ));
        }
        if ui.small_button("Token…").on_hover_text("Insert a token such as <FirstName>").clicked()
        {
            view.token_picker = true;
        }
    });
    let english = ls
        .text(crate::text::edit_language(), mg_core::Gender::Male)
        .map(|t| t.into_owned())
        .unwrap_or_default();
    let id = egui::Id::new(("dlg-text", key, kind, index));
    // Text from the talk table shows under the field; text typed in the
    // field is the line's own, said instead.
    let from_tlk = (english.is_empty() && !ls.strref.is_none())
        .then(|| app.game.as_deref().and_then(|g| g.string(ls.strref)))
        .flatten();
    // Without text in the language edited or a talk-table string: its
    // text in another language (English first), marked as not its own.
    let other =
        (english.is_empty() && ls.strref.is_none()).then(|| crate::text::elsewhere(&ls)).flatten();
    let shown = other.as_ref().map_or(english, |(_, text)| text.clone());
    let typed = match &other {
        Some((language, _)) => crate::widgets::borrowed_field(
            ui,
            language.name().unwrap_or("another language"),
            "This line's text is in another language: it has none in the language edited \
             (Options › Language). Typing here gives it text in that language",
            |ui| crate::widgets::commit_loc_text(app, ui, id, &shown, true, width),
        ),
        None => crate::widgets::commit_loc_text(app, ui, id, &shown, true, width),
    };
    if let Some(v) = typed {
        actions.push(set(
            key,
            "Line text",
            path.clone(),
            "Text",
            with_english(ls.clone(), &v).into_value(),
        ));
    }
    if let Some(t) = from_tlk {
        ui.weak(format!("From the talk table (string {}): {t}", ls.strref.0));
    }
    if view.token_picker {
        // (Those of the language edited: Options › General.)
        let tokens = crate::widgets::language_tokens(app, crate::text::edit_language());
        let mut close = false;
        egui::Window::new("Select Token")
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ui.ctx().content_rect().center())
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for t in &tokens {
                        if ui.selectable_label(false, t).clicked() {
                            let v = crate::widgets::with_token(ui.ctx(), id, &shown, t);
                            actions.push(set(
                                key,
                                "Insert token",
                                path.clone(),
                                "Text",
                                with_english(ls.clone(), &v).into_value(),
                            ));
                            close = true;
                        }
                    }
                });
                if crate::widgets::cancel(ui) {
                    close = true;
                }
            });
        if close {
            view.token_picker = false;
        }
    }
}

/// A script field with its parameters and a preview of its source.
#[allow(clippy::too_many_arguments)]
fn script_with_params(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    path: GffPath,
    label: &str,
    list: &str,
    s: &Struct,
    salt: &str,
    actions: &mut Vec<Action>,
) {
    let current = s.resref(label).unwrap_or(ResRef::EMPTY);
    ui.horizontal(|ui| {
        ui.label("Script");
        let id = egui::Id::new(("dlg-script", key, salt, path.to_string()));
        if let Some(v) =
            resref_field(app, ui, id, current, "Select a script", &[ResType::NSS, ResType::NCS])
        {
            actions.push(set(key, "Script", path.clone(), label, Value::resref(v)));
        }
        if let Some(name) = crate::widgets::edit_script_button(app, ui, id, current) {
            actions.push(Action::EditScript { name, condition: label == "Active" });
        }
    });
    // (As wide as they are, not as the page: the page stretches what is
    // put in it, which squeezed the parameters' fields to a few letters.)
    let own_width = egui::Layout::top_down(egui::Align::Min);
    let wizard = ui
        .with_layout(own_width, |ui| {
            ui.button("Script Wizard…").on_hover_text("Write a new script from a few choices")
        })
        .inner;
    if wizard.clicked() {
        app.open_script_wizard(key, path.clone(), label, salt == "cond");
    }
    // The pairs being edited live in a buffer until a field loses focus or a
    // row is added or removed; with no field focused the file's pairs show.
    let stored = params(s, list);
    let buf_id = egui::Id::new(("dlg-params-buf", key, salt, path.to_string()));
    let mut pairs: Vec<(String, String)> =
        ui.ctx().data_mut(|d| d.get_temp(buf_id)).unwrap_or_else(|| stored.clone());
    ui.label("Parameters");
    let mut commit = false;
    let mut focused = false;
    let mut remove_at = None;
    // A name's field and a value's, sharing the page's width: the value
    // the wider.
    let room = (ui.available_width() - 110.0).max(250.0);
    let name_width = (room * 0.35).clamp(110.0, 260.0);
    let value_width = (room - name_width).clamp(140.0, 520.0);
    let mut add = false;
    ui.with_layout(own_width, |ui| {
        for (i, (k, v)) in pairs.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                // (First in its row: at the row's end it was past the
                // page's edge where the page is narrow.)
                if ui.small_button("−").on_hover_text("Remove the parameter").clicked() {
                    remove_at = Some(i);
                }
                let name = egui::TextEdit::singleline(k).desired_width(name_width);
                let rk = ui.add(name.hint_text("name"));
                let value = egui::TextEdit::singleline(v).desired_width(value_width);
                let rv = ui.add(value.hint_text("value"));
                crate::widgets::leaves_at_a_press(ui.ctx(), &rk);
                crate::widgets::leaves_at_a_press(ui.ctx(), &rv);
                commit |= rk.lost_focus() || rv.lost_focus();
                focused |= crate::widgets::has_keys(&rk) || crate::widgets::has_keys(&rv);
            });
        }
        add = ui
            .small_button("+")
            .on_hover_text("Add a parameter (EE: read with GetScriptParam)")
            .clicked();
    });
    if let Some(i) = remove_at {
        pairs.remove(i);
        commit = true;
    }
    if add {
        pairs.push((String::new(), String::new()));
        commit = true;
    }
    if commit {
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
        let source = app.script_source(&ResKey::new(current, ResType::NSS));
        ui.separator();
        ui.label("Script Preview");
        match source {
            Some(src) => {
                egui::ScrollArea::both().id_salt(("dlg-preview", salt)).max_height(140.0).show(
                    ui,
                    |ui| {
                        ui.monospace(src);
                    },
                );
            }
            None => {
                ui.weak("No source for this script.");
            }
        }
    }
}

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
    egui::Grid::new(("dlg-other", key, kind, index)).num_columns(2).spacing([10.0, 6.0]).show(
        ui,
        |ui| {
            crate::widgets::field_label(ui, "Play Animation");
            let anim = n.dword("Animation").unwrap_or(0);
            let shown = ANIMATIONS
                .iter()
                .find(|a| a.1 == anim)
                .map_or_else(|| anim.to_string(), |a| a.0.to_string());
            let mut pick = anim;
            let values = ANIMATIONS.map(|a| a.1);
            crate::widgets::dropdown(("dlg-anim", key, kind, index)).selected_text(shown).show(
                ui,
                &mut pick,
                &values,
                |v| ANIMATIONS.iter().find(|a| a.1 == *v).map_or("", |a| a.0),
            );
            if pick != anim {
                actions.push(set(key, "Animation", path.clone(), "Animation", Value::Dword(pick)));
            }
            ui.end_row();

            crate::widgets::field_label(ui, "Play Sound");
            let sound = n.resref("Sound").unwrap_or(ResRef::EMPTY);
            let id = egui::Id::new(("dlg-sound", key, kind, index));
            ui.horizontal(|ui| {
                if let Some(v) = resref_field(app, ui, id, sound, "Select a sound", &[ResType::WAV])
                {
                    actions.push(set(key, "Sound", path.clone(), "Sound", Value::resref(v)));
                }
                if ui.add_enabled(!sound.is_empty(), egui::Button::new("Play")).clicked() {
                    app.play_sound(crate::audio::Channel::Preview, sound, 1.0, false);
                }
            });
            ui.end_row();

            // Journal categories and entries of the module.
            let journal: Vec<Struct> = app
                .ws
                .as_mut()
                .and_then(|w| w.doc(&ResKey::parse("module", ResType::JRL)?).ok())
                .map(|g| g.root.items(&jrl::CATEGORIES).to_vec())
                .unwrap_or_default();
            let quest = decode(n.string("Quest").unwrap_or_default());
            ui.label("Journal");
            // (The row of none is the empty tag.)
            let tags: Vec<String> = std::iter::once(String::new())
                .chain(journal.iter().map(|c| decode(c.read(&jrl::categories::TAG).as_bytes())))
                .collect();
            let mut pick = quest.clone();
            crate::widgets::dropdown(("dlg-quest", key, kind, index))
                .selected_text(if quest.is_empty() { "(none)".into() } else { quest.clone() })
                .show(ui, &mut pick, &tags, |t| {
                    if t.is_empty() { "(none)".to_string() } else { t.clone() }
                });
            if pick.is_empty() && !quest.is_empty() {
                actions.push(set(key, "Journal", path.clone(), "Quest", Value::String(Vec::new())));
            } else if pick != quest
                && let Some(c) = (journal.iter())
                    .find(|c| decode(c.read(&jrl::categories::TAG).as_bytes()) == pick)
            {
                let mut edits = vec![Edit::SetField {
                    key,
                    path: path.clone(),
                    label: "Quest".into(),
                    value: Some(Value::String(encode(&pick))),
                }];
                let first = mg_module::journal::entries(c)
                    .first()
                    .map(|e| e.read(&jrl::categories::entry_list::ID));
                edits.push(Edit::SetField {
                    key,
                    path: path.clone(),
                    label: "QuestEntry".into(),
                    value: Some(Value::Dword(first.unwrap_or(0))),
                });
                actions.push(Action::Apply(Command::new("Journal", edits)));
            }
            ui.end_row();
            if !quest.is_empty() {
                ui.label("Journal entry");
                let entry = n.dword("QuestEntry").unwrap_or(0);
                let category = journal
                    .iter()
                    .find(|c| decode(c.read(&jrl::categories::TAG).as_bytes()) == quest);
                let entries = category.map(mg_module::journal::entries).unwrap_or(&[]);
                let ids: Vec<u32> =
                    entries.iter().map(|e| e.read(&jrl::categories::entry_list::ID)).collect();
                let mut pick = entry;
                crate::widgets::dropdown(("dlg-quest-entry", key, kind, index))
                    .selected_text(entry.to_string())
                    .show_with(
                        ui,
                        &mut pick,
                        || ids.clone(),
                        |ui, pick| {
                            for e in entries {
                                let id = e.read(&jrl::categories::entry_list::ID);
                                let t = e.read(&jrl::categories::entry_list::TEXT);
                                let t = crate::text::shown_text(&t);
                                ui.selectable_value(pick, id, format!("{id}: {t}"));
                            }
                        },
                    );
                if pick != entry {
                    let id = Value::Dword(pick);
                    actions.push(set(key, "Journal entry", path.clone(), "QuestEntry", id));
                }
                ui.end_row();
            }
        },
    );
}

fn file_tab(app: &mut Moonglow, ui: &mut Ui, key: ResKey, g: &Gff, actions: &mut Vec<Action>) {
    egui::Grid::new(("dlg-file", key)).num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        for (label, field) in [("Normal end", "EndConversation"), ("Aborted", "EndConverAbort")] {
            crate::widgets::field_label(ui, label);
            let current = g.root.resref(field).unwrap_or(ResRef::EMPTY);
            let id = egui::Id::new(("dlg-end", key, field));
            if let Some(v) =
                resref_field(app, ui, id, current, "Select a script", &[ResType::NSS, ResType::NCS])
            {
                actions.push(set(key, label, GffPath::root(), field, Value::resref(v)));
            }
            ui.end_row();
        }
        let mut zoom = g.root.byte("PreventZoomIn").unwrap_or(0) != 0;
        ui.label("");
        if ui.checkbox(&mut zoom, "Stop camera zoom in").changed() {
            actions.push(set(
                key,
                "Camera zoom",
                GffPath::root(),
                "PreventZoomIn",
                Value::Byte(u8::from(zoom)),
            ));
        }
        ui.end_row();
    });
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
        Named::Cancelled => None,
        Named::Chosen(k) => {
            app.actions.push(Action::Apply(Command::new(
                format!("New conversation {k}"),
                vec![Edit::SetResource { key: k, data: Some(new_file()) }],
            )));
            app.actions.push(Action::OpenTab(Tab::Dialog(k)));
            None
        }
    };
}

fn matches(text: &str, find: &str, s: &DialogSearch) -> bool {
    let o = crate::script_tools::SearchOptions {
        match_case: s.match_case,
        whole_word: s.whole_word,
        backwards: false,
    };
    crate::script_tools::find(text, find, 0, &o).is_some()
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
) {
    let s = &mut view.search;
    egui::Grid::new(("dlg-search", key)).num_columns(2).show(ui, |ui| {
        crate::widgets::field_label(ui, "Find What");
        let field = ui.text_edit_singleline(&mut s.find);
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
        ui.end_row();
        crate::widgets::field_label(ui, "Replace With");
        ui.text_edit_singleline(&mut s.replace);
        ui.end_row();
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.match_case, "Match Case");
        ui.checkbox(&mut s.whole_word, "Match Whole Word Only");
        ui.checkbox(&mut s.all_files, "All Files in Module");
    });
    let mut find = false;
    let mut replace_all = false;
    ui.horizontal(|ui| {
        find = ui.add_enabled(!s.find.is_empty(), egui::Button::new("Find")).clicked();
        find |= std::mem::take(&mut s.again) && !s.find.is_empty();
        replace_all = ui
            .add_enabled(!s.find.is_empty() && !s.all_files, egui::Button::new("Replace All"))
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
            let doc = if k == key {
                Some(g.clone())
            } else {
                app.ws.as_mut().and_then(|w| w.doc(&k).ok().cloned())
            };
            let Some(d) = doc else { continue };
            for kind in [Kind::Entry, Kind::Reply] {
                for (i, n) in mg_module::dialog::nodes(&d, kind).iter().enumerate() {
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
        let mut ng = g.clone();
        let mut n = 0;
        for kind in [Kind::Entry, Kind::Reply] {
            if let Some(list) = ng.root.list_mut(kind.list()) {
                for line in list {
                    let t = text(line);
                    let o = crate::script_tools::SearchOptions {
                        match_case: s.match_case,
                        whole_word: s.whole_word,
                        backwards: false,
                    };
                    let (new, count) =
                        crate::script_tools::replace_all(&t, &s.find, &s.replace, &o);
                    if count > 0 {
                        let ls = line.locstring("Text").cloned().unwrap_or_default();
                        line.set("Text", with_english(ls, &new).into_value());
                        n += count;
                    }
                }
            }
        }
        if n > 0 {
            actions.push(replace(key, "Replace text", &ng));
        }
        app.log.info(format!("Replaced {n} in {key}"));
    }
    ui.separator();
    for (k, kind, index, t) in s.results.clone() {
        let label = if k == key {
            format!("{kind:?} {index}: {t}")
        } else {
            format!("{k}: {kind:?} {index}: {t}")
        };
        if ui.selectable_label(false, label).clicked() {
            if k == key {
                if let Some((parent, pos)) = mg_module::dialog::owner(g, kind, index) {
                    view.selected = Some(Row { parent, pos });
                }
            } else {
                actions.push(Action::OpenTab(Tab::Dialog(k)));
            }
        }
    }
}

impl Moonglow {
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
        let stem: String = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .take(16)
            .collect();
        let stem = if stem.is_empty() { "conversation".to_string() } else { stem };
        let free =
            |name: &str| ResKey::parse(name, ResType::DLG).filter(|k| !ws.module.contains(k));
        let key = free(&stem).or_else(|| {
            (1..1000).find_map(|n| {
                let suffix = format!("{n:03}");
                let base: String = stem.chars().take(16 - suffix.len()).collect();
                free(&format!("{base}{suffix}"))
            })
        })?;
        let lines = mg_module::dialog::nodes(&g, Kind::Entry).len()
            + mg_module::dialog::nodes(&g, Kind::Reply).len();
        self.actions.push(Action::Apply(mg_edit::Command::new(
            format!("Import conversation {}", key.resref),
            vec![mg_edit::Edit::SetResource { key, data: g.to_bytes().ok() }],
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
        .on_hover_text("What this condition is taken to return; click to switch")
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
        let mut close = crate::widgets::escape_closes(ui.ctx(), &title);
        egui::Window::new(title)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ui.ctx().content_rect().center())
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                // What was said so far.
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    for (k, i) in &path {
                        if let Some(n) = node(&g, *k, *i) {
                            let who = if *k == Kind::Entry { "NPC" } else { "You" };
                            ui.weak(format!("{who}: {}", line_text(game, n)));
                        }
                    }
                });
                ui.separator();
                // The NPC's turn: from the root, or after the last reply.
                let parent = match path.last() {
                    None => Parent::Root,
                    Some(&(k, i)) => Parent::Node(k, i),
                };
                let candidates = links(&g, parent).to_vec();
                let spoken = npc_line(&g, parent, &assume);
                // The NPC lines before the one said were passed over.
                for l in &candidates {
                    let i = link_index(l);
                    let Some(n) = node(&g, Kind::Entry, i) else { continue };
                    ui.horizontal(|ui| {
                        if Some(i) == spoken {
                            ui.label(
                                RichText::new(format!("NPC: {}", line_text(game, n))).strong(),
                            );
                        } else {
                            ui.weak(format!("(not said) {}", line_text(game, n)));
                        }
                        if let Some(c) = condition(l) {
                            condition_toggle(ui, &c, &mut assume);
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
                    footer(ui, &mut path, &mut close);
                    return;
                };
                ui.separator();
                // The player's replies.
                let replies = links(&g, Parent::Node(Kind::Entry, entry)).to_vec();
                if replies.is_empty() {
                    ui.weak("[END DIALOGUE]");
                }
                // Numbered as the game numbers the ones it offers.
                let mut number = 0;
                for l in &replies {
                    let i = link_index(l);
                    let Some(n) = node(&g, Kind::Reply, i) else { continue };
                    let t = line_text(game, n);
                    let t = if t.is_empty() { "[CONTINUE]".to_string() } else { t };
                    let shown = passes(l, &assume);
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
                            condition_toggle(ui, &c, &mut assume);
                        }
                        let e = effects(l, n);
                        if !e.is_empty() {
                            ui.weak(e);
                        }
                    });
                }
                footer(ui, &mut path, &mut close);
            });
        view.test = if close { None } else { Some(path) };
        view.assume = assume;
    }
}

/// Back (a turn: the reply and the NPC line before it) and Done.
fn footer(ui: &mut Ui, path: &mut Vec<(Kind, u32)>, close: &mut bool) {
    ui.separator();
    ui.horizontal(|ui| {
        if ui.add_enabled(!path.is_empty(), egui::Button::new("<-- Back")).clicked() {
            path.pop();
            path.pop();
        }
        if ui.button("Done").clicked() {
            *close = true;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Key;
    use mg_module::dialog::{add_link, new_link, node_mut};

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
