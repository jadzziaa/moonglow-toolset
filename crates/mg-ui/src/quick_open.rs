//! Quick Open (M3's T8; SPEC sections 6, 12 and 13: the Command Palette
//! extended to resources): the Top Bar's field "Go to resource or command"
//! and the popover of what it finds under it.
//!
//! What it finds, best first, in three groups:
//!
//! 1. the module's resources, by ResRef and, for an area or a blueprint,
//!    by its name (what the Module panel's Filter matches; it also keeps an
//!    area for the objects placed in it, which Quick Open leaves out);
//! 2. the commands: those of the menus, the plugins' commands and entries
//!    and the project's commands, as the Command Palette listed them
//!    (`commands::found`), grayed where they cannot run now;
//! 3. the load order's resources (the game's, the haks', override's), which
//!    open read-only as the Resources panel opens them.
//!
//! Within a group (the rule [`rank`] keeps, held by the unit tests): the
//! whole of what is typed (its words joined by one space, case and
//! accents aside) equal to a field, then a field that starts with it, then
//! one where it starts a word (after a character that is no letter or
//! digit: `formosa` in `m1q3_formosa_9`), then one that holds it anywhere;
//! then rows that hold every word apart (`sword iron` in "Iron Sword"),
//! and last a command found only with its menu's name (`mod build` finds
//! Verify Module in the Build menu). At one tier, a match on the name
//! before one on the ResRef; then the shorter name, then the name and the
//! ResRef in alphabetical order; last, of two files of one ResRef, the one
//! with an editor (a script's source before its compiled form).
//!
//! What is typed after a `>` looks among the commands alone (Ctrl+Shift+P
//! and Help › Command Palette… open the field with it). SPEC names no other
//! prefix. With nothing typed: the areas of this module opened lately, most
//! recent first (`Settings::recent_areas`); the areas of other recent
//! modules cannot be opened without opening their module, so they are not
//! offered. With no module open: the recent modules, then the commands
//! that work without one.
//!
//! The index is made lazily, the first time the field opens: the module's
//! part again only when the workspace's revision (`Workspace::revision`),
//! its count of resources or its file changes ([`ModuleStamp`]); the load
//! order's part again only when its generation (`Moonglow::load_order`)
//! changes. Names (a blueprint's means reading it) are taken a few at a
//! time, within [`BUDGET`] a frame, and the load order's part is made on a
//! thread of its own where the window runs its work in the background
//! (`Moonglow::background_jobs`): meanwhile the field works on what is in
//! hand and its last row says "Indexing…". At rest (closed) it costs
//! nothing but the field's drawing.

use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use egui::{Rect, Ui};
use mg_core::ResType;
use mg_core::text::{Needle, fold};
use mg_resman::ResKey;

use crate::commands::{self, Choice, Id};
use crate::icons::Icon;
use crate::{Action, Moonglow, Tab};

/// The field's accessible name (and the icon button's, in a narrow
/// window).
pub const NAME: &str = "Quick Open";
/// What the field says while empty (`Main.png`).
pub const HINT: &str = "Go to resource or command";
/// The popover's group: its rows are found inside it.
pub const RESULTS: &str = "Quick Open Results";
/// The most rows the popover shows; a last line says how many more.
pub const MOST: usize = 50;
/// What restricts the search to the commands, first in the field.
pub const COMMANDS: &str = ">";
/// The time a frame gives to naming the module's areas and blueprints,
/// and the most a build may take on the frame before it goes to a thread.
pub const BUDGET: Duration = Duration::from_millis(2);
/// The popover's rows' height (`.menu-row`).
const ROW: f32 = mg_uikit::metrics::MENU_ROW;
/// The gap between the field and the popover (`.popover.below`: 4 px).
const BELOW: f32 = 4.0;
/// The least room kept between the popover and the window's edges.
const EDGE: f32 = 8.0;
/// The popover's widest where its rows want more than the field's width.
const WIDEST: f32 = 640.0;

/// The field's id (its text edit's): the same in the Top Bar and over it.
pub fn field_id() -> egui::Id {
    egui::Id::new("quick-open-field")
}

/// How well a row matches, best first ([`tier_of`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Tier {
    Exact,
    Prefix,
    WordStart,
    Substring,
    /// Every word, apart or in another order.
    Scattered,
    /// A command found only with its menu's name.
    WithMenu,
}

/// Which of a row's fields matched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Field {
    Name,
    ResRef,
}

/// How `field` holds `query` (both folded, the query's words joined by
/// one space): the whole field, its start, a word's start (after a
/// character that is no letter or digit), anywhere; `None` where it does
/// not hold it whole.
pub(crate) fn tier_of(field: &str, query: &str) -> Option<Tier> {
    if query.is_empty() {
        return None;
    }
    if field == query {
        return Some(Tier::Exact);
    }
    if field.starts_with(query) {
        return Some(Tier::Prefix);
    }
    let mut found = None;
    for (at, _) in field.match_indices(query) {
        if field[..at].chars().next_back().is_none_or(|c| !c.is_alphanumeric()) {
            return Some(Tier::WordStart);
        }
        found = Some(Tier::Substring);
    }
    found
}

/// What is typed, read: whether it looks among the commands alone (a
/// `>` first), and the rest.
pub(crate) fn parse(text: &str) -> (bool, &str) {
    let text = text.trim_start();
    match text.strip_prefix(COMMANDS) {
        Some(rest) => (true, rest.trim()),
        None => (false, text.trim()),
    }
}

/// A query's text as [`tier_of`] takes it: folded, its words joined by
/// one space.
pub(crate) fn query_of(needle: &Needle) -> String {
    needle.as_str().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A resource in the index: the module's or the load order's.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) key: ResKey,
    /// Its name (an area's, a blueprint's) where it has one, and folded.
    name: Option<String>,
    name_fold: String,
    /// Its ResRef as typed (ASCII, lower case).
    resref: String,
    /// The layer the game reads it from (a load-order resource's).
    layer: Option<Arc<str>>,
}

impl Entry {
    pub(crate) fn new(key: ResKey, name: Option<String>, layer: Option<Arc<str>>) -> Entry {
        let mut e = Entry {
            key,
            name: None,
            name_fold: String::new(),
            resref: key.resref.to_string().to_ascii_lowercase(),
            layer,
        };
        e.set_name(name);
        e
    }

    fn set_name(&mut self, name: Option<String>) {
        let name = name.filter(|n| !n.trim().is_empty());
        self.name_fold = name.as_deref().map(fold).unwrap_or_default();
        self.name = name;
    }

    /// Its file's name (`m1q3_formosa_9.are`).
    fn file(&self) -> String {
        match self.key.restype.extension() {
            Some(ext) => format!("{}.{ext}", self.resref),
            None => self.resref.clone(),
        }
    }

    /// Its name as the popover shows it: its own, else its file's.
    fn shown(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.file())
    }

    /// Whether it holds every word, in its name or its ResRef.
    fn finds(&self, needle: &Needle) -> bool {
        needle.finds_in(&[&self.name_fold, &self.resref])
    }

    /// How it matches (it holds every word: [`Entry::finds`]).
    fn score(&self, needle: &Needle, query: &str) -> (Tier, Field) {
        let name = (!self.name_fold.is_empty())
            .then(|| tier_of(&self.name_fold, query))
            .flatten()
            .map(|t| (t, Field::Name));
        let resref = tier_of(&self.resref, query).map(|t| (t, Field::ResRef));
        match (name, resref) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) | (None, Some(a)) => a,
            (None, None) => {
                let by_name = !self.name_fold.is_empty() && needle.finds_in(&[&self.name_fold]);
                (Tier::Scattered, if by_name { Field::Name } else { Field::ResRef })
            }
        }
    }
}

/// A row's place in its group: tier, field, the name's length, the name
/// and the ResRef; then, between two files of one ResRef, the one
/// with an editor first (a script's source before its compiled form),
/// and the type's extension (the order [`rank`] sorts by).
pub(crate) type Rank<'a> = (Tier, Field, usize, &'a str, &'a str, bool, &'a str);

/// The entries (of `within`, where given: those an earlier, shorter query
/// found) that hold every word.
pub(crate) fn matches(entries: &[Entry], within: Option<&[u32]>, needle: &Needle) -> Vec<u32> {
    match within {
        Some(within) => {
            within.iter().copied().filter(|&i| entries[i as usize].finds(needle)).collect()
        }
        None => (0..entries.len() as u32).filter(|&i| entries[i as usize].finds(needle)).collect(),
    }
}

