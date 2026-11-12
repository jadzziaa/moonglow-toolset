//! Quick Open (M3's T8; SPEC sections 6, 12 and 13: the Command Palette
//! extended to resources): the Top Bar's field "Go to resource or command"
//! and the popover of what it finds under it.
//!
//! What it finds, best first, in three groups:
//!
//! 1. the module's resources, by their file's name (`hello.nss`, as the
//!    Module panel's Filter finds them; so by ResRef too, as the Resources
//!    panel's search does) and, for an area or a blueprint, by its name
//!    (the Filter's other field; the objects placed in an area, their
//!    names and tags, which the Filter also looks through, are left out);
//! 2. the commands: those of the menus, the plugins' commands and entries
//!    and the project's commands (`commands::found`), grayed where they
//!    cannot run now;
//! 3. the load order's resources (the game's, the haks', override's: of
//!    each the copy in the highest layer but the module's own), which open
//!    read-only as the Resources panel opens them; one the module has too
//!    is listed once, as the module's.
//!
//! A last word naming a type narrows the resources to it: `.utc` (alone
//! or after other words), or an extension after another word (`hello
//! nss`) ([`Query`]).
//!
//! Within a group (the rule [`rank`] keeps, held by the unit tests): first
//! those that open in an editor or a viewer of their own, then those
//! shown only as their bytes (a compiled script, a debug file, a texture,
//! a sound: [`has_own_view`]), each half in the order that follows. The
//! whole of what is typed (its words joined by one space, case and
//! accents aside) equal to a field, then a field that starts with it, then
//! one where it starts a word (after a character that is no letter or
//! digit: `formosa` in `m1q3_formosa_9`, `nss` in `hello.nss`), then one
//! that holds it anywhere; then rows that hold every word apart (`sword
//! iron` in "Iron Sword"), and last a command found only with its menu's
//! name (`mod build` finds Verify Module in the Build menu). At one tier,
//! a match on the name before one on the ResRef before one on the file's
//! name (the ResRef with its extension); then the shorter name, then the
//! name, the ResRef and the extension in alphabetical order.
//!
//! What is typed after a `>` looks among the commands alone (Ctrl+Shift+P
//! and Help › Command Palette… open the field with it). SPEC names no other
//! prefix. With nothing typed: the areas of this module opened lately, most
//! recent first (`Settings::recent_areas`); the areas of other recent
//! modules cannot be opened without opening their module, so they are not
//! offered. With no module open: the recent modules, then the commands
//! that work without one. While the field has the keys, the window's
//! commands that change the module or a tab wait ([`runs_from_field`]).
//!
//! The index is made lazily, the first time the field opens: the module's
//! part again only when the workspace's revision changes
//! ([`ModuleStamp`]); the load order's part only when the layers it is
//! made of (all but the module's) changed, looked at when its generation
//! (`Moonglow::load_order`) moves. Names (a blueprint's means reading it)
//! are taken a few at a time, within [`BUDGET`] a frame, and the load
//! order's part is made on a thread of its own where the window runs its
//! work in the background (`Moonglow::background_jobs`), which gives up
//! at once when the game's data is to be let go of: meanwhile the field
//! works on what is in hand and its last row says "Indexing…". At rest
//! (closed) it costs nothing but the field's drawing. The load order's
//! part is kept while a module stays open (Quick Open opens again at
//! once), let go of when its layers change or the game's data is read
//! again, and not kept without game data: for 380,000 resources its
//! entries hold a key, where its file's name is in one shared text, its
//! layer and whether it has a view of its own (at most 40 bytes each,
//! 22 MB with the shared text for the game and 150 haks).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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
    /// The file's name: the ResRef and its extension.
    File,
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

/// What is typed, read for the resources: its words (folded) and their
/// text as [`tier_of`] takes it, and the type a last word names, which
/// narrows the resources to that type: `.utc` (alone or after other
/// words), or an extension after another word (`hello nss`). A word that
/// holds a dot but names no type is looked for in the file's name as it
/// is (`hello.nss`).
#[derive(Clone, Debug, Default)]
pub(crate) struct Query {
    pub(crate) needle: Needle,
    pub(crate) text: String,
    pub(crate) restype: Option<ResType>,
}

impl Query {
    pub(crate) fn new(typed: &str) -> Query {
        let words: Vec<&str> = typed.split_whitespace().collect();
        let typed_as = |w: &str| ResType::from_extension(&w.to_ascii_lowercase());
        let (restype, rest) = match words.split_last() {
            Some((last, rest)) => match last.strip_prefix('.') {
                Some(ext) => match typed_as(ext) {
                    Some(t) => (Some(t), rest),
                    None => (None, &words[..]),
                },
                None if !rest.is_empty() => match typed_as(last) {
                    Some(t) => (Some(t), rest),
                    None => (None, &words[..]),
                },
                None => (None, &words[..]),
            },
            None => (None, &words[..]),
        };
        let needle = Needle::new(&rest.join(" "));
        Query { text: query_of(&needle), needle, restype }
    }
}

/// Whether a resource opens in an editor or a viewer of its own: a tab of
/// its kind (`Tab::for_resource`: a script, an area, a blueprint, a model,
/// any other GFF's tree...), or, read-only, a 2DA's table or a text
/// (`browser::resource_ui`); not one shown only as its bytes (a compiled
/// script, a script's debug file, a texture, a sound, a walkmesh). Quick
/// Open lists those first.
pub(crate) fn has_own_view(key: ResKey) -> bool {
    Tab::for_resource(key).is_some()
        || key.restype == ResType::TWODA
        || crate::browser::TEXT_TYPES.contains(&key.restype)
}

/// An area's or a blueprint's name, shown, and folded to be found by.
#[derive(Clone, Debug)]
struct Named {
    shown: String,
    fold: String,
}

/// A resource in the index: its key, its file name's place in its part's
/// text, its name where it has one (the module's areas and blueprints),
/// and the layer the game reads it from (the load order's).
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) key: ResKey,
    named: Option<Box<Named>>,
    /// Where its file name (`resref.ext`) starts in its part's text, and
    /// how long its ResRef and its file name are.
    start: u32,
    resref_len: u8,
    file_len: u8,
    /// Which of its part's labels says where it comes from.
    layer: u16,
    /// It opens in an editor or a viewer of its own ([`has_own_view`]).
    viewed: bool,
}

/// A part of the index (the module's resources, or the load order's): its
/// entries, in [`rank`]'s order for unnamed resources (by ResRef; of one
/// ResRef, the one with a view of its own first, then by type), their
/// file names in one text, each ended by a line break, where what is
/// typed is looked for at once (a few hundred thousand resources are
/// searched within a keystroke's time), and the layers' labels.
#[derive(Debug, Default)]
pub(crate) struct Part {
    pub(crate) entries: Vec<Entry>,
    text: String,
    labels: Vec<Arc<str>>,
}

/// A layer as the popover names where a resource comes from.
fn shown_layer(layer: &mg_resman::Layer) -> String {
    match layer.class {
        mg_resman::LayerClass::Key => "Game".to_string(),
        _ => layer.label.clone(),
    }
}

impl Part {
    /// The part of `keys` (each with the label it comes from, an index
    /// into `labels`).
    pub(crate) fn new(mut keys: Vec<(ResKey, u16)>, labels: Vec<Arc<str>>) -> Part {
        let resref = |k: &ResKey| k.resref.to_string().to_ascii_lowercase();
        keys.sort_by_cached_key(|(k, _)| {
            (resref(k), !has_own_view(*k), k.restype.extension().unwrap_or_default())
        });
        let mut text = String::with_capacity(keys.len() * 16);
        let mut entries = Vec::with_capacity(keys.len());
        for (key, layer) in keys {
            let start = text.len() as u32;
            let r = resref(&key);
            text.push_str(&r);
            let resref_len = r.len() as u8;
            if let Some(ext) = key.restype.extension() {
                text.push('.');
                text.push_str(ext);
            }
            let file_len = (text.len() - start as usize) as u8;
            text.push('\n');
            let viewed = has_own_view(key);
            entries.push(Entry { key, named: None, start, resref_len, file_len, layer, viewed });
        }
        text.shrink_to_fit();
        Part { entries, text, labels }
    }