/// The best `most` of `found` (indices into `entries`, all of which hold
/// every word), in order: [`Rank`]'s. Only those taken are sorted.
pub(crate) fn rank(
    entries: &[Entry],
    found: &[u32],
    needle: &Needle,
    query: &str,
    most: usize,
) -> Vec<u32> {
    if most == 0 {
        return Vec::new();
    }
    let mut ranked: Vec<(Rank<'_>, u32)> = found
        .iter()
        .map(|&i| {
            let e = &entries[i as usize];
            let (tier, field) = e.score(needle, query);
            let name = if e.name_fold.is_empty() { e.resref.as_str() } else { &e.name_fold };
            let t = e.key.restype;
            let no_editor = Tab::for_resource(e.key).is_none();
            let ext = t.extension().unwrap_or_default();
            ((tier, field, name.chars().count(), name, e.resref.as_str(), no_editor, ext), i)
        })
        .collect();
    if ranked.len() > most {
        ranked.select_nth_unstable_by(most - 1, |a, b| a.0.cmp(&b.0));
        ranked.truncate(most);
    }
    ranked.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    ranked.into_iter().map(|(_, i)| i).collect()
}

/// A command as the popover ranks it: the palette's row, its name and
/// menu folded.
#[derive(Clone, Debug)]
pub(crate) struct CommandRow {
    pub(crate) row: commands::Row,
    name_fold: String,
    menu_fold: String,
}

impl CommandRow {
    pub(crate) fn new(row: commands::Row) -> CommandRow {
        CommandRow { name_fold: fold(&row.label), menu_fold: fold(&row.menu), row }
    }

    /// How it matches, if it does: by its name, else (last) with its menu's.
    pub(crate) fn score(&self, needle: &Needle, query: &str) -> Option<Tier> {
        tier_of(&self.name_fold, query)
            .or_else(|| needle.finds_in(&[&self.name_fold]).then_some(Tier::Scattered))
            .or_else(|| {
                needle.finds_in(&[&self.name_fold, &self.menu_fold]).then_some(Tier::WithMenu)
            })
    }
}

/// The commands found, best first (the rule of the module's comment;
/// commands that rank alike keep the menus' order).
pub(crate) fn rank_commands(rows: &[CommandRow], needle: &Needle, query: &str) -> Vec<usize> {
    let mut found: Vec<(Tier, usize, usize)> = rows
        .iter()
        .enumerate()
        .filter_map(|(i, r)| r.score(needle, query).map(|t| (t, r.name_fold.chars().count(), i)))
        .collect();
    // (Stable: the menus' order among equals.)
    found.sort_by_key(|(t, len, _)| (*t, *len));
    found.into_iter().map(|(_, _, i)| i).collect()
}

/// What choosing a row does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    /// A resource of the module: opened as the Module panel opens it.
    Resource(ResKey),
    /// A load-order resource: opened read-only, as the Resources panel
    /// opens it.
    Game(ResKey),
    /// A command, run as its menu row runs it.
    Command(Choice),
    /// A recent module, opened as File › Recent Modules opens it.
    Module(PathBuf),
}

/// A row of the popover.
#[derive(Clone, Debug)]
pub(crate) struct Hit {
    pub(crate) target: Target,
    pub(crate) name: String,
    /// Dim after the name: the ResRef and type, a command's menu, a
    /// load-order resource's layer, a recent module's folder.
    pub(crate) detail: String,
    pub(crate) icon: Icon,
    /// A command's key, as its key cap shows it.
    pub(crate) key: String,
    pub(crate) enabled: bool,
    /// Why it cannot run now (a grayed command's tip).
    pub(crate) why: String,
}

/// What the popover shows for what is typed.
#[derive(Clone, Debug, Default)]
pub(crate) struct Found {
    pub(crate) hits: Vec<Hit>,
    /// How many more were found than shown.
    pub(crate) more: usize,
    /// The load order's part of the index is still being made.
    pub(crate) indexing: bool,
}

/// What the module's part of the index was made from: the workspace's
/// revision, its count of resources (one put in past the workspace's
/// commands, as a file read again from outside, changes no revision: the
/// module tree keys its lists by both, too) and the module's file.
type ModuleStamp = (u64, usize, Option<PathBuf>);

/// The index, made when first asked for and again only when what it was
/// made from changed.
#[derive(Debug, Default)]
pub(crate) struct Index {
    module_for: Option<ModuleStamp>,
    /// The module's own resources.
    own: Vec<Entry>,
    /// How many of `own` have been looked at for a name.
    named: usize,
    /// The load order's generation (`Moonglow::load_order`) `game` was
    /// made for.
    game_for: Option<u64>,
    game: Arc<Game>,
    /// The layers (label and fingerprint, the module's aside) `game` was
    /// made from.
    game_layers: Option<Vec<(String, u64)>>,
    /// A build of the load order's part on a thread: for which
    /// generation, and where it is handed back.
    building: Option<(u64, mpsc::Receiver<(Game, Duration)>)>,
    /// How many times each part was made (for the tests).
    pub(crate) module_builds: u32,
    pub(crate) game_builds: u32,
    /// How long the last build of the load order's part took, and of the
    /// module's.
    pub(crate) game_took: Duration,
    pub(crate) module_took: Duration,
}

/// The load order's resources as the index holds them: the copy the game
/// reads of each, without the module's own layer (its resources are the
/// module's group).
pub(crate) fn game_entries(resman: &mg_resman::ResMan) -> Game {
    let labels: Vec<Arc<str>> = resman.layers().iter().map(|l| Arc::from(shown_layer(l))).collect();
    let module = resman.layers().iter().position(|l| l.label == mg_resman::MODULE_LABEL);
    let entries = resman
        .entries()
        .into_iter()
        .filter(|(_, layer)| Some(*layer) != module)
        .map(|(key, layer)| Entry::new(key, None, Some(labels[layer].clone())))
        .collect();
    Game::new(entries)
}

/// The load order's part of the index: its entries in [`rank`]'s order
/// for unnamed resources (by ResRef; of one ResRef, the one with an
/// editor first, then by type), and their ResRefs in one text, each
/// ended by a line break, where what is typed is looked for at once:
/// a few hundred thousand resources (the game's and a world's haks) are
/// searched within a keystroke's time.
#[derive(Debug, Default)]
pub(crate) struct Game {
    pub(crate) entries: Vec<Entry>,
    text: String,
    /// Where each entry's ResRef starts in `text`.
    starts: Vec<u32>,
}

impl Game {
    pub(crate) fn new(mut entries: Vec<Entry>) -> Game {
        let order = |e: &Entry| {
            let ext = e.key.restype.extension().unwrap_or_default();
            (e.resref.clone(), Tab::for_resource(e.key).is_none(), ext)
        };
        entries.sort_by_cached_key(order);
        let mut text = String::with_capacity(entries.iter().map(|e| e.resref.len() + 1).sum());
        let mut starts = Vec::with_capacity(entries.len());
        for e in &entries {
            starts.push(text.len() as u32);
            text.push_str(&e.resref);
            text.push('\n');
        }
        Game { entries, text, starts }
    }

    /// How many entries hold every word of `needle` (in their ResRef: they
    /// have no names), and the best `most` of them in [`rank`]'s order.
    pub(crate) fn find(&self, query: &str, most: usize) -> (usize, Vec<u32>) {
        let words: Vec<&str> = query.split(' ').filter(|w| !w.is_empty()).collect();
        // (Looked for by its longest word; the others in each one found.)
        let Some(longest) = words.iter().copied().max_by_key(|w| w.len()) else {
            return (0, Vec::new());
        };
        let whole = words.len() == 1;
        let mut keys: Vec<u64> = Vec::new();
        let resref = |i: usize| &self.entries[i].resref;
        let key =
            |i: usize, tier: Tier| (tier as u64) << 56 | (resref(i).len() as u64) << 40 | i as u64;
        let text = self.text.as_bytes();
        let finder = memchr::memmem::Finder::new(longest.as_bytes());
        // The entry the last hit was in, and how it matched.
        let mut current: Option<(usize, Tier)> = None;
        let mut at = 0;
        for p in finder.find_iter(text) {
            while at + 1 < self.starts.len() && self.starts[at + 1] as usize <= p {
                at += 1;
            }
            if let Some((i, tier)) = &mut current {
                if *i == at {
                    // (A later hit in the same ResRef: a word's start.)
                    if *tier == Tier::Substring && !text[p - 1].is_ascii_alphanumeric() {
                        *tier = Tier::WordStart;
                    }
                    continue;
                }
                keys.push(key(*i, *tier));
                current = None;
            }
            let r = resref(at);
            let offset = p - self.starts[at] as usize;
            if offset + longest.len() > r.len() {
                continue;
            }
            let tier = if !whole {
                if !words.iter().all(|w| r.contains(w)) {
                    continue;
                }
                Tier::Scattered
            } else if offset == 0 && r.len() == longest.len() {
                Tier::Exact
            } else if offset == 0 {
                Tier::Prefix
            } else if !r.as_bytes()[offset - 1].is_ascii_alphanumeric() {
                Tier::WordStart
            } else {
                Tier::Substring
            };
            current = Some((at, tier));
        }
        if let Some((i, tier)) = current {
            keys.push(key(i, tier));
        }
        let count = keys.len();
        if most == 0 {
            return (count, Vec::new());
        }
        if keys.len() > most {
            keys.select_nth_unstable(most - 1);
            keys.truncate(most);
        }
        keys.sort_unstable();
        (count, keys.into_iter().map(|k| (k & 0xff_ffff_ffff) as u32).collect())
    }
}

/// A layer as the popover names where a resource comes from.
fn shown_layer(layer: &mg_resman::Layer) -> String {
    match layer.class {
        mg_resman::LayerClass::Key => "Game".to_string(),
        _ => layer.label.clone(),
    }
}

impl Index {
    /// The module's part made again where its workspace changed
    /// ([`ModuleStamp`]), and named a few more of it (all of it where the
    /// window waits for its work, as tests have it).
    fn module_part(app: &mut Moonglow) {
        let Moonglow { ws, quick_open, area_names, game, .. } = app;
        let index = &mut quick_open.index;
        let Some(ws) = ws.as_ref() else {
            if index.module_for.is_some() {
                index.module_for = None;
                index.own.clear();
                index.named = 0;
            }
            return;
        };
        let path = ws.module.location.as_ref().map(|l| l.path().to_path_buf());
        let stamp = (ws.revision(), ws.module.len(), path);
        if index.module_for.as_ref() != Some(&stamp) {
            let began = Instant::now();
            let mut keys: Vec<ResKey> = ws.module.keys().copied().collect();
            keys.sort();
            // (The names already known kept, while they are looked at
            // again: a module changed by one edit keeps showing them.)
            let known: std::collections::HashMap<ResKey, Option<String>> =
                index.own.drain(..).map(|e| (e.key, e.name)).collect();
            index.own = (keys.into_iter())
                .map(|k| Entry::new(k, known.get(&k).cloned().flatten(), None))
                .collect();
            index.named = 0;
            index.module_for = Some(stamp);
            index.module_builds += 1;
            index.module_took = began.elapsed();
        }
        let began = Instant::now();
        let game = game.as_deref();
        while index.named < index.own.len() {
            let e = &mut index.own[index.named];
            let named = if e.key.restype == ResType::ARE {
                Some(area_names.name(ws, game, e.key.resref))
            } else if mg_area::ObjectKind::from_restype(e.key.restype).is_some() {
                Some(area_names.blueprint(ws, game, e.key))
            } else {
                None
            };
            if let Some(name) = named {
                e.set_name(Some(name));
            }
            index.named += 1;
            if quick_open.background && began.elapsed() > BUDGET {
                break;
            }
        }
    }

    /// The load order's part made again where its generation changed: on
    /// a thread where the window runs its work in the background, else at
    /// once.
    fn game_part(app: &mut Moonglow, ctx: &egui::Context) {
        let generation = app.load_order;
        let index = &mut app.quick_open.index;
        if let Some((for_generation, rx)) = &index.building {
            match rx.try_recv() {
                Ok((entries, took)) if *for_generation == generation => {
                    index.game = Arc::new(entries);
                    index.game_took = took;
                    index.game_for = Some(generation);
                    index.game_builds += 1;
                    index.building = None;
                }
                Ok(_) | Err(mpsc::TryRecvError::Disconnected) => index.building = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if index.game_for == Some(generation)
            || index.building.as_ref().is_some_and(|(g, _)| *g == generation)
        {
            return;
        }
        let Some(game) = app.game.clone() else {
            index.game = Arc::new(Game::default());
            index.game_for = Some(generation);
            index.game_layers = None;
            index.game_builds += 1;
            return;
        };
        // (A new generation whose layers but the module's are as they were:
        // a save, an area made, a file of the module's read again. The
        // module's layer is no part of this index.)
        let layers: Vec<(String, u64)> = (game.resman.layers().iter())
            .filter(|l| l.label != mg_resman::MODULE_LABEL)
            .map(|l| (l.label.clone(), l.fingerprint))
            .collect();
        if index.game_for.is_some() && index.game_layers.as_ref() == Some(&layers) {
            index.game_for = Some(generation);
            index.building = None;
            return;
        }
        index.game_layers = Some(layers);
        if app.background_jobs {
            let (tx, rx) = mpsc::channel();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let began = Instant::now();
                let entries = game_entries(&game.resman);
                // (The game's data let go of at once: what changes it waits
                // for every holder, `Moonglow::release_game_data`.)
                drop(game);
                let _ = tx.send((entries, began.elapsed()));
                ctx.request_repaint();
            });
            index.building = Some((generation, rx));
        } else {
            let began = Instant::now();
            index.game = Arc::new(game_entries(&game.resman));
            index.game_took = began.elapsed();
            index.game_for = Some(generation);
            index.game_builds += 1;
        }
    }

    /// Whether the index is still being made (names or the load order).
    fn indexing(&self) -> bool {
        self.building.is_some() || self.named < self.own.len()
    }
}

/// What an earlier query found, to narrow from as more is typed (typing
/// on only ever narrows: every word of the shorter query is part of a
/// word of the longer).
#[derive(Debug, Default)]
struct Narrowed {
    text: String,
    module_for: Option<ModuleStamp>,
    named: usize,
    game_for: Option<u64>,
    own: Vec<u32>,
    /// The best of `own`, in order.
    own_best: Vec<u32>,
    /// How many of the load order's hold every word, and the best.
    game_found: usize,
    game_best: Vec<u32>,
}

/// The keys pressed in the field this frame for the popover.
#[derive(Clone, Copy, Debug, Default)]
struct Keys {
    /// Rows to move by (Down and Up: one, wrapping; the Page keys: a page).
    step: isize,
    wrap: bool,
    enter: bool,
    escape: bool,
}

/// Quick Open's state.
#[derive(Debug, Default)]
pub struct QuickOpen {
    /// What is typed.
    pub text: String,
    /// Open: the field has the keys and the popover is under it.
    pub open: bool,
    /// The row Enter runs, among those shown.
    pub selected: usize,
    /// The field is to take the keys (opened by a key or a menu).
    focus_asked: bool,
    /// The widget that had the keys before (Escape gives them back).
    pub(crate) return_to: Option<egui::Id>,
    pub(crate) index: Index,
    narrowed: Narrowed,
    /// The keys of this frame, read before the field took them.
    keys: Keys,
    /// The field's rectangle, the icon's and the popover's, as last drawn
    /// (a press outside them closes it).
    field: Option<Rect>,
    icon: Option<Rect>,
    popover: Option<Rect>,
    /// Rows the popover shows at once (the Page keys' step).
    page: usize,
    /// The selection moved by the keys: the list goes to it.
    reveal: bool,
    /// The Top Bar's middle was too narrow for the field: it is drawn
    /// over the bar while open.
    as_icon: bool,
    /// The window runs its work in the background (copied each frame).
    background: bool,
    /// The rows last shown (their names) and how many more were found.
    shown: (Vec<String>, usize),
}

impl QuickOpen {
    /// Opens it with `text` typed (empty, or [`COMMANDS`]), the field to
    /// take the keys.
    pub(crate) fn open_with(&mut self, text: &str) {
        if !self.open {
            self.text.clear();
        }
        if text == COMMANDS {
            if !parse(&self.text).0 {
                self.text = COMMANDS.to_string();
            }
        } else if !self.open {
            self.text = text.to_string();
        }
        self.open = true;
        self.focus_asked = true;
        self.selected = 0;
    }

    /// Closes it: what was typed goes, and with `back` the keys go back
    /// to the widget that had them (Escape).
    pub(crate) fn close(&mut self, ctx: &egui::Context, back: bool) {
        if !self.open {
            return;
        }
        self.open = false;
        self.text.clear();
        self.selected = 0;
        self.focus_asked = false;
        self.popover = None;
        let id = field_id();
        ctx.memory_mut(|m| {
            if m.has_focus(id) {
                m.surrender_focus(id);
            }
        });
        if let Some(to) = self.return_to.take().filter(|_| back) {
            ctx.memory_mut(|m| m.request_focus(to));
        }
    }

    /// Whether it is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// How many times the module's and the load order's parts of its
    /// index were made (for the tests).
    #[doc(hidden)]
    pub fn builds(&self) -> (u32, u32) {
        (self.index.module_builds, self.index.game_builds)
    }

    /// The popover's rectangle, as last drawn (for the tests).
    #[doc(hidden)]
    pub fn popover_rect(&self) -> Option<Rect> {
        self.popover.filter(|_| self.open)
    }

    /// The rows last shown, by name, all of them (the popover lays out
    /// only those in sight), and how many more were found (for the
    /// tests).
    #[doc(hidden)]
    pub fn shown(&self) -> (Vec<String>, usize) {
        self.shown.clone()
    }

    /// How long the last builds of the index took: the module's part (its
    /// names apart) and the load order's (for the timing tests).
    #[doc(hidden)]
    pub fn build_times(&self) -> (Duration, Duration) {
        (self.index.module_took, self.index.game_took)
    }
}