    /// About how many bytes it holds: its entries, their names and its
    /// text.
    pub(crate) fn bytes(&self) -> usize {
        let named: usize = (self.entries.iter())
            .filter_map(|e| e.named.as_ref())
            .map(|n| std::mem::size_of::<Named>() + n.shown.capacity() + n.fold.capacity())
            .sum();
        self.entries.capacity() * std::mem::size_of::<Entry>() + named + self.text.capacity()
    }

    /// Its file name (`m1q3_formosa_9.are`).
    pub(crate) fn file(&self, i: usize) -> &str {
        let e = &self.entries[i];
        &self.text[e.start as usize..e.start as usize + usize::from(e.file_len)]
    }

    /// Its ResRef as typed (ASCII, lower case).
    fn resref(&self, i: usize) -> &str {
        let e = &self.entries[i];
        &self.text[e.start as usize..e.start as usize + usize::from(e.resref_len)]
    }

    fn name_fold(&self, i: usize) -> &str {
        self.entries[i].named.as_ref().map_or("", |n| n.fold.as_str())
    }

    fn set_name(&mut self, i: usize, name: Option<String>) {
        let name = name.filter(|n| !n.trim().is_empty());
        self.entries[i].named = name.map(|shown| Box::new(Named { fold: fold(&shown), shown }));
    }

    fn name(&self, i: usize) -> Option<&str> {
        self.entries[i].named.as_ref().map(|n| n.shown.as_str())
    }

    /// Its name as the popover shows it: its own, else its file's.
    pub(crate) fn shown(&self, i: usize) -> String {
        self.name(i).map_or_else(|| self.file(i).to_string(), str::to_string)
    }

    /// Whether it is of the query's type and holds every word, in its
    /// name or its file's name (its ResRef with its extension).
    fn finds(&self, i: usize, q: &Query) -> bool {
        q.restype.is_none_or(|t| self.entries[i].key.restype == t)
            && q.needle.finds_in(&[self.name_fold(i), self.file(i)])
    }

    /// How it matches (it holds every word: [`Part::finds`]): the best of
    /// its name's, its ResRef's and its file name's, a name before a
    /// ResRef before a file name at one tier.
    fn score(&self, i: usize, q: &Query) -> (Tier, Field) {
        let name = self.name_fold(i);
        let fields =
            [(name, Field::Name), (self.resref(i), Field::ResRef), (self.file(i), Field::File)];
        let best = fields
            .iter()
            .filter(|(f, _)| !f.is_empty())
            .filter_map(|(f, field)| tier_of(f, &q.text).map(|t| (t, *field)))
            .min();
        best.unwrap_or_else(|| {
            let field = if !name.is_empty() && q.needle.finds_in(&[name]) {
                Field::Name
            } else if q.needle.finds_in(&[self.resref(i)]) {
                Field::ResRef
            } else {
                Field::File
            };
            (Tier::Scattered, field)
        })
    }

    /// How each entry's file name (and ResRef) holds a query of one word,
    /// from one search of the text ([`Part::hits`]): `None` for a query of
    /// several words or none.
    fn file_scores(&self, q: &Query) -> Option<Vec<Option<(Tier, Field)>>> {
        let word = q.text.as_str();
        if word.is_empty() || word.contains(' ') {
            return None;
        }
        let mut scores = vec![None; self.entries.len()];
        self.hits(&[word], word, |i, how| scores[i] = Some(how));
        Some(scores)
    }

    /// Where `key` is, if the part has it (its entries are in order of
    /// ResRef).
    fn position(&self, key: &ResKey) -> Option<usize> {
        let resref = key.resref.to_string().to_ascii_lowercase();
        let from = self.entries.partition_point(|e| {
            let at = e.start as usize;
            &self.text[at..at + usize::from(e.resref_len)] < resref.as_str()
        });
        (from..self.entries.len())
            .take_while(|&i| self.resref(i) == resref)
            .find(|&i| self.entries[i].key == *key)
    }