/// The field (or, given under [`mg_uikit::shell::QUICK_OPEN_ICON_UNDER`],
/// its icon button) in the Top Bar's middle, `width` wide; `key` the key
/// that opens it (its key cap), `enabled` unless a job has the window.
/// Drawn from the state taken out of the window (the Top Bar's other
/// parts hold it meanwhile); [`after_bar`] does the rest.
pub(crate) fn middle(ui: &mut Ui, width: f32, qo: &mut QuickOpen, key: &str, enabled: bool) {
    qo.field = None;
    qo.icon = None;
    let icon_side = mg_uikit::metrics::ICON_BUTTON;
    qo.as_icon = mg_uikit::shell::Yields::quick_open_as_icon(width);
    if !qo.as_icon {
        field(ui, width, qo, key, enabled);
    } else if width >= icon_side {
        let tip = match key {
            "" => "Go to a resource or a command".to_string(),
            k => format!("Go to a resource or a command ({k})"),
        };
        let r = ui.add_enabled(enabled, crate::widgets::icon_only(Icon::Search, NAME));
        let r = r.on_hover_text(&tip).on_disabled_hover_text(&tip);
        qo.icon = Some(r.rect);
        if r.clicked() {
            qo.return_to = ui.memory(|m| m.focused());
            qo.open_with("");
        }
    }
}

/// The field itself: the glass, what is typed (or the hint), the key cap
/// at its right end; the popover's keys read first while it is open.
fn field(ui: &mut Ui, width: f32, qo: &mut QuickOpen, key: &str, enabled: bool) {
    let id = field_id();
    let before = ui.memory(|m| m.focused());
    if qo.open && before == Some(id) {
        qo.keys = read_keys(ui);
    }
    if qo.focus_asked && enabled {
        qo.focus_asked = false;
        ui.memory_mut(|m| m.request_focus(id));
    }
    let t = mg_uikit::tokens(ui.ctx());
    let cap = if key.is_empty() { 0.0 } else { mg_uikit::marks::key_cap_width(ui, key) };
    let margin = mg_uikit::controls::FIELD_MARGIN;
    let glass = mg_uikit::metrics::ICON as i8 + 4;
    let cap_room = if cap > 0.0 { cap.ceil() as i8 + 4 } else { 0 };
    let margin =
        egui::Margin { left: margin.left + glass, right: margin.right + cap_room, ..margin };
    let filter = egui::EventFilter {
        tab: qo.open,
        horizontal_arrows: true,
        vertical_arrows: qo.open,
        escape: qo.open,
    };
    let edit = mg_uikit::text_field(&mut qo.text)
        .id(id)
        .margin(margin)
        .hint_text(HINT)
        .event_filter(filter)
        .desired_width(width)
        .min_size(egui::vec2(width, mg_uikit::metrics::CONTROL_HEIGHT));
    // (Centered in the bar's height, as the board has it: a row's
    // centered layout left it low.)
    let height = mg_uikit::metrics::CONTROL_HEIGHT;
    let at = ui.max_rect();
    let rect = Rect::from_min_size(
        egui::pos2(at.left(), (at.center().y - height / 2.0).round()),
        egui::vec2(width, height),
    );
    let r = ui.add_enabled_ui(enabled, |ui| ui.put(rect, edit)).inner;
    // (Named Quick Open; its key cap, drawn, said as its description.)
    r.ctx.accesskit_node_builder(r.id, |node| {
        node.set_label(NAME);
        if !key.is_empty() {
            node.set_description(key);
        }
    });
    qo.field = Some(r.rect);
    if ui.is_rect_visible(r.rect) {
        let side = egui::Vec2::splat(mg_uikit::metrics::ICON);
        let at = egui::pos2(r.rect.left() + f32::from(margin.left - glass), r.rect.center().y);
        let rect = Rect::from_min_size(at - egui::vec2(0.0, side.y / 2.0), side);
        Icon::Search.paint(ui.painter(), rect, ui.visuals().weak_text_color());
        if cap > 0.0 {
            let h = mg_uikit::marks::MARK_HEIGHT;
            let right = r.rect.right() - 3.0;
            let rect = Rect::from_min_max(
                egui::pos2(right - cap, r.rect.center().y - h / 2.0),
                egui::pos2(right, r.rect.center().y + h / 2.0),
            );
            mg_uikit::marks::paint_key_cap(ui.painter(), &t, rect, key);
        }
    }
    // A click, or typing, opens it (Tab only passes through: the keys
    // would stay in it once open).
    if !qo.open && (r.clicked() || r.changed()) && r.has_focus() && enabled {
        qo.return_to = before.filter(|b| *b != id);
        // (What was typed to open it, kept.)
        let typed = std::mem::take(&mut qo.text);
        qo.open_with("");
        qo.text = typed;
        qo.focus_asked = false;
    }
    if r.changed() {
        qo.selected = 0;
    }
}

/// The popover's keys, taken from the frame before the field sees them:
/// Down and Up (wrapping), Page Down and Page Up, Enter, Escape; Tab and
/// Shift+Tab are swallowed (the keys stay in the field while it is open).
fn read_keys(ui: &mut Ui) -> Keys {
    use egui::{Key, Modifiers};
    ui.input_mut(|i| {
        let mut keys = Keys::default();
        let none = Modifiers::NONE;
        if i.consume_key(none, Key::ArrowDown) {
            keys.step += 1;
            keys.wrap = true;
        }
        if i.consume_key(none, Key::ArrowUp) {
            keys.step -= 1;
            keys.wrap = true;
        }
        if i.consume_key(none, Key::PageDown) {
            keys.step += isize::MAX / 4;
        }
        if i.consume_key(none, Key::PageUp) {
            keys.step -= isize::MAX / 4;
        }
        keys.enter = i.consume_key(none, Key::Enter);
        keys.escape = i.consume_key(none, Key::Escape);
        i.consume_key(Modifiers::SHIFT, Key::Tab);
        i.consume_key(none, Key::Tab);
        keys
    })
}

/// After the Top Bar: the field over the bar where its middle has only
/// the icon (or no room), the popover under the field, and what was
/// chosen, done. Nothing at rest.
pub(crate) fn after_bar(app: &mut Moonglow, ui: &mut Ui, bar: Rect) {
    let ctx = ui.ctx().clone();
    // (A job has the window: Quick Open closes, and its field is grayed.)
    if app.busy() {
        app.quick_open.close(&ctx, false);
        return;
    }
    if !app.quick_open.open {
        return;
    }
    app.quick_open.background = app.background_jobs;
    // The field over the Top Bar's middle, in a window too narrow for it.
    if app.quick_open.as_icon {
        let key = app.keymap.label(crate::keys::Cmd::QuickOpen, &ctx);
        let window = ctx.content_rect();
        let cap = mg_uikit::shell::Yields::of(window.width(), window.width()).quick_open_cap;
        let width = cap.min(window.width() - 2.0 * EDGE).max(0.0);
        let rect = Rect::from_center_size(
            egui::pos2(window.center().x, bar.center().y),
            egui::vec2(width, bar.height()),
        );
        let t = mg_uikit::tokens(&ctx);
        let qo = &mut app.quick_open;
        egui::Area::new(egui::Id::new("quick-open-over-bar"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(&ctx, |ui| {
                ui.set_max_size(rect.size());
                egui::Frame::new().fill(t.bar).show(ui, |ui| {
                    ui.set_min_size(rect.size());
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        field(ui, width, qo, &key, true);
                    });
                });
            });
    }
    // Index::module_part and game_part (lazily: only while open).
    Index::module_part(app);
    Index::game_part(app, &ctx);
    let found = results(app, &ctx);
    // (Names and the load order taken a few at a time: frames go on until
    // it is all in hand, whatever is typed.)
    if app.quick_open.index.indexing() {
        ctx.request_repaint();
    }
    let qo = &mut app.quick_open;
    qo.shown = (found.hits.iter().map(|h| h.name.clone()).collect(), found.more);
    let n = found.hits.len();
    // The keys read before the field took them.
    let keys = std::mem::take(&mut qo.keys);
    if n > 0 {
        qo.selected = qo.selected.min(n - 1);
        if keys.step != 0 {
            let at = qo.selected as isize;
            let page = qo.page.max(1) as isize;
            qo.selected = if keys.wrap {
                (at + keys.step).rem_euclid(n as isize) as usize
            } else {
                let by = keys.step.signum() * page;
                (at + by).clamp(0, n as isize - 1) as usize
            };
            qo.reveal = true;
        }
    } else {
        qo.selected = 0;
    }
    if keys.escape {
        // (Not a dialog's Escape too: a Properties window under it stays.)
        crate::widgets::keep_escape(&ctx);
        qo.close(&ctx, true);
        return;
    }
    // A press outside the field and the popover closes it, running
    // nothing.
    let pressed_at =
        ctx.input(|i| i.pointer.any_pressed().then(|| i.pointer.press_origin()).flatten());
    if let Some(at) = pressed_at {
        let inside = [qo.field, qo.icon, qo.popover].into_iter().flatten().any(|r| r.contains(at));
        if !inside {
            qo.close(&ctx, false);
            return;
        }
    }
    // (The keys stay in the field while it is open: a press on a row
    // took them.)
    let id = field_id();
    if ctx.memory(|m| m.focused().is_none()) {
        ctx.memory_mut(|m| m.request_focus(id));
    }
    let anchor = qo.field.or(qo.icon).unwrap_or(bar);
    let chosen = popover(ui, qo, &found, anchor);
    let enter = keys.enter.then(|| found.hits.get(qo.selected)).flatten();
    let chosen = chosen.or_else(|| enter.filter(|h| h.enabled).cloned());
    if let Some(hit) = chosen {
        qo.close(&ctx, false);
        choose(app, &ctx, hit.target);
    }
}

/// The popover under `anchor`: its rows (each the kind's icon, the name,
/// a dim detail and a command's key cap), the line of how many more, and
/// "Indexing…" while the index is being made. The row clicked, if one
/// that can run was.
fn popover(ui: &Ui, qo: &mut QuickOpen, found: &Found, anchor: Rect) -> Option<Hit> {
    let ctx = ui.ctx().clone();
    let window = ctx.content_rect();
    let t = mg_uikit::tokens(&ctx);
    // As wide as the field, or wider where the rows want it, never past
    // the window's edges.
    let want = found.hits.iter().map(|h| row_width(ui, h)).fold(0.0, f32::max).min(WIDEST);
    let room = (window.width() - 2.0 * EDGE).max(0.0);
    let width = anchor.width().max(want).min(room);
    let left = anchor.left().min(window.right() - EDGE - width).max(window.left() + EDGE);
    let top = anchor.bottom() + BELOW;
    let extra = usize::from(found.more > 0)
        + usize::from(found.indexing)
        + usize::from(found.hits.is_empty() && !found.indexing);
    let margin = f32::from(mg_uikit::menu::MENU_MARGIN.top + mg_uikit::menu::MENU_MARGIN.bottom);
    // (The notes under the rows, always in sight; the rows scroll.)
    let tallest = (window.bottom() - EDGE - top - margin - extra as f32 * ROW).max(ROW);
    let height = ((found.hits.len() as f32) * ROW).min(tallest);
    qo.page = ((height / ROW).floor() as usize).max(1);
    let mut chosen = None;
    let selected = qo.selected;
    let reveal = std::mem::take(&mut qo.reveal);
    let shown = egui::Area::new(egui::Id::new("quick-open-popover"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(left, top))
        .show(&ctx, |ui| {
            let mut style = (*ui.ctx().global_style()).clone();
            mg_uikit::menu_style(&mut style);
            let frame = egui::Frame::popup(&style)
                .inner_margin(mg_uikit::menu::MENU_MARGIN)
                .corner_radius(mg_uikit::metrics::POPOVER_RADIUS);
            frame.show(ui, |ui| {
                ui.set_style(style);
                ui.set_width(width);
                mg_uikit::name_group(ui, RESULTS);
                let mut area = egui::ScrollArea::vertical()
                    .id_salt("quick-open-rows")
                    .max_height(height)
                    .min_scrolled_height(height)
                    .auto_shrink([false, true]);
                if reveal {
                    let at = selected as f32 * ROW;
                    let offset = ui.data(|d| d.get_temp::<f32>(egui::Id::new("quick-open-offset")));
                    let offset = offset.unwrap_or(0.0);
                    let new = if at < offset {
                        at
                    } else if at + ROW > offset + height {
                        at + ROW - height
                    } else {
                        offset
                    };
                    area = area.vertical_scroll_offset(new.max(0.0));
                }
                let out = area.show_rows(ui, ROW, found.hits.len(), |ui, range| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for at in range {
                        let hit = &found.hits[at];
                        if hit_row(ui, &t, hit, at == selected, width) && hit.enabled {
                            chosen = Some(hit.clone());
                        }
                    }
                });
                ui.data_mut(|d| {
                    d.insert_temp(egui::Id::new("quick-open-offset"), out.state.offset.y)
                });
                ui.spacing_mut().item_spacing.y = 0.0;
                for n in 0..extra {
                    note_row(ui, &note_text(found, n), width);
                }
            });
        });
    qo.popover = Some(shown.response.rect);
    chosen
}

/// The words of the popover's `n`th line after its rows.
fn note_text(found: &Found, n: usize) -> String {
    let mut lines = Vec::new();
    if found.hits.is_empty() && !found.indexing {
        lines.push("Nothing found".to_string());
    }
    if found.more > 0 {
        lines.push(format!("{} more: type more to narrow them", found.more));
    }
    if found.indexing {
        lines.push("Indexing…".to_string());
    }
    lines.get(n).cloned().unwrap_or_default()
}

/// The width a row wants: its icon, name, detail and key cap.
fn row_width(ui: &Ui, hit: &Hit) -> f32 {
    let text = |s: &str, size: f32| {
        ui.fonts_mut(|f| {
            f.layout_no_wrap(s.to_string(), egui::FontId::proportional(size), egui::Color32::WHITE)
                .size()
                .x
        })
    };
    let pad = mg_uikit::metrics::CONTROL_PADDING_X;
    let mut w = pad + mg_uikit::metrics::ICON + 8.0 + text(&hit.name, mg_uikit::metrics::BODY_SIZE);
    if !hit.detail.is_empty() {
        w += 12.0 + text(&hit.detail, mg_uikit::metrics::SMALL_SIZE);
    }
    if !hit.key.is_empty() {
        w += 24.0 + mg_uikit::marks::key_cap_width(ui, &hit.key);
    }
    w + pad
}

/// A row of the popover: lit where `selected`; whether it was clicked.
fn hit_row(ui: &mut Ui, t: &mg_uikit::Tokens, hit: &Hit, selected: bool, width: f32) -> bool {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(width, ROW), egui::Sense::click());
    r.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, hit.enabled, selected, &hit.name)
    });
    let r = if hit.enabled || hit.why.is_empty() { r } else { r.on_hover_text(&hit.why) };
    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);
        let lit = selected;
        if lit {
            painter.rect_filled(rect, 0.0, t.accent_fill);
        } else if r.hovered() {
            painter.rect_filled(rect, 0.0, t.hover_row);
        }
        let (color, dim) = match (hit.enabled, lit) {
            (false, _) => (t.text_off, t.text_off),
            (true, true) => (t.on_fill, t.on_fill),
            (true, false) => (t.text, t.text_3),
        };
        let pad = mg_uikit::metrics::CONTROL_PADDING_X;
        let side = mg_uikit::metrics::ICON;
        let icon = Rect::from_min_size(
            egui::pos2(rect.left() + pad, rect.center().y - side / 2.0),
            egui::vec2(side, side),
        );
        let icon_color = if lit && hit.enabled { t.on_fill } else { dim };
        hit.icon.paint(&painter, icon, icon_color);
        let mut right = rect.right() - pad;
        if !hit.key.is_empty() {
            let w = mg_uikit::marks::key_cap_width(ui, &hit.key);
            let h = mg_uikit::marks::MARK_HEIGHT;
            let cap = Rect::from_min_max(
                egui::pos2(right - w, rect.center().y - h / 2.0),
                egui::pos2(right, rect.center().y + h / 2.0),
            );
            mg_uikit::marks::paint_key_cap(&painter, t, cap, &hit.key);
            right = cap.left() - 12.0;
        }
        let x = icon.right() + 8.0;
        let body = egui::FontId::proportional(mg_uikit::metrics::BODY_SIZE);
        let small = egui::FontId::proportional(mg_uikit::metrics::SMALL_SIZE);
        let room = (right - x).max(0.0);
        let name = elided(ui, &hit.name, body, color, room);
        let name_w = name.size().x;
        painter.galley(egui::pos2(x, rect.center().y - name.size().y / 2.0), name, color);
        let left = room - name_w - 12.0;
        if !hit.detail.is_empty() && left > 24.0 {
            let detail = elided(ui, &hit.detail, small, dim, left);
            let at = egui::pos2(x + name_w + 12.0, rect.center().y - detail.size().y / 2.0);
            painter.galley(at, detail, dim);
        }
    }
    r.clicked()
}

/// `text` laid out on one line, cut with an ellipsis to `width`.
fn elided(
    ui: &Ui,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
    width: f32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_string(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(1.0));
    ui.fonts_mut(|f| f.layout_job(job))
}

/// A quiet line after the rows (how many more, "Indexing…").
fn note_row(ui: &mut Ui, text: &str, width: f32) {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(width, ROW), egui::Sense::hover());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, text));
    let color = ui.visuals().weak_text_color();
    let font = egui::FontId::proportional(mg_uikit::metrics::SMALL_SIZE);
    let pad = mg_uikit::metrics::CONTROL_PADDING_X;
    let galley = elided(ui, text, font, color, width - 2.0 * pad);
    let at = egui::pos2(rect.left() + pad, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(at, galley, color);
}

/// Why a command cannot run now (its row's tip).
fn why(app: &Moonglow, choice: &Choice) -> String {
    let no_module = app.ws.is_none();
    match choice {
        Choice::Window(id) => id.why_disabled(app).to_string(),
        Choice::Plugin(..) if no_module => "Open a module first".into(),
        Choice::Project(_) => "The project's file gives this command nothing to run".into(),
        _ => "It cannot be used now".into(),
    }
}