    /// How many entries (not `hidden`) are of the query's type and hold
    /// every word, and the best `most` of them in [`rank`]'s order, for a
    /// part of unnamed resources (the load order's): the text searched at
    /// once for the longest word, the others looked for in each one found.
    pub(crate) fn find(&self, q: &Query, hidden: &[u64], most: usize) -> (usize, Vec<u32>) {
        let words: Vec<&str> = q.text.split(' ').filter(|w| !w.is_empty()).collect();
        let off = |i: usize| hidden.get(i / 64).is_some_and(|w| w & (1 << (i % 64)) != 0);
        let typed = |i: usize| q.restype.is_none_or(|t| self.entries[i].key.restype == t);
        let key = |i: usize, (tier, field): (Tier, Field)| {
            let e = &self.entries[i];
            u64::from(!e.viewed) << 63
                | (tier as u64) << 59
                | (field as u64) << 56
                | u64::from(e.resref_len) << 40
                | i as u64
        };
        let mut keys: Vec<u64> = Vec::new();
        match words.iter().copied().max_by_key(|w| w.len()) {
            // (A type alone: every one of it.)
            None if q.restype.is_some() => {
                for i in 0..self.entries.len() {
                    if typed(i) && !off(i) {
                        keys.push(key(i, (Tier::Scattered, Field::ResRef)));
                    }
                }
            }
            None => return (0, Vec::new()),
            Some(longest) => self.hits(&words, longest, |i, how| {
                if typed(i) && !off(i) {
                    keys.push(key(i, how));
                }
            }),
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

    /// Each entry that holds every word of `words` (`longest` among them)
    /// in its file name, once, with how it matches (the best of its hits).
    fn hits(&self, words: &[&str], longest: &str, mut each: impl FnMut(usize, (Tier, Field))) {
        let whole = words.len() == 1;
        let text = self.text.as_bytes();
        let finder = memchr::memmem::Finder::new(longest.as_bytes());
        // The entry the last hit was in, and how it matched.
        let mut current: Option<(usize, (Tier, Field))> = None;
        let mut at = 0;
        for p in finder.find_iter(text) {
            while at + 1 < self.entries.len() && self.entries[at + 1].start as usize <= p {
                at += 1;
            }
            let e = &self.entries[at];
            let offset = p - e.start as usize;
            let (resref, file) = (usize::from(e.resref_len), usize::from(e.file_len));
            if offset + longest.len() > file {
                continue;
            }
            let how = if !whole {
                // (Every word apart: in the ResRef alone, or the file name.)
                let in_resref = words.iter().all(|w| self.resref(at).contains(w));
                (Tier::Scattered, if in_resref { Field::ResRef } else { Field::File })
            } else {
                let (field, len) = if offset + longest.len() <= resref {
                    (Field::ResRef, resref)
                } else {
                    (Field::File, file)
                };
                let tier = if offset == 0 && longest.len() == len {
                    Tier::Exact
                } else if offset == 0 {
                    Tier::Prefix
                } else if !text[p - 1].is_ascii_alphanumeric() {
                    Tier::WordStart
                } else {
                    Tier::Substring
                };
                (tier, field)
            };
            match &mut current {
                Some((i, best)) if *i == at => *best = (*best).min(how),
                _ => {
                    if let Some((i, best)) = current.take() {
                        each(i, best);
                    }
                    if !whole && !words.iter().all(|w| self.file(at).contains(w)) {
                        continue;
                    }
                    current = Some((at, how));
                }
            }
        }
        if let Some((i, best)) = current {
            each(i, best);
        }
    }
}

/// A row's place in its group: whether it opens only as its bytes (those
/// with a view of their own first), tier, field, the name's length, the
/// name and the ResRef, and the type's extension (the order [`rank`]
/// sorts by).
pub(crate) type Rank<'a> = (bool, Tier, Field, usize, &'a str, &'a str, &'a str);

/// The entries of `part` (of `within`, where given: those an earlier,
/// shorter query found) that are of the query's type and hold every word.
pub(crate) fn matches(part: &Part, within: Option<&[u32]>, q: &Query) -> Vec<u32> {
    match (within, part.file_scores(q)) {
        (Some(within), _) => {
            within.iter().copied().filter(|&i| part.finds(i as usize, q)).collect()
        }
        // (One word: the file names searched at once, the names one by one.)
        (None, Some(files)) => {
            let typed = |i: usize| q.restype.is_none_or(|t| part.entries[i].key.restype == t);
            (0..part.entries.len())
                .filter(|&i| {
                    typed(i) && (files[i].is_some() || part.name_fold(i).contains(q.text.as_str()))
                })
                .map(|i| i as u32)
                .collect()
        }
        (None, None) => {
            (0..part.entries.len() as u32).filter(|&i| part.finds(i as usize, q)).collect()
        }
    }
}

/// The best `most` of `found` (indices into `part`, all of which match),
/// in order: [`Rank`]'s. Only those taken are sorted.
pub(crate) fn rank(part: &Part, found: &[u32], q: &Query, most: usize) -> Vec<u32> {
    if most == 0 {
        return Vec::new();
    }
    // (One word, and many found: the file names' tiers from one search.)
    let files = (found.len() > 256).then(|| part.file_scores(q)).flatten();
    let mut ranked: Vec<(Rank<'_>, u32)> = found
        .iter()
        .map(|&i| {
            let at = i as usize;
            let e = &part.entries[at];
            let (tier, field) = match &files {
                Some(files) => {
                    let name = part.name_fold(at);
                    let by_name = (!name.is_empty())
                        .then(|| tier_of(name, &q.text).map(|t| (t, Field::Name)))
                        .flatten();
                    match (by_name, files[at]) {
                        (Some(a), Some(b)) => a.min(b),
                        (Some(a), None) | (None, Some(a)) => a,
                        (None, None) => part.score(at, q),
                    }
                }
                None => part.score(at, q),
            };
            let name = match part.name_fold(at) {
                "" => part.resref(at),
                name => name,
            };
            let ext = e.key.restype.extension().unwrap_or_default();
            ((!e.viewed, tier, field, name.chars().count(), name, part.resref(at), ext), i)
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
/// revision, which changes whenever its module does (a command, an undo,
/// a file read again from outside: `Workspace::adopt`) and which no other
/// workspace has had.
type ModuleStamp = u64;

/// The layers a load-order part is made from: each one's label and
/// fingerprint (`Layer::fingerprint`), the module's own aside.
type Layers = Vec<(String, u64)>;

/// Where a test holds a build of the load order's part back
/// ([`QuickOpen::hold_builds`]).
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoldAt {
    /// Before it reads the load order (holding the game's data).
    Start,
    /// With its part made, before it hands it over (the game's data let
    /// go of).
    Handing,
}

/// A build of the load order's part on a thread.
#[derive(Debug)]
struct Building {
    layers: Layers,
    /// Where it hands its part back (none: it gave up).
    rx: mpsc::Receiver<Option<(Part, Duration)>>,
    /// Set to have it give up (the game's data is to be let go of, the
    /// module closes, the window ends).
    cancel: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Building {
    /// Has it give up, and waits for its thread (it looks at the flag
    /// every few thousand resources, and every millisecond while a test
    /// holds it).
    fn stop(mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The index, made when first asked for and again only when what it was
/// made from changed.
#[derive(Debug, Default)]
pub(crate) struct Index {
    module_for: Option<ModuleStamp>,
    /// The module's own resources.
    own: Part,
    /// How many of `own` have been looked at for a name.
    named: usize,
    /// The load order's generation (`Moonglow::load_order`) `game` is
    /// taken as current for.
    game_for: Option<u64>,
    game: Arc<Part>,
    /// The layers `game` was made from.
    game_layers: Option<Layers>,
    /// The load order's resources the module has (it overrides them, and
    /// lists them as its own), one bit each, and what that was worked out
    /// for: the module's stamp and the load order's build.
    hidden: Vec<u64>,
    hidden_for: Option<(ModuleStamp, u32)>,
    building: Option<Building>,
    /// A test's hold on the next build ([`QuickOpen::hold_builds`]).
    hold: Option<(HoldAt, mpsc::Receiver<()>)>,
    /// How many times each part was made, and how many builds gave up
    /// (for the tests).
    pub(crate) module_builds: u32,
    pub(crate) game_builds: u32,
    pub(crate) gave_up: Arc<AtomicU32>,
    /// How long the last build of the load order's part took, and of the
    /// module's.
    pub(crate) game_took: Duration,
    pub(crate) module_took: Duration,
}

impl Drop for Index {
    fn drop(&mut self) {
        if let Some(b) = self.building.take() {
            b.stop();
        }
    }
}

/// The load order's resources as the index holds them: of each, the copy
/// in the highest of its layers but the module's own (whose resources are
/// the module's group), so the module's layer is no part of it; `None`
/// where `cancel` was set meanwhile.
pub(crate) fn game_entries(resman: &mg_resman::ResMan, cancel: &AtomicBool) -> Option<Part> {
    let layers = resman.layers();
    let labels: Vec<Arc<str>> = layers.iter().map(|l| Arc::from(shown_layer(l))).collect();
    let mut seen = std::collections::HashSet::new();
    let mut keys = Vec::new();
    for (i, layer) in layers.iter().enumerate() {
        if layer.label == mg_resman::MODULE_LABEL {
            continue;
        }
        for (n, key) in layer.container.keys().enumerate() {
            if n % 4096 == 0 && cancel.load(Ordering::Relaxed) {
                return None;
            }
            if seen.insert(key) {
                keys.push((key, i as u16));
            }
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return None;
    }
    Some(Part::new(keys, labels))
}

/// The layers `resman`'s load-order part is made from ([`Layers`]).
fn layers_of(resman: &mg_resman::ResMan) -> Layers {
    (resman.layers().iter())
        .filter(|l| l.label != mg_resman::MODULE_LABEL)
        .map(|l| (l.label.clone(), l.fingerprint))
        .collect()
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
                index.own = Part::default();
                index.named = 0;
            }
            return;
        };
        let stamp = ws.revision();
        if index.module_for != Some(stamp) {
            let began = Instant::now();
            // (The names already known kept, while they are looked at
            // again: a module changed by one edit keeps showing them.)
            let old = std::mem::take(&mut index.own);
            let known: std::collections::HashMap<ResKey, Box<Named>> =
                (old.entries.into_iter()).filter_map(|e| Some((e.key, e.named?))).collect();
            index.own = Part::new(ws.module.keys().map(|k| (*k, 0)).collect(), Vec::new());
            for e in &mut index.own.entries {
                e.named = known.get(&e.key).cloned();
            }
            index.named = 0;
            index.module_for = Some(stamp);
            index.module_builds += 1;
            index.module_took = began.elapsed();
        }
        let began = Instant::now();
        let game = game.as_deref();
        while index.named < index.own.entries.len() {
            let at = index.named;
            let key = index.own.entries[at].key;
            let named = if key.restype == ResType::ARE {
                Some(area_names.name(ws, game, key.resref))
            } else if mg_area::ObjectKind::from_restype(key.restype).is_some() {
                Some(area_names.blueprint(ws, game, key))
            } else {
                None
            };
            if let Some(name) = named {
                index.own.set_name(at, Some(name));
            }
            index.named += 1;
            if quick_open.background && began.elapsed() > BUDGET {
                break;
            }
        }
    }

    /// The load order's part, current for this generation: kept where the
    /// layers it was made from are those of now (a save, an area made, a
    /// file of the module's read again move the generation and touch only
    /// the module's layer); a build's part taken whenever it arrives, if
    /// its layers are those of now; else made again, on a thread where
    /// the window runs its work in the background, else at once. A part
    /// of other layers is let go of as a new one is begun.
    fn game_part(app: &mut Moonglow, ctx: &egui::Context) {
        let generation = app.load_order;
        let layers = app.game.as_ref().map(|g| layers_of(&g.resman));
        let index = &mut app.quick_open.index;
        if let Some(b) = &index.building {
            match b.rx.try_recv() {
                Ok(Some((part, took))) => {
                    let b = index.building.take().expect("a build");
                    if Some(&b.layers) == layers.as_ref() {
                        index.game = Arc::new(part);
                        index.game_layers = Some(b.layers.clone());
                        index.game_took = took;
                        index.game_builds += 1;
                        index.hidden_for = None;
                    }
                    b.stop();
                }
                Ok(None) | Err(mpsc::TryRecvError::Disconnected) => {
                    index.building.take().expect("a build").stop();
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if index.game_for == Some(generation) {
            return;
        }
        let Some(game) = app.game.clone() else {
            index.forget_game();
            index.game_for = Some(generation);
            index.game_builds += 1;
            return;
        };
        let layers = layers.expect("the game's layers");
        if index.game_layers.as_ref() == Some(&layers) {
            index.game_for = Some(generation);
            return;
        }
        if index.building.as_ref().is_some_and(|b| b.layers == layers) {
            return;
        }
        // (Never a part of other layers beside the new one.)
        index.forget_game();
        if app.background_jobs {
            let (tx, rx) = mpsc::channel();
            let cancel = Arc::new(AtomicBool::new(false));
            let gave_up = index.gave_up.clone();
            let hold = index.hold.take();
            let ctx = ctx.clone();
            let stop = cancel.clone();
            let thread = std::thread::spawn(move || {
                // (A test's hold: the flag looked at every millisecond.)
                let held = |at: HoldAt| {
                    let Some((when, gate)) = &hold else { return true };
                    if *when != at {
                        return true;
                    }
                    loop {
                        if stop.load(Ordering::Relaxed) {
                            return false;
                        }
                        match gate.recv_timeout(Duration::from_millis(1)) {
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                            _ => return true,
                        }
                    }
                };
                let began = Instant::now();
                let part = held(HoldAt::Start).then(|| game_entries(&game.resman, &stop)).flatten();
                // (The game's data let go of at once: what changes it waits
                // for every holder, `Moonglow::release_game_data`.)
                drop(game);
                let part = part.filter(|_| held(HoldAt::Handing));
                if part.is_none() {
                    gave_up.fetch_add(1, Ordering::Relaxed);
                }
                let _ = tx.send(part.map(|p| (p, began.elapsed())));
                ctx.request_repaint();
            });
            index.building = Some(Building { layers, rx, cancel, thread: Some(thread) });
        } else {
            let began = Instant::now();
            let never = AtomicBool::new(false);
            if let Some(part) = game_entries(&game.resman, &never) {
                index.game = Arc::new(part);
                index.game_layers = Some(layers);
                index.game_took = began.elapsed();
                index.game_for = Some(generation);
                index.game_builds += 1;
                index.hidden_for = None;
            }
        }
    }

    /// The load order's part let go of (with a build under way).
    pub(crate) fn forget_game(&mut self) {
        if let Some(b) = self.building.take() {
            b.stop();
        }
        self.game = Arc::new(Part::default());
        self.game_layers = None;
        self.game_for = None;
        self.hidden = Vec::new();
        self.hidden_for = None;
    }

    /// A build under way gives up (the game's data is to be let go of);
    /// it is begun again the next time Quick Open opens.
    pub(crate) fn give_up_build(&mut self) {
        if let Some(b) = self.building.take() {
            b.stop();
            self.game_for = None;
        }
    }

    /// The load order's resources the module has, worked out again where
    /// either changed ([`Index::hidden`]).
    fn hide(&mut self, ws: Option<&mg_edit::Workspace>) {
        let now = (self.module_for.unwrap_or(u64::MAX), self.game_builds);
        if self.hidden_for == Some(now) {
            return;
        }
        let mut hidden = vec![0u64; self.game.entries.len().div_ceil(64)];
        if let Some(ws) = ws {
            for key in ws.module.keys() {
                if let Some(i) = self.game.position(key) {
                    hidden[i / 64] |= 1 << (i % 64);
                }
            }
        }
        self.hidden = hidden;
        self.hidden_for = Some(now);
    }

    /// Whether the index is still being made (names or the load order).
    fn indexing(&self) -> bool {
        self.building.is_some() || self.named < self.own.entries.len()
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
    game_for: Option<(u32, Option<(ModuleStamp, u32)>)>,
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
    /// An input method is composing in the field ([`read_keys`]).
    composing: bool,
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

    /// Holds the next build of the load order's part `at` that point
    /// until `gate` is sent to (or dropped), for a test to see what the
    /// window does meanwhile; the build gives up as ever if asked to.
    #[doc(hidden)]
    pub fn hold_builds(&mut self, at: HoldAt, gate: mpsc::Receiver<()>) {
        self.index.hold = Some((at, gate));
    }

    /// How many builds of the load order's part gave up (for the tests).
    #[doc(hidden)]
    pub fn builds_given_up(&self) -> u32 {
        self.index.gave_up.load(Ordering::Relaxed)
    }

    /// Whether a build of the load order's part is under way (for the
    /// tests).
    #[doc(hidden)]
    pub fn building(&self) -> bool {
        self.index.building.is_some()
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

/// Whether a command of the window runs by its keys while Quick Open's
/// field has the keys (open or not). The rule: what saves, builds, tests,
/// shows or hides a panel, opens a view or a tool that changes nothing
/// until asked, or helps runs (Save, Compile, Build Module, Verify, Test
/// Module, the Log's switch, Options, the Faction Editor, the manual...);
/// what changes or closes the module, a resource, a tab or what the
/// window holds waits, its keys left to the field (Undo and Redo, New,
/// Open and Close Module, Import, the wizards and New Script, Close Tab
/// and the tabs' and panes' keys, Reload Resources, Clear Log...): typing
/// a search is no place for them. Cut, Copy, Paste and Delete are the
/// field's own keys, no command's. The plugins' and the project's
/// commands wait too (they read and change the module). Every command is
/// named here (no other arm), so a new one is put on one side or the
/// other where it is added.
pub(crate) fn runs_from_field(id: Id) -> bool {
    match id {
        Id::Save
        | Id::SaveAs
        | Id::SaveAsProject
        | Id::Export
        | Id::CompileAll
        | Id::CompileModels
        | Id::BuildModule
        | Id::PublishNwsync
        | Id::Verify
        | Id::RoundTrip
        | Id::TestModule
        | Id::TestChoose
        | Id::PackTarget
        | Id::AreaStatistics
        | Id::ModuleProperties
        | Id::FindInstance
        | Id::FindReferences
        | Id::AdvancedSearch
        | Id::GoToStart
        | Id::Factions
        | Id::Journal
        | Id::TalkTable
        | Id::OpenTable
        | Id::Palettes
        | Id::PreviewWindow
        | Id::PlaceableGallery
        | Id::Resources
        | Id::Options
        | Id::Plugins
        | Id::Manual
        | Id::CommandPalette
        | Id::QuickOpen
        | Id::CheckUpdates
        | Id::About
        | Id::FullScreen
        | Id::ViewTree
        | Id::ViewPalettes
        | Id::ViewLog
        | Id::CopyLog
        | Id::SaveLog
        | Id::HidePanels => true,
        Id::NewModule
        | Id::OpenModule
        | Id::OpenFolder
        | Id::Import
        | Id::ImportFiles
        | Id::ImportFolder
        | Id::ImportConversation
        | Id::Close
        | Id::Exit
        | Id::Undo
        | Id::Redo
        | Id::BuilderCode
        | Id::ClaimsTable
        | Id::ResizeArea
        | Id::RotateArea
        | Id::ChangeTileset
        | Id::EditAreasTogether
        | Id::UpdateAllInstances
        | Id::ReplaceText
        | Id::IndentToTabs
        | Id::IndentToSpaces
        | Id::AreaWizard
        | Id::CreatureWizard
        | Id::Wizard(_)
        | Id::NewConversation
        | Id::NewScript
        | Id::NewNui
        | Id::NewTileset
        | Id::OpenTileset
        | Id::NewHak
        | Id::OpenHak
        | Id::BuildHak
        | Id::ReloadResources
        | Id::TakeInLeftOut
        | Id::InstallPlugin
        | Id::CloseTab
        | Id::NextTab
        | Id::PreviousTab
        | Id::ReopenTab
        | Id::MaximizeWindow
        | Id::SplitEditor
        | Id::NextPane
        | Id::PreviousPane
        | Id::ClearLog
        | Id::ResetLayout => false,
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
        qo.keys = read_keys(ui, &mut qo.composing);
    }
    if qo.focus_asked && enabled {
        qo.focus_asked = false;
        ui.memory_mut(|m| m.request_focus(id));
        // (Opened with `>` or nothing: that is where its Undo stops, not
        // the text of an earlier search.)
        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
            state.set_undoer(Default::default());
            state.store(ui.ctx(), id);
        }
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
fn read_keys(ui: &mut Ui, composing: &mut bool) -> Keys {
    use egui::{Event, ImeEvent, Key, Modifiers};
    ui.input_mut(|i| {
        let mut keys = Keys::default();
        // (An input method composing a character: Enter, the arrows and
        // Escape are its, which commit, move in or drop what is composed,
        // in the frame that ends it too.)
        let mut ime = *composing;
        for e in &i.events {
            if let Event::Ime(e) = e {
                ime = true;
                *composing = match e {
                    ImeEvent::Preedit { text, .. } => !text.is_empty(),
                    ImeEvent::Commit(_) => false,
                    _ => *composing,
                };
            }
        }
        if ime {
            return keys;
        }
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
    // (A menu opened, by the pointer or its key: Quick Open closes, the
    // menu has the keys.)
    if egui::Popup::is_any_open(&ctx) {
        app.quick_open.close(&ctx, false);
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
    // (The frame's own margin and line round the rows, kept inside too.)
    let mut style = (*ctx.global_style()).clone();
    mg_uikit::menu_style(&mut style);
    let frame = egui::Frame::popup(&style)
        .inner_margin(mg_uikit::menu::MENU_MARGIN)
        .corner_radius(mg_uikit::metrics::POPOVER_RADIUS);
    let around = frame.total_margin().sum();
    let room = (window.width() - 2.0 * EDGE - around.x).max(0.0);
    let width = anchor.width().max(want).min(room);
    let outer = width + around.x;
    let left = anchor.left().min(window.right() - EDGE - outer).max(window.left() + EDGE);
    let top = anchor.bottom() + BELOW;
    let extra = usize::from(found.more > 0)
        + usize::from(found.indexing)
        + usize::from(found.hits.is_empty() && !found.indexing);
    // (The notes under the rows, always in sight; the rows scroll.)
    let tallest = (window.bottom() - EDGE - top - around.y - extra as f32 * ROW).max(ROW);
    let height = ((found.hits.len() as f32) * ROW).min(tallest);
    qo.page = ((height / ROW).floor() as usize).max(1);
    let mut chosen = None;
    let selected = qo.selected;
    let reveal = std::mem::take(&mut qo.reveal);
    let shown = egui::Area::new(egui::Id::new("quick-open-popover"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(left, top))
        .show(&ctx, |ui| {
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
    // (Its detail, the dim words after its name, said as its description:
    // three blueprints named Guard are told apart by their files.)
    if !hit.detail.is_empty() {
        r.ctx.accesskit_node_builder(r.id, |node| node.set_description(hit.detail.as_str()));
    }
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
        Choice::Plugin(..) | Choice::Entry(..) if no_module => "Open a module first".into(),
        Choice::Project(_) => "The project's file gives this command nothing to run".into(),
        // (An exporter for what is selected: `plugin_entry_enabled`.)
        Choice::Entry(kind, plugin, id) => {
            let entries = app.plugin_entries(*kind);
            let entry = entries.iter().find(|e| e.plugin == *plugin && e.id == *id);
            let targets: Vec<String> =
                entry.map(|e| e.targets.iter().map(|t| t.name()).collect()).unwrap_or_default();
            match targets.is_empty() {
                true => "The plugin offers it for none of what is selected".into(),
                false => format!("Select what it exports first: {}", targets.join(", ")),
            }
        }
        Choice::Plugin(..) => "The plugin offers it for none of what is selected".into(),
    }
}

/// A resource's row: the `i`th of the module's part, or (`game`) of the
/// load order's.
fn resource_hit(part: &Part, i: usize, game: bool) -> Hit {
    let e = &part.entries[i];
    let layer = part.labels.get(usize::from(e.layer)).map(|l| l.to_string()).unwrap_or_default();
    let detail = match (part.name(i), game) {
        (Some(_), true) => format!("{}, {layer}", part.file(i)),
        (None, true) => layer,
        (Some(_), false) => part.file(i).to_string(),
        (None, false) => String::new(),
    };
    Hit {
        target: if game { Target::Game(e.key) } else { Target::Resource(e.key) },
        name: part.shown(i),
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
    // (The commands by every word typed; the resources by the words, a
    // last word naming a type narrowing them: [`Query`].)
    let needle = Needle::new(typed);
    let query = query_of(&needle);
    let q = Query::new(typed);
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
                let own = &quick_open.index.own;
                let known = own.position(&key).and_then(|i| own.name(i).map(str::to_string));
                let name =
                    known.unwrap_or_else(|| area_names.name(ws, game.as_deref(), key.resref));
                let mut part = Part::new(vec![(key, 0)], Vec::new());
                part.set_name(0, Some(name));
                hits.push(resource_hit(&part, 0, false));
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
    narrow(app, typed, &q, commands_only);
    let qo = &app.quick_open;
    let index = &qo.index;
    if !commands_only {
        if app.ws.is_some() {
            total += qo.narrowed.own.len();
            let best = qo.narrowed.own_best.iter();
            hits.extend(best.map(|&i| resource_hit(&index.own, i as usize, false)));
        } else if q.restype.is_none() {
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
        hits.extend(best.map(|&i| resource_hit(&index.game, i as usize, true)));
    }
    hits.truncate(MOST);
    Found { more: total.saturating_sub(hits.len()), indexing, hits }
}

/// The module's and the load order's entries that match (`narrowed`),
/// found again only where what is typed or the index changed, and from
/// those found before where what is typed only grew. The load order's
/// leave out those the module has ([`Index::hide`]): a resource the
/// module overrides is listed once, as the module's.
fn narrow(app: &mut Moonglow, typed: &str, q: &Query, commands_only: bool) {
    let Moonglow { quick_open: qo, ws, .. } = app;
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
    qo.index.hide(ws.as_ref());
    let index = &qo.index;
    let n = &mut qo.narrowed;
    let same_index = n.module_for == index.module_for && n.named == index.named;
    let game_now = Some((index.game_builds, index.hidden_for));
    let same_game = n.game_for == game_now;
    if n.text == typed && same_index && same_game {
        return;
    }
    // (Typing on narrows: the words typed before are each part of one
    // typed now, a type's word part of the file's name. A name taken
    // since may match where it did not.)
    let grew = !n.text.is_empty() && typed.starts_with(n.text.as_str());
    let module_within = (grew && same_index).then_some(n.own.as_slice());
    let own = matches(&index.own, module_within, q);
    // (The load order's searched anew: [`Part::find`].)
    if !(same_game && n.text == typed) {
        (n.game_found, n.game_best) = index.game.find(q, &index.hidden, MOST);
    }
    n.own_best = rank(&index.own, &own, q, MOST);
    n.own = own;
    n.text = typed.to_string();
    n.module_for = index.module_for;
    n.named = index.named;
    n.game_for = game_now;
}

/// Does what a row stands for, the popover closed.
fn choose(app: &mut Moonglow, ctx: &egui::Context, target: Target) {
    match target {
        // (As the Module panel opens it: an area's tab, a blueprint's
        // Properties, a script's editor; one open is brought to the front.
        // One with no editor of its own, read-only, as the Resources
        // panel shows it.)
        Target::Resource(key) if Tab::for_resource(key).is_some() => {
            app.actions.push(Action::OpenResource(key))
        }
        Target::Resource(key) => app.actions.push(Action::OpenTab(Tab::Resource(key))),
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
) -> (usize, usize, Duration, Vec<(String, usize, Duration)>) {
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
    let mut game = Part::default();
    let never = AtomicBool::new(false);
    let built = best(&mut || game = game_entries(resman, &never).unwrap_or_default());
    let mut out = Vec::new();
    for typed in queries {
        let q = Query::new(typed);
        let mut found = 0;
        let took = best(&mut || found = game.find(&q, &[], MOST).0);
        out.push((typed.to_string(), found, took));
    }
    (game.entries.len(), game.bytes(), built, out)
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
    let keys: Vec<(ResKey, u16)> = (0..count)
        .filter_map(|i| {
            let (prefix, _, t) = KINDS[i % KINDS.len()];
            Some((ResKey::parse(&format!("{prefix}_{i:05}"), t)?, 0))
        })
        .collect();
    let mut part = Part::new(keys, Vec::new());
    for i in 0..part.entries.len() {
        let n: usize = part.resref(i)[part.resref(i).len() - 5..].parse().unwrap_or_default();
        let name = KINDS[n % KINDS.len()].1;
        part.set_name(i, Some(format!("{name} {n}")));
    }
    queries
        .iter()
        .map(|typed| {
            let q = Query::new(typed);
            let mut found = 0;
            let took = (0..tries.max(1))
                .map(|_| {
                    let began = Instant::now();
                    let all = matches(&part, None, &q);
                    rank(&part, &all, &q, MOST);
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

    /// A part of these resources, each with its name where given.
    fn part(list: &[(&str, ResType, Option<&str>)]) -> Part {
        let keys = list.iter().map(|(n, t, _)| (key(n, *t), 0)).collect();
        let mut part = Part::new(keys, vec![Arc::from("Game")]);
        for (n, t, shown) in list {
            let at = part.position(&key(n, *t)).unwrap();
            part.set_name(at, shown.map(str::to_string));
        }
        part
    }

    /// The names of the best of `part` for `typed`, in order.
    fn ranked(part: &Part, typed: &str) -> Vec<String> {
        let q = Query::new(typed);
        let found = matches(part, None, &q);
        let best = rank(part, &found, &q, MOST);
        best.iter().map(|&i| part.shown(i as usize)).collect()
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
        let entries = part(&[
            ("informosa", ResType::NSS, None),
            ("m1q3_formosa_9", ResType::ARE, Some("Port of Formosa")),
            ("formosa_inn", ResType::ARE, None),
            ("formosa", ResType::DLG, None),
            ("abc", ResType::ARE, Some("Formosa Bay")),
            ("formosa_bay", ResType::UTC, Some("Sailor")),
            ("x", ResType::UTP, Some("Formosas")),
        ]);
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
        let entries =
            part(&[("gate", ResType::UTD, Some("Door")), ("door", ResType::UTD, Some("Old"))]);
        // ("door" is a ResRef's exact match, and the other's name's.)
        assert_eq!(ranked(&entries, "door"), ["Door", "Old"]);
    }

    #[test]
    fn words_apart_rank_after_the_whole_and_find_across_fields() {
        let entries = part(&[
            ("sword01", ResType::UTI, Some("Iron Sword")),
            ("iron_sword", ResType::UTI, None),
            ("ironsword", ResType::UTI, Some("Plain")),
        ]);
        // ("Iron Sword" whole; then every word apart: the shorter first.)
        assert_eq!(ranked(&entries, "iron sword"), ["Iron Sword", "Plain", "iron_sword.uti"]);
        // (Apart all three, the name's match first.)
        assert_eq!(ranked(&entries, "sword iron"), ["Iron Sword", "Plain", "iron_sword.uti"]);
        // (Every word, one in the name and one in the ResRef.)
        assert_eq!(ranked(&entries, "plain ironsw"), ["Plain"]);
    }

    #[test]
    fn case_accents_and_other_scripts_fold() {
        let entries = part(&[
            ("a1", ResType::ARE, Some("Árbol Seco")),
            ("a2", ResType::ARE, Some("Деревня Колдуна")),
            ("a3", ResType::ARE, Some("Ёлка")),
        ]);
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
        let entries = part(&[("inn", ResType::ARE, Some("The Inn"))]);
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
        let names: Vec<String> = (0..500).map(|i| format!("zz_tree{i:03}")).collect();
        let mut list: Vec<(&str, ResType, Option<&str>)> =
            names.iter().map(|n| (n.as_str(), ResType::UTP, None)).collect();
        list.push(("tree", ResType::UTP, None));
        let entries = part(&list);
        let q = Query::new("tree");
        let found = matches(&entries, None, &q);
        assert_eq!(found.len(), 501);
        let best = rank(&entries, &found, &q, MOST);
        assert_eq!(best.len(), MOST);
        assert_eq!(entries.shown(best[0] as usize), "tree.utp");
        // (Then word starts, alphabetical: zz_tree000, 001...)
        assert_eq!(entries.shown(best[1] as usize), "zz_tree000.utp");
        assert_eq!(entries.shown(best[MOST - 1] as usize), "zz_tree048.utp");
        assert!(rank(&entries, &found, &q, 0).is_empty());
    }

    #[test]
    fn narrowing_from_an_earlier_query_finds_the_same() {
        let entries = part(&[
            ("formosa", ResType::ARE, None),
            ("form", ResType::NSS, None),
            ("fort", ResType::ARE, None),
            ("formula", ResType::NCS, None),
            ("deform", ResType::ARE, None),
        ]);
        let short = matches(&entries, None, &Query::new("for"));
        // (A type's word too: its extension is part of the file's name.)
        for longer in ["form", "form a", "form are", "for .ncs", "for n", "for nss"] {
            let long = Query::new(longer);
            let narrowed = matches(&entries, Some(&short), &long);
            assert_eq!(narrowed, matches(&entries, None, &long), "{longer}");
        }
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
    /// workspace's revision or the load order's generation changed: the
    /// builds counted.
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
        // (A file read again from outside: a new revision too.)
        let data: Arc<[u8]> = Arc::from(&b"x"[..]);
        app.ws.as_mut().unwrap().adopt(&[(key("cellar", ResType::ARE), Some(data))]).unwrap();
        assert_eq!(refresh(&mut app), (3, 2), "a resource read from outside");
        let own = &app.quick_open.index.own;
        let keys: Vec<&str> = (0..own.entries.len()).map(|i| own.file(i)).collect();
        assert_eq!(keys, ["cellar.are", "inn.are"]);
    }

    /// The load order's search (one text, looked through at once) finds
    /// and ranks as the module's does, entry by entry, file names and
    /// types too.
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
            "nss_tool",
        ];
        let mut list = Vec::new();
        for n in names {
            for t in [ResType::NSS, ResType::NCS, ResType::UTP, ResType::TGA] {
                list.push((n, t, None));
            }
        }
        let game = part(&list);
        let typed = [
            "formosa",
            "for",
            "a",
            "b a",
            "tree",
            "nw c2",
            "zzz",
            "o",
            "ab",
            "formosa nss",
            ".ncs",
            "a.n",
            "default1.nss",
            "nss",
            "nss_tool.ncs",
            "tree tga",
            "s",
        ];
        for typed in typed {
            let q = Query::new(typed);
            let found = matches(&game, None, &q);
            // (All of them: a word's start later in a ResRef, as in
            // `xformosa_formosa`, ranks above a plain substring.)
            let all = game.entries.len();
            let by_rule = rank(&game, &found, &q, all);
            let (count, best) = game.find(&q, &[], all);
            assert_eq!(count, found.len(), "{typed}");
            assert_eq!(best, by_rule, "{typed}");
        }
        // (Those with a view of their own first, whatever their match: a
        // compiled script and a texture after every source and blueprint,
        // in the match's order among themselves.)
        let (_, best) = game.find(&Query::new("tree"), &[], 8);
        let files: Vec<&str> = best.iter().map(|&i| game.file(i as usize)).collect();
        let order = [
            "tree.nss",
            "tree.utp",
            "zz_tree000.nss",
            "zz_tree000.utp",
            "tree.ncs",
            "tree.tga",
            "zz_tree000.ncs",
            "zz_tree000.tga",
        ];
        assert_eq!(files, order);
        // (Hidden: the module has it.)
        let at = game.position(&key("tree", ResType::NSS)).unwrap();
        let mut hidden = vec![0u64; all_words(&game)];
        hidden[at / 64] |= 1 << (at % 64);
        let (count, best) = game.find(&Query::new("tree"), &hidden, 4);
        assert_eq!(count, 7, "eight less the one hidden");
        assert!(!best.contains(&(at as u32)));
    }

    fn all_words(part: &Part) -> usize {
        part.entries.len().div_ceil(64)
    }

    /// A file's name is found with its extension (`hello.nss`), and a
    /// last word naming a type narrows to it (`hello nss`, `.utc`): a file
    /// name's match ranks after a name's and a ResRef's of its tier.
    #[test]
    fn a_file_name_and_a_type_are_found() {
        let p = part(&[
            ("hello", ResType::NSS, None),
            ("hello", ResType::NCS, None),
            ("guard", ResType::UTC, Some("Guard")),
            ("hello_x", ResType::UTC, Some("Hello")),
        ]);
        assert_eq!(ranked(&p, "hello.nss"), ["hello.nss"]);
        assert_eq!(ranked(&p, "hello nss"), ["hello.nss"]);
        assert_eq!(ranked(&p, "hello .ncs"), ["hello.ncs"]);
        assert_eq!(ranked(&p, ".utc"), ["Guard", "Hello"]);
        assert_eq!(ranked(&p, "HELLO.NSS"), ["hello.nss"]);
        // (Its name equal: before a ResRef's equal, one with a view of its
        // own before a compiled script.)
        assert_eq!(ranked(&p, "hello"), ["Hello", "hello.nss", "hello.ncs"]);
        // (`nss` alone is a word, not a type: the file names that hold it.)
        assert_eq!(ranked(&p, "nss"), ["hello.nss"]);
        let q = Query::new("hello nss");
        assert_eq!((q.restype, q.text.as_str()), (Some(ResType::NSS), "hello"));
        let q = Query::new("hello zzz");
        assert_eq!((q.restype, q.text.as_str()), (None, "hello zzz"));
    }

    /// Against the Module panel's Filter (`tree.rs`: every word in the
    /// key's text, `hello.nss`, or the name) and the Resources panel's
    /// search (every word in the ResRef), case by case: what either finds,
    /// Quick Open finds; beyond them, a last word naming a type narrows.
    #[test]
    fn it_finds_what_the_module_and_resources_panels_find() {
        let p = part(&[
            ("hello", ResType::NSS, None),
            ("hello", ResType::NCS, None),
            ("inn", ResType::ARE, Some("The Inn")),
            ("cre_guard", ResType::UTC, Some("Guard Captain")),
        ]);
        let tree = |q: &str, i: usize| {
            let k = p.entries[i].key;
            let name = p.name(i).map(fold).unwrap_or_default();
            Needle::new(q).finds_in(&[&k.to_string().to_ascii_lowercase(), &name])
        };
        let resources =
            |q: &str, i: usize| Needle::new(q).finds(&p.entries[i].key.resref.to_string());
        let quick = |q: &str, i: usize| p.finds(i, &Query::new(q));
        // (What is typed, the file, and whether the Filter, the Resources
        // panel and Quick Open find it.)
        let cases: [(&str, &str, bool, bool, bool); 16] = [
            ("hello", "hello.nss", true, true, true),
            ("hello.nss", "hello.nss", true, false, true),
            ("HELLO.NSS", "hello.nss", true, false, true),
            ("hello nss", "hello.nss", true, false, true),
            ("hello nss", "hello.ncs", false, false, false),
            ("hello ncs", "hello.ncs", true, false, true),
            (".ncs", "hello.ncs", true, false, true),
            (".ncs", "hello.nss", false, false, false),
            ("nss", "hello.nss", true, false, true),
            ("the inn", "inn.are", true, false, true),
            ("inn", "inn.are", true, true, true),
            ("captain", "cre_guard.utc", true, false, true),
            ("guard captain", "cre_guard.utc", true, false, true),
            ("cre guard", "cre_guard.utc", true, true, true),
            ("utc", "cre_guard.utc", true, false, true),
            ("zzz", "inn.are", false, false, false),
        ];
        for (q, file, in_tree, in_resources, in_quick) in cases {
            let i = (0..p.entries.len()).find(|&i| p.file(i) == file).unwrap();
            assert_eq!(tree(q, i), in_tree, "the Filter, {q:?} on {file}");
            assert_eq!(resources(q, i), in_resources, "the Resources panel, {q:?} on {file}");
            assert_eq!(quick(q, i), in_quick, "Quick Open, {q:?} on {file}");
            assert!(!(in_tree || in_resources) || in_quick, "{q:?} on {file}: found by a panel");
        }
    }

    /// What has a view of its own (a tab of its kind, a 2DA's table, a
    /// text) and what opens only as its bytes, as `Tab::for_resource` and
    /// the Resources panel's view have them.
    #[test]
    fn what_opens_in_a_view_of_its_own() {
        for t in [ResType::NSS, ResType::ARE, ResType::UTC, ResType::DLG, ResType::MDL] {
            assert!(has_own_view(key("x", t)), "{t:?}");
        }
        for t in [ResType::TWODA, ResType::TXT, ResType::SET, ResType::GIT] {
            assert!(has_own_view(key("x", t)), "{t:?}");
        }
        for t in [ResType::NCS, ResType::NDB, ResType::TGA, ResType::DDS, ResType::WAV] {
            assert!(!has_own_view(key("x", t)), "{t:?}");
        }
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
            let game = &app.quick_open.index.game;
            let names: Vec<String> =
                (0..game.entries.len()).map(|i| game.file(i).to_string()).collect();
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

    /// The load order's part is made of every layer but the module's:
    /// what a module overrides is still in it, and shows once, as the
    /// module's (the load order's row left out while the module has the
    /// key). Opening another module, or a copy the module gets or loses,
    /// changes no build; a hak added or taken away does.
    #[test]
    fn the_module_s_layer_is_no_part_of_the_load_order_s() {
        use mg_resman::{LayerClass, MemContainer, ResMan};
        let ctx = egui::Context::default();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        let layer = |keys: &[ResKey]| {
            let mut c = MemContainer::new();
            for k in keys {
                c.insert(*k, b"x".to_vec());
            }
            c
        };
        let script = key("nw_c2_default1", ResType::NSS);
        let base =
            [script, key("nw_c2_default1", ResType::NCS), key("nw_c2_default1", ResType::NDB)];
        let game = |module: &[ResKey], hak: bool| {
            let mut rm = ResMan::new();
            rm.add(1, "key:base", LayerClass::Key, layer(&base));
            if hak {
                rm.add(5, "hak:extra", LayerClass::Erf, layer(&[key("nw_c2_hak", ResType::NSS)]));
            }
            rm.add(9, mg_resman::MODULE_LABEL, LayerClass::Erf, layer(module));
            let tlk = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
            Some(Arc::new(mg_rules::GameData::new(rm, tlk)))
        };
        let module = |keys: &[ResKey]| {
            let mut m = mg_module::Module::new();
            for k in keys {
                m.set(*k, b"x".to_vec());
            }
            Some(mg_edit::Workspace::new(m))
        };
        // What "nw_c2" finds: each row's file and whether it is the module's.
        let found = |app: &mut Moonglow| {
            app.load_order += 1;
            app.quick_open.open_with("");
            app.quick_open.text = "nw_c2".into();
            Index::module_part(app);
            Index::game_part(app, &ctx);
            let rows: Vec<(String, bool)> = (results(app, &ctx).hits.into_iter())
                .filter_map(|h| match h.target {
                    Target::Resource(k) => Some((k.to_string(), true)),
                    Target::Game(k) => Some((k.to_string(), false)),
                    _ => None,
                })
                .collect();
            (app.quick_open.index.game_builds, rows)
        };
        let owned = |rows: &[(&str, bool)]| -> Vec<(String, bool)> {
            rows.iter().map(|(f, m)| (f.to_string(), *m)).collect()
        };
        // Module A has the script: it shows once, as the module's.
        app.game = game(&[script], false);
        app.ws = module(&[script]);
        let a = owned(&[
            ("nw_c2_default1.nss", true),
            ("nw_c2_default1.ncs", false),
            ("nw_c2_default1.ndb", false),
        ]);
        assert_eq!(found(&mut app), (1, a.clone()));
        // Module B hasn't it: the game's copy, the same build.
        app.game = game(&[], false);
        app.ws = module(&[]);
        let b = owned(&[
            ("nw_c2_default1.nss", false),
            ("nw_c2_default1.ncs", false),
            ("nw_c2_default1.ndb", false),
        ]);
        assert_eq!(found(&mut app), (1, b.clone()), "the reviewer's case");
        // A copy into the module, and taken away again.
        let data: Arc<[u8]> = Arc::from(&b"x"[..]);
        app.ws.as_mut().unwrap().adopt(&[(script, Some(data))]).unwrap();
        app.game = game(&[script], false);
        assert_eq!(found(&mut app), (1, a));
        app.ws.as_mut().unwrap().adopt(&[(script, None)]).unwrap();
        app.game = game(&[], false);
        assert_eq!(found(&mut app), (1, b.clone()));
        // A hak added: made again, its script found; taken away: again.
        app.game = game(&[], true);
        let (builds, rows) = found(&mut app);
        assert_eq!(builds, 2);
        assert!(rows.contains(&("nw_c2_hak.nss".to_string(), false)), "{rows:?}");
        app.game = game(&[], false);
        assert_eq!(found(&mut app), (3, b));
    }

    /// Game data of a base layer and, with `hak`, a hak's (its script
    /// `hak_script`).
    fn game_data(hak: Option<&str>) -> Option<Arc<mg_rules::GameData>> {
        use mg_resman::{LayerClass, MemContainer, ResMan};
        let layer = |name: &str| {
            let mut c = MemContainer::new();
            c.insert(key(name, ResType::NSS), b"x".to_vec());
            c
        };
        let mut rm = ResMan::new();
        rm.add(1, "key:base", LayerClass::Key, layer("nw_c2_default1"));
        if let Some(hak) = hak {
            rm.add(5, format!("hak:{hak}"), LayerClass::Erf, layer(hak));
        }
        let tlk = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
        Some(Arc::new(mg_rules::GameData::new(rm, tlk)))
    }

    /// The load order's part looked at until the build under way ends
    /// (a deadline, not a count of tries).
    fn until_built(app: &mut Moonglow, ctx: &egui::Context) {
        let deadline = Instant::now() + Duration::from_secs(30);
        while app.quick_open.index.building.is_some() {
            assert!(Instant::now() < deadline, "the build never ended");
            Index::game_part(app, ctx);
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn game_files(app: &Moonglow) -> Vec<String> {
        let game = &app.quick_open.index.game;
        (0..game.entries.len()).map(|i| game.file(i).to_string()).collect()
    }

    /// A build's part is taken whenever it arrives if the layers it was
    /// made from are those of now, though the generation moved meanwhile
    /// (a save while Quick Open was closed); one made from other layers is
    /// dropped and the part made again.
    #[test]
    fn a_finished_build_is_kept_for_the_layers_it_was_made_from() {
        let ctx = egui::Context::default();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.background_jobs = true;
        app.game = game_data(Some("hak_one"));
        let (hand, held) = mpsc::channel();
        app.quick_open.hold_builds(HoldAt::Handing, held);
        Index::game_part(&mut app, &ctx);
        assert!(app.quick_open.index.building.is_some());
        // (A save: the generation moves, the layers but the module's not.)
        app.load_order += 1;
        Index::game_part(&mut app, &ctx);
        hand.send(()).unwrap();
        until_built(&mut app, &ctx);
        Index::game_part(&mut app, &ctx);
        assert_eq!(app.quick_open.index.game_builds, 1, "taken");
        assert_eq!(app.quick_open.index.game_for, Some(app.load_order));
        assert_eq!(game_files(&app), ["hak_one.nss", "nw_c2_default1.nss"]);
        // Held again while another hak replaces it: dropped, made again.
        app.game = game_data(Some("hak_two"));
        app.load_order += 1;
        let (hand, held) = mpsc::channel();
        app.quick_open.hold_builds(HoldAt::Handing, held);
        Index::game_part(&mut app, &ctx);
        app.game = game_data(Some("hak_three"));
        app.load_order += 1;
        hand.send(()).unwrap();
        until_built(&mut app, &ctx);
        Index::game_part(&mut app, &ctx);
        until_built(&mut app, &ctx);
        assert_eq!(game_files(&app), ["hak_three.nss", "nw_c2_default1.nss"]);
        assert_eq!(app.quick_open.index.game_builds, 2, "the one of other layers not taken");
    }

    /// A build gives up as soon as the game's data is to be let go of:
    /// releasing it waits for no build (the build sees the flag and ends
    /// without a part); one is begun again on the next look. Dropping the
    /// window ends a build too: no thread outlives it.
    #[test]
    fn a_release_does_not_wait_for_a_build() {
        let ctx = egui::Context::default();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.background_jobs = true;
        app.game = game_data(None);
        // (Held where it holds the game's data, as a long build would.)
        let (_hand, held) = mpsc::channel();
        app.quick_open.hold_builds(HoldAt::Start, held);
        Index::game_part(&mut app, &ctx);
        assert_eq!(Arc::strong_count(app.game.as_ref().unwrap()), 2, "the build holds it");
        app.release_game_data();
        assert_eq!(Arc::strong_count(app.game.as_ref().unwrap()), 1, "let go of");
        assert!(app.quick_open.index.building.is_none());
        assert_eq!(app.quick_open.builds_given_up(), 1);
        assert_eq!(app.quick_open.index.game_builds, 0, "no part from it");
        // Begun again (not held), and made.
        Index::game_part(&mut app, &ctx);
        until_built(&mut app, &ctx);
        Index::game_part(&mut app, &ctx);
        assert_eq!(app.quick_open.index.game_builds, 1);
        // A build held when the window goes: it ends with it.
        app.load_order += 1;
        app.game = game_data(Some("hak_one"));
        let (_hand, held) = mpsc::channel();
        app.quick_open.hold_builds(HoldAt::Start, held);
        Index::game_part(&mut app, &ctx);
        let gave_up = app.quick_open.index.gave_up.clone();
        let game = app.game.clone().unwrap();
        drop(app);
        assert_eq!(gave_up.load(Ordering::Relaxed), 2);
        assert_eq!(Arc::strong_count(&game), 1, "no thread holds it");
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

    /// The commands with keys, each on one side of the field's rule: what
    /// changes or closes the module or a tab waits, saving and building
    /// run.
    #[test]
    fn every_command_with_a_key_runs_from_the_field_or_waits() {
        let keymap = crate::keys::Keymap::default();
        let keyed: Vec<Id> =
            Id::all().into_iter().filter(|id| !keymap.keys_of(&id.id()).is_empty()).collect();
        assert!(keyed.len() > 20, "{keyed:?}");
        let (run, wait): (Vec<Id>, Vec<Id>) =
            keyed.into_iter().partition(|id| runs_from_field(*id));
        for id in [Id::Undo, Id::Redo, Id::NewModule, Id::OpenModule, Id::CloseTab] {
            assert!(wait.contains(&id), "{id:?} waits");
        }
        // (Given a key by the user: Close Module waits too.)
        assert!(!runs_from_field(Id::Close));
        for id in [Id::NextTab, Id::ReopenTab, Id::NewScript, Id::NewConversation, Id::Exit] {
            assert!(wait.contains(&id), "{id:?} waits");
        }
        for id in [Id::Save, Id::CompileAll, Id::TestModule, Id::ViewLog, Id::QuickOpen] {
            assert!(run.contains(&id), "{id:?} runs");
        }
    }

    /// An entry is small: the load order's hundreds of thousands are held
    /// while a module stays open.
    #[test]
    fn an_entry_is_small() {
        assert!(std::mem::size_of::<Entry>() <= 40, "{}", std::mem::size_of::<Entry>());
    }

    #[test]
    fn a_layer_of_the_game_is_named_so() {
        // (No layers: nothing; the module's own is left out.)
        let rm = mg_resman::ResMan::new();
        let never = AtomicBool::new(false);
        assert!(game_entries(&rm, &never).unwrap().entries.is_empty());
    }
}