/// A resource's row of the module (`resource`) or the load order.
fn resource_hit(e: &Entry, game: bool) -> Hit {
    let detail = match (&e.name, &e.layer) {
        (_, Some(layer)) if game => {
            if e.name.is_some() {
                format!("{}, {layer}", e.file())
            } else {
                layer.to_string()
            }
        }
        (Some(_), _) => e.file(),
        (None, _) => String::new(),
    };
    Hit {
        target: if game { Target::Game(e.key) } else { Target::Resource(e.key) },
        name: e.shown(),
        detail,
        icon: crate::browser::kind_icon(e.key.restype),
        key: String::new(),
        enabled: true,
        why: String::new(),
    }
}

/// A command's row.
fn command_hit(app: &Moonglow, ctx: &egui::Context, row: &commands::Row) -> Hit {
    let icon = match row.choice {
        Choice::Window(_) | Choice::Project(_) => Icon::Command,
        Choice::Plugin(..) | Choice::Entry(..) => Icon::Plugin,
    };
    Hit {
        target: Target::Command(row.choice.clone()),
        name: row.label.clone(),
        detail: row.menu.clone(),
        icon,
        key: app.keymap.label_of(&row.key_id, ctx),
        enabled: row.enabled,
        why: if row.enabled { String::new() } else { why(app, &row.choice) },
    }
}

/// A recent module's row.
fn module_hit(path: &std::path::Path) -> Hit {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let folder = path.parent().map(|p| p.display().to_string()).unwrap_or_default();
    Hit {
        target: Target::Module(path.to_path_buf()),
        name,
        detail: folder,
        icon: Icon::Module,
        key: String::new(),
        enabled: true,
        why: String::new(),
    }
}

/// The window's commands as Quick Open offers them (Quick Open itself and
/// the Command Palette aside).
fn command_rows(app: &Moonglow) -> Vec<CommandRow> {
    commands::found(app)
        .into_iter()
        .filter(|r| !matches!(r.choice, Choice::Window(Id::QuickOpen | Id::CommandPalette)))
        .map(CommandRow::new)
        .collect()
}

/// What the popover shows now.
fn results(app: &mut Moonglow, ctx: &egui::Context) -> Found {
    let text = app.quick_open.text.clone();
    let (commands_only, typed) = parse(&text);
    let needle = Needle::new(typed);
    let query = query_of(&needle);
    let commands = command_rows(app);
    let indexing = app.quick_open.index.indexing();
    let mut hits: Vec<Hit> = Vec::new();
    let mut total = 0;
    if needle.is_empty() {
        // (This module's areas opened lately, most recent first.)
        let recent =
            app.module_path().map(|m| app.settings.recent_areas(&m).to_vec()).unwrap_or_default();
        if commands_only {
            total = commands.len();
            hits.extend(commands.iter().take(MOST).map(|c| command_hit(app, ctx, &c.row)));
        } else if let Moonglow { quick_open, area_names, game, ws: Some(ws), .. } = &mut *app {
            for area in recent {
                let Some(key) = ResKey::parse(&area, ResType::ARE) else { continue };
                if !ws.module.contains(&key) {
                    continue;
                }
                // (Named at once, where the index has not come to it yet.)
                let known = quick_open.index.own.iter().find(|e| e.key == key);
                let name = match known.and_then(|e| e.name.clone()) {
                    Some(name) => Some(name),
                    None => Some(area_names.name(ws, game.as_deref(), key.resref)),
                };
                hits.push(resource_hit(&Entry::new(key, name, None), false));
            }
            total = hits.len();
        } else {
            let recent = app.settings.recent.clone();
            hits.extend(recent.iter().map(|p| module_hit(p)));
            let free = commands.iter().filter(|c| c.row.enabled);
            hits.extend(free.map(|c| command_hit(app, ctx, &c.row)));
            total = hits.len();
            hits.truncate(MOST);
        }
        return Found { more: total - hits.len(), indexing: false, hits };
    }
    // Typed: the module's (or the recent modules), the commands, the load
    // order's, each ranked.
    narrow(app, typed, &needle, commands_only);
    let qo = &app.quick_open;
    let index = &qo.index;
    if !commands_only {
        if app.ws.is_some() {
            total += qo.narrowed.own.len();
            let best = qo.narrowed.own_best.iter();
            hits.extend(best.map(|&i| resource_hit(&index.own[i as usize], false)));
        } else {
            let mut found: Vec<(Tier, &PathBuf)> = (app.settings.recent.iter())
                .filter_map(|p| {
                    let name = fold(&p.file_name().unwrap_or_default().to_string_lossy());
                    tier_of(&name, &query)
                        .or_else(|| needle.finds_in(&[&name]).then_some(Tier::Scattered))
                        .map(|t| (t, p))
                })
                .collect();
            found.sort_by_key(|(t, _)| *t);
            total += found.len();
            hits.extend(found.into_iter().map(|(_, p)| module_hit(p)));
        }
    }
    let found = rank_commands(&commands, &needle, &query);
    total += found.len();
    let room = MOST.saturating_sub(hits.len());
    hits.extend(found.iter().take(room).map(|&i| command_hit(app, ctx, &commands[i].row)));
    if !commands_only {
        total += qo.narrowed.game_found;
        let room = MOST.saturating_sub(hits.len());
        let best = qo.narrowed.game_best.iter().take(room);
        hits.extend(best.map(|&i| resource_hit(&index.game.entries[i as usize], true)));
    }
    hits.truncate(MOST);
    Found { more: total.saturating_sub(hits.len()), indexing, hits }
}

/// The module's and the load order's entries that hold every word
/// (`narrowed`), found again only where what is typed or the index
/// changed, and from those found before where what is typed only grew.
fn narrow(app: &mut Moonglow, typed: &str, needle: &Needle, commands_only: bool) {
    let qo = &mut app.quick_open;
    let index = &qo.index;
    let n = &mut qo.narrowed;
    if commands_only {
        n.text.clear();
        n.own.clear();
        n.own_best.clear();
        n.game_best.clear();
        n.game_found = 0;
        n.module_for = None;
        n.game_for = None;
        return;
    }
    let same_index = n.module_for == index.module_for && n.named == index.named;
    let same_game = n.game_for == index.game_for;
    if n.text == typed && same_index && same_game {
        return;
    }
    // (Typing on narrows: the words typed before are each part of one
    // typed now. A name taken since may match where it did not.)
    let grew = !n.text.is_empty() && typed.starts_with(n.text.as_str());
    let module_within = (grew && same_index).then_some(n.own.as_slice());
    let own = matches(&index.own, module_within, needle);
    // (The load order's searched anew: [`Game::find`].)
    if !(same_game && n.text == typed) {
        let query = query_of(needle);
        (n.game_found, n.game_best) = index.game.find(&query, MOST);
    }
    n.own_best = rank(&index.own, &own, needle, &query_of(needle), MOST);
    n.own = own;
    n.text = typed.to_string();
    n.module_for = index.module_for.clone();
    n.named = index.named;
    n.game_for = index.game_for;
}

/// Does what a row stands for, the popover closed.
fn choose(app: &mut Moonglow, ctx: &egui::Context, target: Target) {
    match target {
        // (As the Module panel opens it: an area's tab, a blueprint's
        // Properties, a script's editor; one open is brought to the front.)
        Target::Resource(key) => app.actions.push(Action::OpenResource(key)),
        // (As the Resources panel opens it, read-only.)
        Target::Game(key) => {
            let tab = crate::browser::tab_for(app, key, None);
            app.actions.push(Action::OpenTab(tab));
        }
        Target::Command(choice) => commands::run_choice(app, ctx, choice),
        // (Asked about unsaved work first, as File › Recent Modules is.)
        Target::Module(path) => app.actions.push(Action::OpenModule(path)),
    }
}

/// What a keystroke costs on the load order `resman` (for the timing
/// test): the index made (and how long it took, the best of `tries`),
/// then each of `queries` looked for in the whole of it and the best 50
/// ranked: how many it found and how long that took (the best of
/// `tries`).
#[doc(hidden)]
pub fn time_keystrokes(
    resman: &mg_resman::ResMan,
    queries: &[&str],
    tries: usize,
) -> (usize, Duration, Vec<(String, usize, Duration)>) {
    let best = |f: &mut dyn FnMut()| {
        (0..tries.max(1))
            .map(|_| {
                let began = Instant::now();
                f();
                began.elapsed()
            })
            .min()
            .unwrap_or_default()
    };
    let mut game = Game::default();
    let built = best(&mut || game = game_entries(resman));
    let mut out = Vec::new();
    for typed in queries {
        let query = query_of(&Needle::new(typed));
        let mut found = 0;
        let took = best(&mut || found = game.find(&query, MOST).0);
        out.push((typed.to_string(), found, took));
    }
    (game.entries.len(), built, out)
}

/// What a keystroke costs on a module of `count` resources (for the
/// timing test): blueprints with names, as a persistent world's are
/// (`plc_tree_0001`, "Tree 1"...), each of `queries` found anew and the
/// best 50 ranked as a keystroke does it; how many it found and how long
/// it took, the best of `tries`.
#[doc(hidden)]
pub fn time_module_keystrokes(
    count: usize,
    queries: &[&str],
    tries: usize,
) -> Vec<(String, usize, Duration)> {
    const KINDS: [(&str, &str, ResType); 4] = [
        ("plc_tree", "Tree", ResType::UTP),
        ("cre_guard", "Guard", ResType::UTC),
        ("itm_sword", "Iron Sword", ResType::UTI),
        ("are_road", "Road", ResType::ARE),
    ];
    let entries: Vec<Entry> = (0..count)
        .filter_map(|i| {
            let (prefix, name, t) = KINDS[i % KINDS.len()];
            let key = ResKey::parse(&format!("{prefix}_{i:05}"), t)?;
            Some(Entry::new(key, Some(format!("{name} {i}")), None))
        })
        .collect();
    queries
        .iter()
        .map(|typed| {
            let needle = Needle::new(typed);
            let query = query_of(&needle);
            let mut found = 0;
            let took = (0..tries.max(1))
                .map(|_| {
                    let began = Instant::now();
                    let all = matches(&entries, None, &needle);
                    rank(&entries, &all, &needle, &query, MOST);
                    found = all.len();
                    began.elapsed()
                })
                .min()
                .unwrap_or_default();
            (typed.to_string(), found, took)
        })
        .collect()
}

/// What a whole keystroke costs in `app` (for the timing test): the index
/// made first (all of it, names too), then each of `queries` typed anew
/// (nothing narrowed from before) and the popover's rows made, commands
/// and all, as a frame makes them; how many rows were found and how long
/// it took, the best of `tries`.
#[doc(hidden)]
pub fn time_whole_keystrokes(
    app: &mut Moonglow,
    queries: &[&str],
    tries: usize,
) -> Vec<(String, usize, Duration)> {
    let ctx = egui::Context::default();
    app.quick_open.background = false;
    app.quick_open.open_with("");
    Index::module_part(app);
    Index::game_part(app, &ctx);
    queries
        .iter()
        .map(|typed| {
            let mut found = 0;
            let took = (0..tries.max(1))
                .map(|_| {
                    app.quick_open.narrowed = Narrowed::default();
                    app.quick_open.text = typed.to_string();
                    let began = Instant::now();
                    let f = results(app, &ctx);
                    found = f.hits.len() + f.more;
                    began.elapsed()
                })
                .min()
                .unwrap_or_default();
            (typed.to_string(), found, took)
        })
        .collect()
}

/// The tab a load-order resource opens in from Quick Open (the Resources
/// panel's choice: [`crate::browser::tab_for`]).
#[doc(hidden)]
pub fn game_tab(app: &Moonglow, key: ResKey) -> Tab {
    crate::browser::tab_for(app, key, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str, t: ResType) -> ResKey {
        ResKey::parse(name, t).unwrap()
    }

    fn entry(name: &str, t: ResType, shown: Option<&str>) -> Entry {
        Entry::new(key(name, t), shown.map(str::to_string), None)
    }

    /// The names of the best of `entries` for `typed`, in order.
    fn ranked(entries: &[Entry], typed: &str) -> Vec<String> {
        let needle = Needle::new(typed);
        let found = matches(entries, None, &needle);
        let best = rank(entries, &found, &needle, &query_of(&needle), MOST);
        best.iter().map(|&i| entries[i as usize].shown()).collect()
    }

    #[test]
    fn tiers_go_exact_prefix_word_start_substring() {
        assert_eq!(tier_of("formosa", "formosa"), Some(Tier::Exact));
        assert_eq!(tier_of("formosa_9", "formosa"), Some(Tier::Prefix));
        assert_eq!(tier_of("m1q3_formosa_9", "formosa"), Some(Tier::WordStart));
        assert_eq!(tier_of("the formosa", "formosa"), Some(Tier::WordStart));
        assert_eq!(tier_of("informosa", "formosa"), Some(Tier::Substring));
        // (A later word start counts where an earlier match is inside a word.)
        assert_eq!(tier_of("xformosa formosa", "formosa"), Some(Tier::WordStart));
        assert_eq!(tier_of("formosa", "formosb"), None);
        assert_eq!(tier_of("formosa", ""), None);
        assert!(Tier::Exact < Tier::Prefix && Tier::Prefix < Tier::WordStart);
        assert!(Tier::WordStart < Tier::Substring && Tier::Substring < Tier::Scattered);
        assert!(Tier::Scattered < Tier::WithMenu);
    }

    #[test]
    fn rows_rank_by_tier_then_name_before_resref_then_length() {
        let entries = vec![
            entry("informosa", ResType::NSS, None),
            entry("m1q3_formosa_9", ResType::ARE, Some("Port of Formosa")),
            entry("formosa_inn", ResType::ARE, None),
            entry("formosa", ResType::DLG, None),
            entry("abc", ResType::ARE, Some("Formosa Bay")),
            entry("formosa_bay", ResType::UTC, Some("Sailor")),
            entry("x", ResType::UTP, Some("Formosas")),
        ];
        assert_eq!(
            ranked(&entries, "Formosa"),
            [
                // Exact (on the ResRef: no name equals it).
                "formosa.dlg",
                // Prefix: on a name, the shorter first, then on ResRefs.
                "Formosas",
                "Formosa Bay",
                "Sailor",
                "formosa_inn.are",
                // Word start ("Port of Formosa" by its name).
                "Port of Formosa",
                // Substring.
                "informosa.nss",
            ]
        );
    }

    #[test]
    fn a_name_match_comes_before_a_resref_match_of_one_tier() {
        let entries = vec![
            entry("gate", ResType::UTD, Some("Door")),
            entry("door", ResType::UTD, Some("Old")),
        ];
        // ("door" is a ResRef's exact match, and the other's name's.)
        assert_eq!(ranked(&entries, "door"), ["Door", "Old"]);
    }

    #[test]
    fn words_apart_rank_after_the_whole_and_find_across_fields() {
        let entries = vec![
            entry("sword01", ResType::UTI, Some("Iron Sword")),
            entry("iron_sword", ResType::UTI, None),
            entry("ironsword", ResType::UTI, Some("Plain")),
        ];
        // ("Iron Sword" whole; then every word apart: the shorter first.)
        assert_eq!(ranked(&entries, "iron sword"), ["Iron Sword", "Plain", "iron_sword.uti"]);
        // (Apart all three, the name's match first.)
        assert_eq!(ranked(&entries, "sword iron"), ["Iron Sword", "Plain", "iron_sword.uti"]);
        // (Every word, one in the name and one in the ResRef.)
        assert_eq!(ranked(&entries, "plain ironsw"), ["Plain"]);
    }

    #[test]
    fn case_accents_and_other_scripts_fold() {
        let entries = vec![
            entry("a1", ResType::ARE, Some("Árbol Seco")),
            entry("a2", ResType::ARE, Some("Деревня Колдуна")),
            entry("a3", ResType::ARE, Some("Ёлка")),
        ];
        assert_eq!(ranked(&entries, "arbol"), ["Árbol Seco"]);
        assert_eq!(ranked(&entries, "ARBOL SECO"), ["Árbol Seco"]);
        assert_eq!(ranked(&entries, "деревня"), ["Деревня Колдуна"]);
        assert_eq!(ranked(&entries, "КОЛДУНА"), ["Деревня Колдуна"]);
        assert_eq!(ranked(&entries, "елка"), ["Ёлка"]);
        // ResRefs are ASCII, typed in either case.
        assert_eq!(ranked(&entries, "A2"), ["Деревня Колдуна"]);
    }

    #[test]
    fn spaces_and_long_queries() {
        let entries = vec![entry("inn", ResType::ARE, Some("The Inn"))];
        let (only, typed) = parse("    ");
        assert!(!only && typed.is_empty());
        assert!(Needle::new(typed).is_empty());
        let long = "inn ".repeat(2000);
        assert_eq!(ranked(&entries, &long), ["The Inn"]);
        let longer = "x".repeat(10_000);
        assert!(ranked(&entries, &longer).is_empty());
    }

    #[test]
    fn a_greater_than_sign_looks_among_the_commands() {
        assert_eq!(parse(">build"), (true, "build"));
        assert_eq!(parse("  > build mod "), (true, "build mod"));
        assert_eq!(parse(">"), (true, ""));
        assert_eq!(parse("build"), (false, "build"));
        assert_eq!(parse("a>b"), (false, "a>b"));
    }

    #[test]
    fn no_more_than_the_most_are_ranked_and_they_are_the_best() {
        let mut entries: Vec<Entry> =
            (0..500).map(|i| entry(&format!("zz_tree{i:03}"), ResType::UTP, None)).collect();
        entries.push(entry("tree", ResType::UTP, None));
        let needle = Needle::new("tree");
        let found = matches(&entries, None, &needle);
        assert_eq!(found.len(), 501);
        let best = rank(&entries, &found, &needle, "tree", MOST);
        assert_eq!(best.len(), MOST);
        assert_eq!(entries[best[0] as usize].shown(), "tree.utp");
        // (Then word starts, alphabetical: zz_tree000, 001...)
        assert_eq!(entries[best[1] as usize].shown(), "zz_tree000.utp");
        assert_eq!(entries[best[MOST - 1] as usize].shown(), "zz_tree048.utp");
        assert!(rank(&entries, &found, &needle, "tree", 0).is_empty());
    }

    #[test]
    fn narrowing_from_an_earlier_query_finds_the_same() {
        let entries: Vec<Entry> = ["formosa", "form", "fort", "formula", "deform"]
            .iter()
            .map(|n| entry(n, ResType::ARE, None))
            .collect();
        let short = matches(&entries, None, &Needle::new("for"));
        let long = Needle::new("form");
        assert_eq!(matches(&entries, Some(&short), &long), matches(&entries, None, &long));
        let words = Needle::new("form a");
        assert_eq!(matches(&entries, Some(&short), &words), matches(&entries, None, &words));
    }

    #[test]
    fn commands_rank_by_name_then_with_their_menu() {
        let row = |label: &str, menu: &str| {
            CommandRow::new(commands::Row {
                choice: Choice::Project(label.into()),
                label: label.into(),
                menu: menu.into(),
                key_id: String::new(),
                enabled: true,
            })
        };
        let rows = vec![
            row("Verify Module", "Build"),
            row("Build Module…", "Build"),
            row("Open Module…", "File"),
            row("Build", "Plugins"),
        ];
        let needle = Needle::new("mod build");
        let found = rank_commands(&rows, &needle, &query_of(&needle));
        // (By name first, then by its menu's name too; Open Module isn't
        // in a menu named Build.)
        assert_eq!(found, [1, 0]);
        let needle = Needle::new("build");
        assert_eq!(rank_commands(&rows, &needle, "build"), [3, 1, 0]);
    }

    /// The index is made when first asked for, and again only when the
    /// workspace's revision (or its count of resources) or the load
    /// order's generation changed: the builds counted.
    #[test]
    fn the_index_is_made_again_only_when_what_it_was_made_from_changed() {
        let ctx = egui::Context::default();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        let mut module = mg_module::Module::new();
        module.set(key("inn", ResType::ARE), b"x".to_vec());
        app.ws = Some(mg_edit::Workspace::new(module));
        let refresh = |app: &mut Moonglow| {
            Index::module_part(app);
            Index::game_part(app, &ctx);
            (app.quick_open.index.module_builds, app.quick_open.index.game_builds)
        };
        assert_eq!(refresh(&mut app), (1, 1));
        assert_eq!(refresh(&mut app), (1, 1), "nothing changed: not made again");
        let nothing = mg_edit::Command::new("Nothing", Vec::new());
        app.apply(nothing).unwrap();
        assert_eq!(refresh(&mut app), (2, 1), "a new revision: the module's part");
        assert_eq!(refresh(&mut app), (2, 1));
        app.load_order += 1;
        assert_eq!(refresh(&mut app), (2, 2), "a new load order: the load order's part");
        assert_eq!(refresh(&mut app), (2, 2));
        // (As a file read again from outside: `Workspace::adopt`'s way.)
        let workspace = app.ws.as_mut().unwrap();
        workspace.module.set(key("cellar", ResType::ARE), b"x".to_vec());
        assert_eq!(refresh(&mut app), (3, 2), "a resource put in past the commands");
        let keys: Vec<String> = app.quick_open.index.own.iter().map(Entry::file).collect();
        assert_eq!(keys, ["cellar.are", "inn.are"]);
    }

    /// The load order's search (one text, looked through at once) finds
    /// and ranks as the module's does, entry by entry.
    #[test]
    fn the_load_order_s_search_ranks_as_the_rule_does() {
        let names = [
            "formosa",
            "formosa_inn",
            "m1q3_formosa_9",
            "informosa",
            "xformosa_formosa",
            "zz_tree000",
            "tree",
            "nw_c2_default1",
            "a_b",
            "ab",
            "b_a",
            "deform",
        ];
        let mut entries = Vec::new();
        for n in names {
            for t in [ResType::NSS, ResType::NCS, ResType::UTP] {
                entries.push(Entry::new(key(n, t), None, Some(Arc::from("Game"))));
            }
        }
        let game = Game::new(entries);
        for typed in ["formosa", "for", "a", "b a", "tree", "nw c2", "zzz", "o", "ab"] {
            let needle = Needle::new(typed);
            let query = query_of(&needle);
            let found = matches(&game.entries, None, &needle);
            // (All of them: a word's start later in a ResRef, as in
            // `xformosa_formosa`, ranks above a plain substring.)
            let all = game.entries.len();
            let by_rule = rank(&game.entries, &found, &needle, &query, all);
            let (count, best) = game.find(&query, all);
            assert_eq!(count, found.len(), "{typed}");
            assert_eq!(best, by_rule, "{typed}");
        }
        // (Those with an editor, by type, before a compiled script.)
        let (_, best) = game.find("tree", 3);
        let files: Vec<String> = best.iter().map(|&i| game.entries[i as usize].file()).collect();
        assert_eq!(files, ["tree.nss", "tree.utp", "tree.ncs"]);
    }

    /// A new load-order generation whose layers but the module's are as
    /// they were (a save, an area made) keeps the load order's part; a
    /// layer added or read anew makes it again.
    #[test]
    fn the_load_order_s_part_is_kept_when_only_the_module_s_layer_moved() {
        use mg_resman::{LayerClass, MemContainer, ResMan};
        let ctx = egui::Context::default();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        let layer = |name: &str| {
            let mut c = MemContainer::new();
            c.insert(key(name, ResType::NSS), b"x".to_vec());
            c
        };
        let mut rm = ResMan::new();
        rm.add(1, "key:base", LayerClass::Key, layer("nw_c2_default1"));
        rm.add(9, mg_resman::MODULE_LABEL, LayerClass::Erf, layer("own_script"));
        app.game = Some(Arc::new(mg_rules::GameData::new(
            rm,
            mg_tlk::Tlk::new(mg_core::Language::ENGLISH),
        )));
        let refresh = |app: &mut Moonglow| {
            Index::game_part(app, &ctx);
            let names: Vec<String> =
                app.quick_open.index.game.entries.iter().map(Entry::file).collect();
            (app.quick_open.index.game_builds, names)
        };
        // (The module's own layer is no part of it.)
        assert_eq!(refresh(&mut app), (1, vec!["nw_c2_default1.nss".to_string()]));
        app.load_order += 1;
        assert_eq!(refresh(&mut app).0, 1, "the module's layer alone moved: kept");
        let mut rm = ResMan::new();
        rm.add(1, "key:base", LayerClass::Key, layer("nw_c2_default1"));
        rm.add(5, "hak:extra", LayerClass::Erf, layer("hak_script"));
        app.game = Some(Arc::new(mg_rules::GameData::new(
            rm,
            mg_tlk::Tlk::new(mg_core::Language::ENGLISH),
        )));
        app.load_order += 1;
        let (builds, names) = refresh(&mut app);
        assert_eq!(builds, 2, "a hak added: made again");
        assert_eq!(names, ["hak_script.nss", "nw_c2_default1.nss"]);
    }

    /// One list in three groups: the module's resources, then the
    /// commands, then the load order's resources (opened read-only).
    #[test]
    fn the_module_comes_first_then_the_commands_then_the_load_order() {
        use mg_resman::{LayerClass, MemContainer, ResMan};
        let ctx = egui::Context::default();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        let mut base = MemContainer::new();
        base.insert(key("new_game", ResType::NSS), b"x".to_vec());
        let mut rm = ResMan::new();
        rm.add(1, "key:base", LayerClass::Key, base);
        let tlk = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
        app.game = Some(Arc::new(mg_rules::GameData::new(rm, tlk)));
        let mut module = mg_module::Module::new();
        module.set(key("new_mine", ResType::NSS), b"x".to_vec());
        app.ws = Some(mg_edit::Workspace::new(module));
        app.quick_open.open_with("");
        app.quick_open.text = "new".into();
        Index::module_part(&mut app);
        Index::game_part(&mut app, &ctx);
        let found = results(&mut app, &ctx);
        let group = |t: &Target| match t {
            Target::Resource(_) | Target::Module(_) => 0,
            Target::Command(_) => 1,
            Target::Game(_) => 2,
        };
        let groups: Vec<u8> = found.hits.iter().map(|h| group(&h.target)).collect();
        assert!(groups.windows(2).all(|w| w[0] <= w[1]), "{groups:?}");
        assert_eq!(groups.first(), Some(&0));
        assert!(groups.contains(&1), "New Module… and the rest");
        assert_eq!(groups.last(), Some(&2));
        assert_eq!(found.hits.last().unwrap().name, "new_game.nss");
        assert_eq!(found.hits.last().unwrap().detail, "Game");
        assert_eq!(found.hits[0].target, Target::Resource(key("new_mine", ResType::NSS)));
        // (`>`: the commands alone.)
        app.quick_open.text = ">new".into();
        let found = results(&mut app, &ctx);
        assert!(found.hits.iter().all(|h| matches!(h.target, Target::Command(_))));
        assert!(!found.hits.is_empty());
    }

    #[test]
    fn a_layer_of_the_game_is_named_so() {
        // (No layers: nothing; the module's own is left out.)
        let rm = mg_resman::ResMan::new();
        assert!(game_entries(&rm).entries.is_empty());
    }
}
