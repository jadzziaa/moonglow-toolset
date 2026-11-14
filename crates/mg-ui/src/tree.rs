//! The module contents tree (Aurora's left pane).

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use egui::Ui;
use mg_core::text::Needle;
use mg_core::{ResRef, ResType};
use mg_edit::Workspace;
use mg_gff::Gff;
use mg_resman::ResKey;
use mg_rules::{GameData, ObjectKind};

use crate::{Action, Moonglow, Tab};

/// What Make Placeables Static does, for its menu items.
const STATIC_TIP: &str = "Set Static on the placeables that lose nothing by it, which the game \
                          draws faster";

/// The tree's groups: the module's own kinds of resource, then each
/// kind of blueprint (`mg_rules::ObjectKind`'s order and names), then the
/// journal and factions.
pub(crate) const GROUPS: &[(&str, &[ResType])] = &{
    let mut groups: [(&str, &[ResType]); 14] = [("", &[]); 14];
    groups[0] = ("Areas", &[ResType::ARE]);
    groups[1] = ("Conversations", &[ResType::DLG]);
    groups[2] = ("Scripts", &[ResType::NSS]);
    groups[3] = ("NUI windows", &[ResType::JUI]);
    let mut i = 0;
    while i < ObjectKind::ALL.len() {
        let kind = ObjectKind::ALL[i];
        groups[4 + i] = (kind.label(), kind.restypes());
        i += 1;
    }
    groups[13] = ("Journal and factions", &[ResType::JRL, ResType::FAC]);
    groups
};

/// What lists of areas show and filter by, from an area's ARE: its name
/// (in the editing language, else as the game shows it), tag, tileset,
/// flags and whether the module's area list names it; as `mg areas` has
/// it.
pub(crate) use mg_module::areas::AreaInfo;

/// The areas' names (and tilesets and flags), each read from its ARE once:
/// again when the ARE is replaced, and from the open document while it is
/// being edited.
#[derive(Debug, Default)]
pub(crate) struct AreaNames {
    /// By area: the stored ARE read ([`stored_stamp`]) and what it says.
    read: HashMap<ResRef, (Stamp, AreaInfo)>,
    /// The areas the module's area list names, and what it was read from.
    listed: Option<(ListRead, std::collections::HashSet<ResRef>)>,
    /// Blueprints' names, likewise.
    names: HashMap<ResKey, (Stamp, String)>,
    /// What each of the tree's groups lists, as made for a revision of the
    /// workspace and the two options that shape it (areas by name, ResRefs
    /// beside names): not made again every frame.
    groups: Option<Groups>,
    /// What the Filter finds, kept while it has the same typed.
    filtered: Filtered,
    /// How many times an area's placed objects were read for the tree
    /// (what the Filter found in them is then looked for again).
    reads: u64,
}

/// What the groups' lists were made for ([`Groups::shape`]).
type Shape = (u64, usize, bool, bool);

/// What the Filter finds, kept from frame to frame: it was looked for
/// again every frame (each resource's name lowered, every placed object of
/// every area matched, two to four times), tens of milliseconds a frame in
/// a persistent world for as long as a word stood in the Filter.
#[derive(Debug, Default)]
struct Filtered {
    /// What is typed and the groups' lists `matched` was made from.
    made_for: Option<(String, Shape)>,
    /// The resources found by their own ResRef or name.
    matched: Arc<std::collections::HashSet<ResKey>>,
    /// By area whose placed objects are read: those found, kind by kind
    /// ([`found_in`]), for what is typed; `None` where none is. (An area
    /// read again is taken out, and looked through again.)
    areas: HashMap<ResRef, Option<Arc<Vec<Order>>>>,
    /// What `found` was made for: the areas read (how many times, and how
    /// many are held) and the rows kept under the Filter for a tab chosen.
    found_for: Option<(u64, usize, Option<Vec<ResKey>>)>,
    /// Each group's rows under the Filter, in the group's order.
    found: Arc<Vec<Vec<ResKey>>>,
}

/// The placed objects of an area that the Filter finds: for each kind,
/// their places in the area's list in the order listed; `None` when it
/// finds none (the area is then listed whole, if at all).
fn found_in(contents: &AreaContents, filter: &Needle) -> Option<Vec<Order>> {
    if filter.is_empty() {
        return None;
    }
    let found: Vec<Order> = (contents.iter())
        .map(|c| {
            (c.order.iter().copied()).filter(|&i| filter.finds_folded(&c.rows[i].sought)).collect()
        })
        .collect();
    found.iter().any(|f: &Order| !f.is_empty()).then_some(found)
}

/// What a resource's stored data is told apart by, for what was read from
/// it and kept: where the data is, its length, and the revision at which
/// the resource last changed.
pub(crate) type Stamp = (usize, usize, u64);

/// The [`Stamp`] of `key`'s stored `data`. Replaced data is elsewhere, or
/// of another length, nearly always: but data replaced twice at one length
/// can come back to where the first was (the allocator hands the room
/// out again), and what was kept of the first then passed for the third's
/// where nothing looked in between (a tab behind another, the tree folded
/// away). The revision is what a command, its undo or a file read again
/// moves ([`Workspace::changed_at`]); the place and length, what a
/// replacement past the commands moves.
pub(crate) fn stored_stamp(ws: &Workspace, key: &ResKey, data: &[u8]) -> Stamp {
    (data.as_ptr() as usize, data.len(), ws.changed_at(key))
}

/// What the module's area list was read from: the stored `module.ifo`
/// ([`stored_stamp`]), or the open document at a revision of the
/// workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListRead {
    Stored(Stamp),
    Open(u64),
}

/// A group's resources in the order the tree shows them, and the names of
/// those shown by name.
type GroupList = (Vec<ResKey>, HashMap<ResKey, String>);

/// The groups' lists and what they were made for: the workspace's
/// revision and how many resources the module has (which also tells one
/// put in past the workspace's commands), areas by name, ResRefs beside
/// names.
#[derive(Debug)]
struct Groups {
    shape: Shape,
    lists: Arc<Vec<GroupList>>,
    /// The areas the module's area list leaves out (the game doesn't load
    /// them): marked in the tree.
    unlisted: Arc<std::collections::HashSet<ResRef>>,
}

/// A blueprint's name in the editing language, else as the game shows it:
/// a creature's first and last names, another's `LocalizedName` or
/// `LocName`.
pub(crate) fn blueprint_name(game: Option<&GameData>, root: &mg_gff::Struct) -> String {
    let text = |label: &str| root.locstring(label).map_or(String::new(), |n| shown(game, n));
    let full = [text("FirstName"), text("LastName")];
    let full: Vec<&str> = full.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if !full.is_empty() {
        return full.join(" ");
    }
    let name = text("LocalizedName");
    if name.trim().is_empty() { text("LocName") } else { name }
}

/// The command that adds `area` at the end of the module's area list
/// (`Mod_Area_list`), as the Area Wizard lists a new one.
fn list_area(ws: &Workspace, area: ResRef) -> Option<mg_edit::Command> {
    use mg_schema::{StructExt, ifo};
    let key = mg_module::Module::info_key();
    let count = |info: &Gff| info.root.items(&ifo::MOD_AREA_LIST).len();
    let at = match ws.loaded(&key) {
        Some(info) => count(info),
        None => count(&ws.module.info().ok()?),
    };
    let mut item = ifo::MOD_AREA_LIST.new_item();
    item.write(&ifo::mod_area_list::AREA_NAME, area);
    let label = ifo::MOD_AREA_LIST.label.to_string();
    let edit = mg_edit::Edit::insert_item(key, mg_edit::GffPath::root(), label, at, item);
    Some(mg_edit::Command::new("Add to the area list", vec![edit]))
}

/// What the mark of an area its module's list leaves out says.
pub(crate) const UNLISTED_TIP: &str = "The module's area list doesn't name this area, so the game \
                                       never loads it";

/// A localized string's text as the tree and the area lists show it: in
/// the editing language, else as the game shows it (its talk-table
/// string), else in another language (`mg_rules::shown`, as `mg` shows
/// names).
pub(crate) fn shown(game: Option<&GameData>, s: &mg_core::LocString) -> String {
    mg_rules::shown(game, s, crate::text::edit_language(), crate::text::game_table())
}

impl AreaNames {
    /// Looks for what the Filter finds, where it is not in hand
    /// ([`Filtered`]): the resources by their own names when what is typed
    /// or the groups' lists changed, the placed objects of the areas read
    /// since, and each group's rows when either did or the rows `kept`
    /// under the Filter are others.
    fn find(
        &mut self,
        filter: &Needle,
        shape: Shape,
        lists: &[GroupList],
        contents: &HashMap<ResRef, (Read, AreaContents)>,
        kept: Option<&[ResKey]>,
    ) {
        let f = &mut self.filtered;
        let typed = filter.as_str();
        if f.made_for.as_ref().is_none_or(|(t, s)| t != typed || *s != shape) {
            if f.made_for.as_ref().is_none_or(|(t, _)| t != typed) {
                f.areas.clear();
            }
            // (Every word in its ResRef or its name, in any order.)
            let mut matched = std::collections::HashSet::new();
            for (all, names) in lists {
                for k in all {
                    let name = names.get(k).map(|n| mg_core::text::fold(n)).unwrap_or_default();
                    if filter.finds_in(&[&k.to_string().to_ascii_lowercase(), &name]) {
                        matched.insert(*k);
                    }
                }
            }
            f.matched = Arc::new(matched);
            f.made_for = Some((typed.to_string(), shape));
            f.found_for = None;
        }
        let now = (self.reads, contents.len(), kept.map(<[ResKey]>::to_vec));
        if f.found_for.as_ref() == Some(&now) {
            return;
        }
        f.areas.retain(|area, _| contents.contains_key(area));
        for (area, (_, c)) in contents {
            f.areas.entry(*area).or_insert_with(|| found_in(c, filter).map(Arc::new));
        }
        // (An area stays for what is placed in it, too, opened out or not;
        // and the rows of a tab chosen while the Filter has this typed.)
        let stays = |k: &ResKey| {
            f.matched.contains(k)
                || (k.restype == ResType::ARE
                    && f.areas.get(&k.resref).is_some_and(Option::is_some))
                || kept.is_some_and(|keys| keys.contains(k))
        };
        let found = (lists.iter())
            .map(|(all, _)| all.iter().filter(|k| stays(k)).copied().collect())
            .collect();
        f.found = Arc::new(found);
        f.found_for = Some(now);
    }

    /// An area's placed objects were read (again): what the Filter found
    /// in it is looked for again.
    fn read_again(&mut self, area: ResRef) {
        self.filtered.areas.remove(&area);
        self.reads += 1;
    }

    pub(crate) fn info(
        &mut self,
        ws: &Workspace,
        game: Option<&GameData>,
        area: ResRef,
    ) -> AreaInfo {
        let key = ResKey::new(area, ResType::ARE);
        let listed = self.listed(ws, area);
        let read =
            |are: &Gff| mg_module::areas::info_of(area, &are.root, &|n| shown(game, n), listed);
        if let Some(are) = ws.loaded(&key) {
            return read(are);
        }
        let Some(data) = ws.module.get(&key) else {
            return AreaInfo { resref: area, listed, ..Default::default() };
        };
        let stamp = stored_stamp(ws, &key, data);
        if let Some((s, info)) = self.read.get(&area)
            && *s == stamp
            && info.listed == listed
        {
            return info.clone();
        }
        let info = Gff::read(data).map(|g| read(&g)).unwrap_or_default();
        let info = AreaInfo { resref: area, listed, ..info };
        self.read.insert(area, (stamp, info.clone()));
        info
    }

    /// Whether the module's area list names `area` (read again once the
    /// `module.ifo` is another; true for a module without one).
    fn listed(&mut self, ws: &Workspace, area: ResRef) -> bool {
        let key = mg_module::Module::info_key();
        let read = match (ws.loaded(&key), ws.module.get(&key)) {
            (Some(_), _) => ListRead::Open(ws.revision()),
            (None, Some(d)) => ListRead::Stored(stored_stamp(ws, &key, d)),
            (None, None) => return true,
        };
        if self.listed.as_ref().is_none_or(|(r, _)| *r != read) {
            let names = match ws.loaded(&key) {
                Some(info) => mg_module::areas_of(&info.root),
                None => ws.module.areas().unwrap_or_default(),
            };
            self.listed = Some((read, names.into_iter().collect()));
        }
        self.listed.as_ref().is_some_and(|(_, l)| l.contains(&area))
    }

    /// A blueprint's name (empty when it has none), read once as the
    /// areas' are.
    pub(crate) fn blueprint(
        &mut self,
        ws: &Workspace,
        game: Option<&GameData>,
        key: ResKey,
    ) -> String {
        if let Some(doc) = ws.loaded(&key) {
            return blueprint_name(game, &doc.root);
        }
        let Some(data) = ws.module.get(&key) else { return String::new() };
        let stamp = stored_stamp(ws, &key, data);
        if let Some((s, name)) = self.names.get(&key)
            && *s == stamp
        {
            return name.clone();
        }
        let name = Gff::read(data).map(|g| blueprint_name(game, &g.root)).unwrap_or_default();
        self.names.insert(key, (stamp, name.clone()));
        name
    }

    /// The area's name in the editing language, else as the game shows it
    /// (its talk-table string); empty when it has none.
    pub(crate) fn name(&mut self, ws: &Workspace, game: Option<&GameData>, area: ResRef) -> String {
        self.info(ws, game, area).name
    }

    /// What the area is called in lists and tab titles: its name when
    /// `by_name` and it has one, else its ResRef.
    pub(crate) fn label(
        &mut self,
        ws: &Workspace,
        game: Option<&GameData>,
        area: ResRef,
        by_name: bool,
    ) -> String {
        let name = if by_name { self.name(ws, game, area) } else { String::new() };
        if name.trim().is_empty() { area.to_string() } else { name }
    }
}

/// What is placed in an area, for the module tree and the Outliner: each
/// kind of object and its objects.
pub(crate) type AreaContents = Vec<KindContents>;

/// The objects of one kind placed in an area, for the module tree and the
/// Outliner.
pub(crate) struct KindContents {
    pub(crate) kind: mg_area::ObjectKind,
    /// Each object's row (by its place in the area's list).
    rows: Vec<ObjectRow>,
    /// The places in the area's list, in the order listed: by the rows'
    /// text, as Aurora sorts them (those alike in the area's order).
    order: Order,
}

/// Places in an area's list, in the order listed: shared by the tree,
/// what the Filter finds and the keys, not copied for each every frame.
pub(crate) type Order = Arc<[usize]>;

impl KindContents {
    /// How many of its kind are placed.
    pub(crate) fn count(&self) -> usize {
        self.rows.len()
    }
}

/// How many of each kind `area` holds as last read (`cache`, the module
/// tree's and the Outliner's copy), by [`mg_area::ObjectKind::index`].
pub(crate) fn counts_of(
    cache: &HashMap<ResRef, (Read, AreaContents)>,
    area: ResRef,
) -> Option<[usize; 9]> {
    let (_, contents) = cache.get(&area)?;
    let mut counts = [0; 9];
    for k in contents {
        counts[k.kind.index()] = k.count();
    }
    Some(counts)
}

/// The placed object a Shift+click chooses from (the module tree's and
/// the Outliner's), with its area and how many of each kind that area
/// held then: it holds only in that area, and only while none came or went
/// (a delete, a paste, an Undo or Redo, an outside change: the places
/// after it shift, and the same place is another object).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Anchor {
    area: ResRef,
    at: Placed,
    counts: Option<[usize; 9]>,
}

impl Anchor {
    /// `at` in `area`, as `cache` holds it now.
    pub(crate) fn new(
        cache: &HashMap<ResRef, (Read, AreaContents)>,
        area: ResRef,
        at: Placed,
    ) -> Anchor {
        Anchor { area, at, counts: counts_of(cache, area) }
    }

    /// Where `anchor` is in `area`, if it holds there still.
    pub(crate) fn in_area(
        anchor: Option<Anchor>,
        cache: &HashMap<ResRef, (Read, AreaContents)>,
        area: ResRef,
    ) -> Option<Placed> {
        let a = anchor?;
        (a.area == area && a.counts == counts_of(cache, area)).then_some(a.at)
    }
}

/// An object's row in the module tree.
struct ObjectRow {
    /// What the row says: its name, or for a waypoint its tag (waypoints
    /// often share a name, and their tags are what scripts and
    /// transitions go by: Aurora lists them by tag); the one, where the
    /// other is empty.
    text: String,
    /// Its name and tag, lowered, for the filter.
    sought: String,
    /// What the pointer over the row shows: the tag of a row that says
    /// the name, the name of one that says the tag.
    tip: String,
    /// Its name and its tag, as they are (trimmed), for the Outliner's
    /// row: the name, and the tag weak after it.
    name: String,
    tag: String,
}

/// A placed object of an area, by its kind and its place in its kind's
/// list.
pub(crate) type Placed = (mg_area::ObjectKind, usize);

/// What an area's placed objects were read from, for the module tree's
/// copy of them: the open document at a revision of the workspace (it is
/// being edited), else the stored GIT's data ([`stored_stamp`]; none: no
/// GIT). They are read again when it changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Read {
    Revision(u64),
    Data(Option<Stamp>),
}

/// What is placed in `area` as the workspace has it now, from `cache` (the
/// module tree's copy, `Moonglow::area_contents`, which the Outliner
/// shares): read again first where it was read from another revision of
/// the area's GIT, or another stored copy ([`Read`]): an edit, an Undo or
/// Redo, a paste or a delete there, a rename of a blueprint does not
/// change what was read of others. Read at once, in the frame it is asked
/// for: the Outliner shows one area, which it lists whole (the module tree
/// reads those it opens out after it is drawn, a few a frame).
pub(crate) fn fresh_contents<'a>(
    cache: &'a mut HashMap<ResRef, (Read, AreaContents)>,
    names: &mut AreaNames,
    ws: &Workspace,
    game: Option<&GameData>,
    area: ResRef,
) -> &'a AreaContents {
    let now = read_of(ws, area);
    if cache.get(&area).is_none_or(|c| c.0 != now) {
        cache.insert(area, (now, area_contents(ws, game, area)));
        names.read_again(area);
    }
    &cache[&area].1
}

/// What an area's placed objects would be read from now ([`Read`]).
fn read_of(ws: &Workspace, area: ResRef) -> Read {
    let git = ResKey::new(area, ResType::GIT);
    // (Open: as it changed last, not as the module did: an edit of
    // another resource leaves its list as it is.)
    if ws.loaded(&git).is_some() {
        return Read::Revision(ws.changed_at(&git));
    }
    Read::Data(ws.module.get(&git).map(|d| stored_stamp(ws, &git, d)))
}

/// What is placed in an area, as the workspace has it now: every kind,
/// its objects by name (a waypoint by tag; either, where the other is
/// empty), sorted. (A GIT not open is read, not opened: the Filter reads
/// every area's.)
fn area_contents(ws: &Workspace, game: Option<&GameData>, area: ResRef) -> AreaContents {
    let key = ResKey::new(area, ResType::GIT);
    let stored;
    let git = match ws.loaded(&key) {
        Some(git) => git,
        None => {
            let Some(Ok(git)) = ws.module.get(&key).map(Gff::read) else { return Vec::new() };
            stored = git;
            &stored
        }
    };
    mg_area::ObjectKind::ALL
        .into_iter()
        .map(|kind| {
            let rows: Vec<ObjectRow> = (git.root.list(kind.list()).unwrap_or(&[]).iter())
                .map(|o| {
                    let name = blueprint_name(game, o).trim().to_string();
                    let tag = crate::text::decode(o.string("Tag").unwrap_or_default());
                    let tag = tag.trim().to_string();
                    let sought = mg_core::text::fold(&format!("{name}\n{tag}"));
                    let by_tag = kind == mg_area::ObjectKind::Waypoint;
                    let both = (name.clone(), tag.clone());
                    let (text, other) = if by_tag { (tag, name) } else { (name, tag) };
                    let (text, tip) = match (text.is_empty(), other.is_empty()) {
                        (true, _) => (other, String::new()),
                        (false, true) => (text, String::new()),
                        (false, false) => {
                            let tip = if by_tag {
                                format!("Name: {other}")
                            } else {
                                format!("Tag: {other}")
                            };
                            (text, tip)
                        }
                    };
                    let (name, tag) = both;
                    ObjectRow { text, sought, tip, name, tag }
                })
                .collect();
            let mut order: Vec<usize> = (0..rows.len()).collect();
            // (Case and accents aside: `Árbol` beside `arbol`, not after `z`.)
            let keys: Vec<_> = rows.iter().map(|r| mg_core::text::sort_key(&r.text)).collect();
            // (Stable: alike rows keep the area's order.)
            order.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
            KindContents { kind, rows, order: order.into() }
        })
        .collect()
}

/// What was asked of an object in an area's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Asked {
    /// Shown in the area's view.
    Go,
    /// Selected in the area's view, the camera left where it is (a click
    /// on the Outliner's row: its area is the one in front).
    Select,
    Properties,
    /// Its variables, in the Variables window (as Aurora's list offers).
    Variables,
    /// In the model viewer, as it is placed.
    Preview,
    Copy,
    Delete,
    /// Written out as a blueprint of its own, in an archive (Export as
    /// Blueprint…).
    Export,
    /// Selected in the area's view, and Adjust Location opened for it (or
    /// them).
    Adjust,
    /// A plugin's command run on it: its place among the plugins'
    /// commands for menus.
    Plugin(usize),
}

/// Whether a placed object of `kind` has a picture for the model viewer.
pub(crate) fn previewable(kind: mg_area::ObjectKind) -> bool {
    use mg_area::ObjectKind as K;
    matches!(kind, K::Creature | K::Door | K::Item | K::Placeable)
}

/// An area's placed objects under its row of the module tree, kind by
/// kind as Aurora lists them. A click goes to the object in the area's
/// view; a double click opens its Properties; a right click has both, and
/// Adjust Location…, Copy, Delete, Export as Blueprint… and the plugins' commands for its
/// kind. Ctrl and Shift with a click choose several (`chosen`, marked),
/// which a right click on one of them acts on together. With a filter
/// that some of them match (lower case), only those are listed. Only the
/// rows in view are laid out (an area can hold thousands of a kind).
#[allow(clippy::too_many_arguments)]
pub(crate) fn contents_ui(
    ui: &mut Ui,
    area: ResRef,
    contents: &AreaContents,
    // What the Filter finds among them ([`found_in`]), if anything.
    found: Option<&[Order]>,
    go: &mut Option<(ResRef, mg_area::ObjectKind, usize, Asked)>,
    // The object in hand (its row marked), and whether to go to it.
    in_hand: Option<(mg_area::ObjectKind, usize, bool)>,
    // The objects chosen together, when there are several.
    chosen: Option<&BTreeSet<Placed>>,
    keys: &mut ContentKeys,
    // The plugins' commands for right-click menus.
    commands: &[crate::plugins::PluginCommand],
    // The module tree's rows, or the Outliner's.
    look: Look,
) -> bool {
    let outliner = matches!(look, Look::Outliner { .. });
    let hide_empty = matches!(look, Look::Tree { hide_empty: true });
    let (shown_kinds, fold) = match look {
        Look::Outliner { shown, fold } => (shown, fold),
        Look::Tree { .. } => ([true; 9], None),
    };
    // (Of one kind: Properties opens them together.)
    let one_kind =
        chosen.is_some_and(|c| c.iter().all(|(k, _)| Some(k) == c.first().map(|f| &f.0)));
    let mut went = false;
    let filtered = found.is_some();
    // (Collapse All closed its kinds too, as they are next shown; the
    // Outliner's own, theirs.)
    let closing = fold == Some(false)
        || !outliner
            && ui.data_mut(|d| {
                d.remove_temp::<bool>(egui::Id::new(("tree-area-kinds-closed", area))).is_some()
            });
    let mut body = |ui: &mut Ui| {
        for (at, KindContents { kind, rows, order }) in contents.iter().enumerate() {
            // (By their places in the area's list, in the order listed:
            // borrowed, not copied every frame for each kind.)
            let shown: Order = match found {
                Some(found) => found.get(at).cloned().unwrap_or_else(|| Arc::from([])),
                None => order.clone(),
            };
            let count = if filtered {
                format!("{} of {}", shown.len(), rows.len())
            } else {
                rows.len().to_string()
            };
            // (Named with its count, as its row shows them: "Creatures
            // (46)", "Creatures (3 of 46)"; the Outliner's "46 creatures",
            // apart from the tree's beside it.)
            let title = match outliner {
                true => kind_count(*kind, rows.len()),
                false => format!("{} ({count})", kind.label()),
            };
            let icon = crate::icons::object(*kind);
            // (The Outliner's eye: what the area's view shows of the kind.)
            let seen = shown_kinds[kind.index()];
            let (eye_name, eye_tip) = eye_words(*kind);
            if shown.is_empty() && outliner {
                // (A row with its count and its eye, without an arrow:
                // nothing to open.)
                let row = mg_uikit::TreeRow::new(kind.label()).icon(icon).count(&count);
                let row = row.label(&title).quiet(!seen).sense(egui::Sense::hover());
                let row = row.eye(seen, eye_name, eye_tip).show_parts(ui);
                if row.eye.is_some_and(|e| e.clicked()) {
                    keys.eye = Some(*kind);
                }
                continue;
            }
            if shown.is_empty() {
                if !filtered && !hide_empty {
                    // (A row, quiet, as the kinds' with something in them
                    // without their arrow.)
                    let row = mg_uikit::TreeRow::new(kind.label()).icon(icon).count(&count);
                    row.label(&title).dim().show(ui);
                }
                continue;
            }
            let at_kind = keys.cursor == Some(TreeAt::Kind(area, *kind));
            // (Each its own open state: the tree's and the Outliner's.)
            let salt = if outliner { "outliner-kind" } else { "area-kind" };
            let mut header = mg_uikit::TreeGroup::new((salt, area, *kind), kind.label())
                .icon(icon)
                .count(&count)
                .label(&title)
                // (The keys' cursor on a kind shows as its row marked.)
                .marked(at_kind);
            if outliner {
                header = header.quiet(!seen).eye(seen, eye_name, eye_tip);
            }
            // The keys' cursor at one of its objects, by its place among
            // those listed.
            let at_object = match keys.cursor {
                Some(TreeAt::Object(a, k, i)) if a == area && k == *kind => Some(i),
                _ => None,
            };
            let sought =
                at_object.filter(|_| keys.moved).and_then(|i| shown.iter().position(|&s| s == i));
            // (What is looked for shows without opening each kind; so does
            // the object to go to.)
            let marked = in_hand.filter(|(k, ..)| k == kind);
            let goes = marked.filter(|m| m.2).and_then(|m| shown.iter().position(|&i| i == m.1));
            // (One the filter leaves out is not gone to.)
            if marked.is_some_and(|m| m.2) && goes.is_none() {
                went = true;
            }
            // (A kind the filter finds in is opened once for each change
            // of what is typed, and can be closed after: it was held open,
            // a click on its arrow undone the next frame, a builder's
            // report.)
            let opened_for = egui::Id::new(("tree-kind-filter", area, *kind));
            let newly_found = filtered
                && ui.data_mut(|d| {
                    let now = d.get_temp::<u64>(egui::Id::new("tree-filter-changes"));
                    let was = d.get_temp::<Option<u64>>(opened_for);
                    d.insert_temp(opened_for, now);
                    was != Some(now)
                });
            if newly_found || goes.is_some() {
                header = header.open(Some(true));
            } else if let Some((_, open)) = keys.fold.filter(|f| f.0 == TreeAt::Kind(area, *kind)) {
                header = header.open(Some(open));
            } else if closing {
                header = header.open(Some(false));
            } else if fold == Some(true) {
                header = header.open(Some(true));
            }
            let (clicked, hovered) = (&mut keys.clicked, &mut keys.hovered);
            let (picked, together) = (&mut keys.picked, &mut keys.together);
            let (shown_header, eye) = header.show_eyed(ui, |ui| {
                // The rows in view, and room for those above and below
                // (the kit's rows: `row_height` high).
                let row = mg_uikit::row_height(ui);
                let step = mg_uikit::row_pitch(ui);
                let top = ui.cursor().top();
                // The row to go to, in view or not (rows out of view are
                // not laid out: by where it would be). The tree scrolls
                // only as far as it shows it: a row in view stays where it
                // is (under the pointer, for the second click of a double
                // click).
                if let Some(at) = goes {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(ui.cursor().left(), top + at as f32 * step),
                        egui::vec2(1.0, row),
                    );
                    reveal_into_view(ui, rect);
                    went = true;
                }
                // (And the keys' row, as near as it need be.)
                if let Some(at) = sought {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(ui.cursor().left(), top + at as f32 * step),
                        egui::vec2(1.0, row),
                    );
                    bring_into_view(ui, rect);
                }
                let clip = ui.clip_rect();
                let first =
                    (((clip.top() - top) / step).floor().max(0.0) as usize).min(shown.len());
                let last = (((clip.bottom() - top) / step).ceil().max(0.0) as usize)
                    .clamp(first, shown.len());
                let room = |ui: &mut Ui, rows: usize| {
                    if rows > 0 {
                        let tall = rows as f32 * step - spacing_gap(ui);
                        ui.allocate_space(egui::vec2(1.0, tall.max(0.0)));
                    }
                };
                room(ui, first);
                for &index in &shown[first..last] {
                    let object = &rows[index];
                    let text = if object.text.is_empty() { NO_NAME } else { object.text.as_str() };
                    let several = chosen.is_some_and(|c| c.contains(&(*kind, index)));
                    let is_marked =
                        marked.is_some_and(|m| m.1 == index) || at_object == Some(index) || several;
                    let r = if outliner {
                        // (Its name, and its tag weak after it, cut to the
                        // panel's width: `LightArea_crop_outl.png`.)
                        let name = if object.name.is_empty() { NO_NAME } else { &object.name };
                        let tip = "Click to select it in the area, double-click for its \
                                   Properties. Right-click for more.";
                        let label = object_label(&object.name, &object.tag);
                        let row = mg_uikit::TreeRow::new(name).second(&object.tag).label(label);
                        row.selected(is_marked).show(ui).on_hover_text(tip)
                    } else {
                        let how = "Click to go to it in the area, double-click for its \
                                   Properties. Right-click for more.";
                        let tip = if object.tip.is_empty() {
                            how.to_string()
                        } else {
                            format!("{}\n{how}", object.tip)
                        };
                        // (The kit's row, its fill across the whole row.)
                        let r = mg_uikit::TreeRow::new(text).selected(is_marked).show(ui);
                        r.on_hover_text(tip)
                    };
                    let m = ui.input(|i| i.modifiers);
                    let (command, shift) = (m.command || m.ctrl, m.shift);
                    if r.clicked() && (command || shift) {
                        // Chosen with the others (after the rows are laid
                        // out: `tree_rows`).
                        *picked = Some((TreeAt::Object(area, *kind, index), command, shift));
                    } else if r.double_clicked() {
                        *go = Some((area, *kind, index, Asked::Properties));
                    } else if r.clicked() {
                        // (The Outliner's area is the one in front: its
                        // click selects there, the camera where it is.)
                        let asked = if outliner { Asked::Select } else { Asked::Go };
                        *go = Some((area, *kind, index, asked));
                    }
                    if r.clicked() {
                        *clicked = Some(TreeAt::Object(area, *kind, index));
                    }
                    if r.hovered() {
                        *hovered = Some(TreeAt::Object(area, *kind, index));
                    }
                    mg_uikit::context_menu(&r, |ui| {
                        // Several chosen, this one among them: what is
                        // done to them together.
                        if several {
                            let n = chosen.map_or(0, BTreeSet::len);
                            mg_uikit::menu_note(ui, format!("{n} objects chosen"));
                            mg_uikit::menu_separator(ui);
                        }
                        // Variables and Properties last, as in Aurora;
                        // Adjust Location… after Go To, as the area
                        // view's menu has it near the top.
                        for (label, asked) in [
                            ("Go To", Asked::Go),
                            ("Adjust Location…", Asked::Adjust),
                            ("Preview", Asked::Preview),
                            ("Copy", Asked::Copy),
                            ("Delete", Asked::Delete),
                            ("Export as Blueprint…", Asked::Export),
                            ("Variables…", Asked::Variables),
                            ("Properties", Asked::Properties),
                        ] {
                            if asked == Asked::Preview && (several || !previewable(*kind)) {
                                continue;
                            }
                            if asked == Asked::Variables {
                                // (The plugins' commands for it, or for
                                // the kinds of those chosen, before.)
                                let mut kinds = vec![mg_plugin::Target::Placed(*kind)];
                                if several {
                                    let all = chosen.into_iter().flatten();
                                    kinds =
                                        all.map(|(k, _)| mg_plugin::Target::Placed(*k)).collect();
                                    kinds.sort();
                                    kinds.dedup();
                                }
                                if let Some(i) = crate::plugins::menu_commands(ui, commands, &kinds)
                                {
                                    *go = Some((area, *kind, index, Asked::Plugin(i)));
                                    *together = several;
                                }
                                mg_uikit::menu_separator(ui);
                            }
                            // (One at a time: its variables, and a
                            // blueprint of it; Properties of several of one
                            // kind.)
                            let one = matches!(asked, Asked::Export | Asked::Variables);
                            if several && (one || (asked == Asked::Properties && !one_kind)) {
                                continue;
                            }
                            let mut b = ui.button(label);
                            if asked == Asked::Export {
                                b = b.on_hover_text(
                                    "Write it into an .erf as a blueprint, with its items, for \
                                     another module",
                                );
                            }
                            if b.clicked() {
                                *go = Some((area, *kind, index, asked));
                                *together = several;
                                ui.close();
                            }
                        }
                    });
                }
                room(ui, shown.len() - last);
            });
            if eye.is_some_and(|e| e.clicked()) {
                keys.eye = Some(*kind);
            }
            mg_uikit::say_expanded(
                &shown_header.header_response,
                shown_header.body_returned.is_some(),
            );
            if at_kind && keys.moved {
                bring_into_view(ui, shown_header.header_response.rect);
            }
            if shown_header.header_response.clicked() {
                keys.clicked = Some(TreeAt::Kind(area, *kind));
            }
            keys.listed.push((*kind, shown_header.openness > 0.5, shown));
        }
    };
    // (The tree's kinds indented under their area's row; the Outliner's
    // at its left, as the board has them.)
    if outliner {
        body(ui);
    } else {
        ui.indent(("area-contents", area), body);
    }
    went
}

/// Whose rows [`contents_ui`] lays out, and how.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Look {
    /// The module tree's, under an area's row: each kind named with its
    /// count ("Creatures (46)"), a kind with nothing placed quiet, or left
    /// out where `hide_empty` (Options › General); an object's row its
    /// name (a waypoint's tag), a click going to it.
    Tree { hide_empty: bool },
    /// The Outliner's (`outliner.rs`), the area's alone: each kind with an
    /// eye at its right (`shown`: what the area's view shows, by
    /// [`mg_area::ObjectKind::index`]), quiet where hidden; a kind with
    /// nothing placed a row with its count and eye, without an arrow; an
    /// object's row its name and its tag weak after it, cut to the
    /// panel's width, a click selecting it. Every kind opened (`fold`
    /// `Some(true)`, Expand All) or closed (Collapse All) this frame.
    Outliner { shown: [bool; 9], fold: Option<bool> },
}

/// A kind's count as the Outliner's row is named: "46 creatures", "1
/// creature", "0 items" (the module tree's beside it are "Creatures
/// (46)").
pub(crate) fn kind_count(kind: mg_area::ObjectKind, n: usize) -> String {
    match n {
        1 => format!("1 {}", kind.name().to_lowercase()),
        n => format!("{n} {}", kind.label().to_lowercase()),
    }
}

/// An Outliner's eye's name and tip for `kind`: named as the area
/// toolbar's toggle of the same state is, "Show Creatures" (title case: a
/// toggle names what it does when on, and is said pressed or not; a name
/// that changed with its state, as the board's "Hide creatures", would
/// leave a toggle that no longer answers to its name), the Outliner's
/// group apart from the toolbar's.
pub(crate) fn eye_words(kind: mg_area::ObjectKind) -> (String, String) {
    let name = format!("Show {}", kind.label());
    let tip = format!("Show or hide the {} in the area's view", kind.label().to_lowercase());
    (name, tip)
}

/// An Outliner's object row's accessible name: its name and its tag
/// ("Corpse, tag Corpse - 2"; "Corpse, no tag"; "(no name), tag X"),
/// apart from the module tree's row of the same object beside it, which
/// is named by its name alone.
pub(crate) fn object_label(name: &str, tag: &str) -> String {
    let name = if name.is_empty() { NO_NAME } else { name };
    if tag.is_empty() { format!("{name}, no tag") } else { format!("{name}, tag {tag}") }
}

/// The keys' part in an area's contents: where their cursor is, whether
/// it just moved, a kind they open or close; and back, the row clicked,
/// the object's row under the pointer and what was listed.
#[derive(Default)]
pub(crate) struct ContentKeys {
    pub(crate) cursor: Option<TreeAt>,
    pub(crate) moved: bool,
    pub(crate) fold: Option<(TreeAt, bool)>,
    pub(crate) clicked: Option<TreeAt>,
    pub(crate) hovered: Option<TreeAt>,
    /// A row clicked with Ctrl (`.1`) or Shift (`.2`).
    pub(crate) picked: Option<(TreeAt, bool, bool)>,
    /// What was asked from the menu of a row among several chosen: for
    /// them all.
    pub(crate) together: bool,
    pub(crate) listed: Vec<(mg_area::ObjectKind, bool, Order)>,
    /// The kind whose eye was clicked (the Outliner's).
    pub(crate) eye: Option<mg_area::ObjectKind>,
}

/// The gap between rows (taken off a block of rows left out, which is
/// followed by one itself).
fn spacing_gap(ui: &Ui) -> f32 {
    ui.spacing().item_spacing.y
}

/// Whether the keys typed are the tree's to read: not while a field has
/// the keyboard, nor while a dialog is open over the tree (the Rename
/// window F2 opens, Delete's question): what is typed there is the
/// dialog's alone, its Escape and Enter too, and the tree is as it was
/// once it closes (its cursor at the same row, the keys still its own).
pub(crate) fn keys_free(ui: &Ui) -> bool {
    !ui.ctx().egui_wants_keyboard_input() && !crate::widgets::dialog_open(ui.ctx())
}

/// Brings a row (`rect`) into view up or down only: the tree scrolled
/// sideways stays so (egui's own goes to a long row's start).
/// (In the Areas group's well, [`areas_well`] scrolls the well to it
/// instead: [`well_target`].)
fn bring_into_view(ui: &Ui, rect: egui::Rect) {
    if well_target(ui, rect, false) {
        return;
    }
    let x = ui.clip_rect().center().x;
    ui.scroll_to_rect(egui::Rect::from_x_y_ranges(x..=x, rect.y_range()), None);
}

/// Brings a row gone to from elsewhere (a tab chosen, an object selected
/// in the area) into view: one out of sight to the middle of the pane,
/// rather than its foot or top (where the tree scrolled only as far as it
/// must, a row brought up from below sat at the foot, the rows after it
/// out of sight); one in sight stays where it is (under the pointer, for
/// a double click's second click). (In the Areas group's well, as
/// [`bring_into_view`].)
fn reveal_into_view(ui: &Ui, rect: egui::Rect) {
    let clip = ui.clip_rect();
    if clip.y_range().contains(rect.top()) && clip.y_range().contains(rect.bottom()) {
        return;
    }
    if well_target(ui, rect, true) {
        return;
    }
    let x = clip.center().x;
    let to = egui::Rect::from_x_y_ranges(x..=x, rect.y_range());
    ui.scroll_to_rect(to, Some(egui::Align::Center));
}

/// Where the Areas group's well is drawn, while its rows are being laid
/// out ([`areas_well`]): its view on the screen (as high as it is at
/// most).
fn well_view() -> egui::Id {
    egui::Id::new("tree-well-view")
}

/// A row of the Areas group's well to scroll to, once its rows are laid
/// out: where it is, and whether to the middle of the well.
#[derive(Clone, Copy, Default)]
struct WellTarget {
    /// (`None`: a row in the well's view already, the well alone to
    /// bring into the tree's.)
    rows: Option<egui::Rangef>,
    center: bool,
}

/// While the Areas group's well is being laid out, the row at `rect` to be
/// scrolled to in it (to the middle where `center`), for [`areas_well`];
/// whether it was. A scroll area works out what is scrolled to inside it
/// by what it shows of itself, which, with the well partly out of the
/// tree's view (the tree scrolled down past it), was the part in sight,
/// or nothing, and the row went out of sight; and it takes what is
/// scrolled to inside it for itself, so the tree around it would not
/// bring the well into view. (A row in the well's view, the well partly
/// out of the tree's, is scrolled to no further in it.)
fn well_target(ui: &Ui, rect: egui::Rect, center: bool) -> bool {
    let Some(view) = ui.data(|d| d.get_temp::<Option<egui::Rect>>(well_view())).flatten() else {
        return false;
    };
    let seen = view.y_range().contains(rect.top()) && view.y_range().contains(rect.bottom());
    let center = center && !seen;
    let rows = (!seen).then(|| rect.y_range());
    ui.data_mut(|d| d.insert_temp(egui::Id::new("tree-well-target"), WellTarget { rows, center }));
    true
}

/// The Module panel's name: the group its body is in (its Filter and the
/// tree), apart from the Left Dock's tab bar (whose Module Properties is a
/// button, the tree's a row) and from a dropdown's Filter.
pub(crate) const MODULE_PANEL: &str = "Module Panel";
/// The module tree's Filter's name: it says "Filter" while empty, as a
/// dropdown's does.
pub(crate) const TREE_FILTER: &str = "Filter Module Tree";
/// The module tree's name: the group its rows are in, in the Module
/// panel's.
pub(crate) const MODULE_TREE: &str = "Module Tree Rows";

/// The least number of rows the Areas group's well shows while it holds
/// more (the Main board's five).
const WELL_LEAST_ROWS: f32 = 5.0;

/// The Module panel's body (the Left Dock's Module tab, under its tab bar,
/// whose Expand All and Collapse All give `fold`): the Filter, which stays
/// in sight, over the tree, which scrolls (Home and End go to its top and
/// bottom while the pointer is over it, the Areas group's while it is
/// over that); the Areas group scrolls in a well of its own
/// ([`areas_well`]).
pub(crate) fn module_tree(app: &mut Moonglow, ui: &mut Ui, fold: Option<bool>) {
    if app.ws.is_none() {
        return;
    }
    mg_uikit::name_group(ui, MODULE_PANEL);
    // (The pane clicked in last, whose selection a plugin's command from
    // the Plugins menu or a key runs on.)
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        app.plugins.pane = Some(crate::plugins::Pane::Tree);
    }
    let filter_id = egui::Id::new("tree-filter");
    let mut filter = app.buffers.get(&filter_id).cloned().unwrap_or_default();
    // As the palettes' pane: the search across the pane, with a button to
    // clear it. Find's keys (Ctrl+F), with the keys the tree's, go to it;
    // Escape there clears what is typed; again, back to the rows, and Down
    // to the first row it finds.
    // (The button that folds the dock away is on its tab bar: GitHub
    // issue 10, `shell::left_dock`.)
    let here = crate::palette_view::keys_are(ui.ctx(), crate::palette_view::Keys::Tree)
        && app.tree_cursor.is_some()
        && !crate::widgets::dialog_open(ui.ctx());
    // (The mock's `.panel-bar`: 4 px above and below, 8 at the sides.)
    let bar = egui::Margin { left: 8, right: 8, top: 4, bottom: 4 };
    let left = egui::Frame::new()
        .inner_margin(bar)
        .show(ui, |ui| {
            crate::widgets::SearchField::new(egui::Id::new("tree-filter-field"), &mut filter)
                .icon(mg_uikit::Icon::Filter)
                .hint("Filter")
                .name(TREE_FILTER)
                .clear_tip("Clear the filter")
                .clear_name("Clear Filter")
                .find_when(here)
                .down()
                .show(ui)
                .left
        })
        .inner;
    if let Some(left) = left {
        if left == crate::widgets::FilterLeft::Down {
            app.tree_to_match = true;
        } else if app.tree_cursor.is_none() {
            app.tree_cursor = Some(first_row());
        }
        app.tree_cursor_moved = true;
        crate::palette_view::give_keys(ui.ctx(), crate::palette_view::Keys::Tree);
    }
    app.buffers.insert(filter_id, filter.clone());
    // (Every word, case and accents aside: `arbol` finds `Árbol`.)
    let filter = Needle::new(&filter);
    // Every group opened or closed at once (this frame). A filter typed
    // opens every group with a match, as it is typed: they can be closed
    // again after.
    let changed = crate::widgets::text_changed(ui, filter_id, filter.as_str());
    let typed = changed && !filter.is_empty();
    // (Each change of the Filter counted, for the kinds of objects it
    // opens once: `contents_ui`.)
    if changed {
        ui.data_mut(|d| *d.get_temp_mut_or(egui::Id::new("tree-filter-changes"), 0_u64) += 1);
    }
    // Expand All and Collapse All (on the tab bar), and their keys too,
    // while the keys are the tree's.
    let mut fold = fold;
    let keyed =
        crate::palette_view::keys_are(ui.ctx(), crate::palette_view::Keys::Tree) && keys_free(ui);
    if keyed {
        fold = fold.or_else(|| crate::palette_view::fold_keys(ui, &app.keymap));
    }
    // Collapse All closes every area opened out to its objects too, and
    // the kinds under it (Expand All leaves them: an area's objects can be
    // hundreds of rows).
    if fold == Some(false)
        && let Some(ws) = app.ws.as_ref()
    {
        for area in ws.module.keys_of(ResType::ARE).map(|k| k.resref) {
            ui.data_mut(|d| {
                // (Those the Filter opened out too, until it changes.)
                let now = d.get_temp::<u64>(egui::Id::new("tree-filter-changes"));
                d.insert_temp(egui::Id::new(("tree-area-filter-closed", area)), now);
                d.insert_temp(egui::Id::new(("tree-area-open", area)), false);
                d.insert_temp(egui::Id::new(("tree-area-kinds-closed", area)), true);
            });
        }
    }
    let fold = fold.or(typed.then_some(true));
    // (The tree's whole height, for the Areas group's well: a scroll bar
    // across its foot, there or not, leaves the well's rows as they are.)
    let view = ui.available_height();
    // (Where the Areas group's well is, known for the frame it is drawn
    // in: `areas_well`.)
    let well_rect = egui::Id::new("tree-well-rect");
    ui.data_mut(|d| d.remove::<egui::Rect>(well_rect));
    // Where the tree was scrolled to before something was typed in the
    // Filter, back once it is cleared (a word that finds little made the
    // tree short, and a scroll area keeps no more than it can show).
    let unfiltered = egui::Id::new("tree-unfiltered-offset");
    // (egui's default id, under which eframe has kept the tree's place
    // from run to run; the Resources panel's list, drawn in the same place
    // of the Left Dock, has one of its own: `browser::ui`.)
    let mut area = egui::ScrollArea::both().auto_shrink(false);
    if changed
        && filter.is_empty()
        && let Some(y) = ui.data(|d| d.get_temp::<f32>(unfiltered))
    {
        area = area.vertical_scroll_offset(y);
    }
    let mut rows = area.show(ui, |ui| {
        mg_uikit::name_group(ui, MODULE_TREE);
        mg_uikit::list_rows(ui);
        // A name too long for the pane goes on past it, and the tree
        // scrolls sideways to it, as Aurora's does (a builder's report:
        // only widening the pane showed long names whole).
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        let left = ui.min_rect().left();
        tree_rows(app, ui, &filter, fold, view);
        // (Rows out of sight are not laid out: the widest drawn since the
        // filter changed keeps the width scrolled through, which then
        // doesn't change as rows go by.)
        let widest = egui::Id::new(("tree-widest", filter.as_str()));
        let drawn = ui.min_rect().right() - left;
        let width = ui.data_mut(|d| {
            let w = d.get_temp_mut_or(widest, 0.0_f32);
            *w = w.max(drawn);
            *w
        });
        ui.expand_to_include_x(left + width);
    });
    if filter.is_empty() {
        ui.data_mut(|d| d.insert_temp(unfiltered, rows.state.offset.y));
    }
    // (Over the Areas group's well, its own: `areas_well`.)
    let well = ui.data(|d| d.get_temp::<egui::Rect>(well_rect));
    let well = well.map(|w| w.intersect(rows.inner_rect));
    if !well.is_some_and(|w| w.is_positive() && ui.rect_contains_pointer(w)) {
        crate::widgets::home_and_end(ui, &mut rows);
    }
    crate::palette_view::keys_outline(ui, rows.inner_rect, crate::palette_view::Keys::Tree);
}

/// `add` in the Areas group's well ([`areas_well`]) where `well`, else as
/// it is.
fn in_well<R>(
    ui: &mut Ui,
    well: bool,
    (view, others, filtered): (f32, usize, bool),
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    if well { areas_well(ui, view, others, filtered, add) } else { add(ui) }
}

/// A group's icon: its kind's (an area's, a conversation's, a blueprint
/// kind's, the journal's for the journal and factions).
fn group_icon(types: &[ResType]) -> mg_uikit::Icon {
    use mg_uikit::Icon;
    match types.first().copied() {
        Some(ResType::ARE) => Icon::Area,
        Some(ResType::DLG) => Icon::Conversation,
        Some(ResType::NSS) => Icon::Script,
        Some(ResType::JUI) => Icon::Nui,
        Some(ResType::JRL) => Icon::Journal,
        Some(t) => mg_area::ObjectKind::from_restype(t).map_or(Icon::File, crate::icons::object),
        None => Icon::File,
    }
}

/// The Areas group's rows (`add`) in a well of their own that scrolls by
/// itself (ruling 10 of 2026-11-10; the mock's `.tree-well`), so that the
/// groups under it stay in sight and in place. The well is as high as the
/// tree's view (`view`: its whole height, a scroll bar across its foot or
/// not) less a row for each of the tree's other rows (`others`: Module
/// Properties, each group's row and the line under them), in whole rows,
/// at least [`WELL_LEAST_ROWS`], and no higher than the rows it holds; as
/// wide as the tree's view, where it stays when the tree is scrolled
/// sideways (it scrolls sideways itself to a long name). It takes its
/// height alone from the tree's layout: the tree scrolls sideways only to
/// its own rows (taken as wide as the view, it kept the widest view it
/// was ever drawn in, `tree-widest`, and scrolled sideways into nothing
/// once the dock was narrowed). While the Filter has something typed
/// (`filtered`) it scrolls under an id of its own, so that the place it
/// was scrolled to is where it was once the Filter is cleared. A row
/// brought into view in it ([`bring_into_view`], [`reveal_into_view`])
/// is scrolled to there, and the well into the tree's view.
fn areas_well<R>(
    ui: &mut Ui,
    view: f32,
    others: usize,
    filtered: bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let row = mg_uikit::row_height(ui);
    let rows = ((view / row).floor() - others as f32).max(WELL_LEAST_ROWS);
    let height = rows * row;
    let clip = ui.clip_rect();
    // (Pinned to the tree's view sideways: the group's rows are indented,
    // the well spans the pane as the board draws it.)
    let at = egui::pos2(clip.left(), ui.cursor().top());
    let room = egui::Rect::from_min_size(at, egui::vec2(clip.width(), height));
    let target_id = egui::Id::new("tree-well-target");
    ui.data_mut(|d| {
        d.insert_temp(well_view(), Some(room));
        d.remove_temp::<WellTarget>(target_id);
    });
    let builder =
        egui::UiBuilder::new().max_rect(room).layout(egui::Layout::top_down(egui::Align::Min));
    let mut well = ui.new_child(builder);
    let mut out = egui::ScrollArea::both()
        .id_salt(("tree-areas", filtered))
        .auto_shrink([false, true])
        .max_height(height)
        .show(&mut well, |ui| {
            // (Where the rows began, for the row to scroll to.)
            let top = ui.max_rect().top();
            (ui.indent("tree-areas-rows", add).inner, top)
        });
    crate::widgets::home_and_end(&well, &mut out);
    let ((inner, top), view) = (out.inner, out.inner_rect);
    ui.data_mut(|d| d.insert_temp(well_view(), None::<egui::Rect>));
    // A row to go to: the well scrolled to it (from the next frame, as a
    // scroll area scrolls), to its middle or only as far as it shows it;
    // and the well into the tree's view.
    let target = ui.data_mut(|d| d.remove_temp::<WellTarget>(target_id));
    if let Some(WellTarget { rows, center }) = target {
        let (now, tall) = (out.state.offset.y, view.height());
        let most = (out.content_size.y - tall).max(0.0);
        let (from, to) = rows.map_or((now, now), |r| (r.min - top, r.max - top));
        let offset = if rows.is_none() {
            now
        } else if center {
            (from + to) / 2.0 - tall / 2.0
        } else if from < now {
            from
        } else if to > now + tall {
            to - tall
        } else {
            now
        };
        let offset = offset.clamp(0.0, most);
        if (offset - now).abs() > 0.1 {
            out.state.offset.y = offset;
            out.state.store(ui.ctx(), out.id);
            ui.ctx().request_repaint();
        }
    }
    let rect = well.min_rect();
    // (Its height taken in the tree, none of its width: `Line::allocate`
    // takes a row's so too.)
    ui.allocate_space(egui::vec2(1.0, rect.height()));
    ui.data_mut(|d| d.insert_temp(egui::Id::new("tree-well-rect"), rect));
    if target.is_some() {
        ui.scroll_to_rect(rect, None);
    }
    inner
}

/// The one object selected in the area in front, if it is another than
/// the tree shows: the tree then goes to it (its kind's list opened),
/// where its area is opened out there. An area left closed in the tree
/// is opened out to it, as in Aurora, unless Options › General says to
/// leave it closed (the tree then does not move). While the Outliner is
/// shown beside the tree (the owner's ruling of 2026-11-14: "while the
/// Outliner shows, the tree stops following the selection"), the
/// Outliner follows instead: the tree marks the object's row (and the
/// keys' row goes to it) but neither opens its area out nor scrolls to
/// it, staying where the builder left it. What the builder does in the
/// tree, and what asks the tree by name (Show in Module Tree, a tab
/// chosen), still moves it.
fn follow_selection(app: &mut Moonglow) {
    // (Shown this frame: the tree is drawn, so the Left Dock is, unfolded,
    // and the Outliner under it where the active editor brings it. The
    // dock's fold read as it is drawn, not as the builder's setting: in a
    // narrow window it is folded by its width with `hide_tree` false, and
    // unfolded by hand there with `hide_tree` true.)
    let outliner = crate::shell::Panel::OUTLINER.available(app) && !crate::shell::left_folded(app);
    let seen = app.palette.area.and_then(|area| {
        let view = app.area_views.get(&area)?;
        Some((area, view.selection.clone()))
    });
    // Several selected there: chosen in the tree too, each row marked
    // (as the selection changes: those chosen in the tree of an area not
    // shown stay).
    if seen != app.tree_seen_selection {
        if let Some((area, selection)) = &seen {
            let several = selection.len() > 1;
            app.tree_objects = several.then(|| (*area, selection.iter().copied().collect()));
        }
        app.tree_seen_selection = seen.clone();
    }
    let selected = seen.and_then(|(area, selection)| match selection.as_slice() {
        [(kind, index)] => Some((area, *kind, *index)),
        _ => None,
    });
    if selected != app.tree_object {
        app.tree_object = selected;
        app.tree_object_pending = selected.is_some() && !outliner;
        // The keys' cursor on an object's row (a row clicked) goes to the
        // one selected now: one row marked, the one selected last, however
        // it was selected (it stayed marked too, a builder's report).
        if matches!(app.tree_cursor, Some(TreeAt::Object(..))) {
            app.tree_cursor = selected.map(|(area, kind, index)| TreeAt::Object(area, kind, index));
        }
        if let Some((area, ..)) =
            selected.filter(|_| !app.settings.tree_keeps_areas_closed && !outliner)
        {
            app.tree_reveal = Some((ResKey::new(area, ResType::ARE), true));
        }
    }
}

/// Whether the module tree lists a resource: the module's, of a kind of
/// its groups.
pub(crate) fn listed(app: &Moonglow, key: &ResKey) -> bool {
    GROUPS.iter().any(|(_, types)| types.contains(&key.restype))
        && app.ws.as_ref().is_some_and(|ws| ws.module.contains(key))
}

/// The module tree goes to what a tab chosen shows (its row marked, its
/// group opened, the row brought into view), however it was chosen: a
/// click on it, the keys, opened. The keys stay with the tab. Where
/// Options › General says the tree stays, only an area's row is marked,
/// as the one in front (Aurora marks it so, and its tree follows nothing
/// else).
pub(crate) fn reveal_tab(app: &mut Moonglow, tab: &Tab) {
    app.tree_went = Some(tab.clone());
    // Module Properties: its row, at the top.
    if *tab == Tab::ModuleProperties {
        if !app.settings.tree_stays {
            app.tree_reveal = Some((module_row(), false));
            app.tree_selected.clear();
        }
        return;
    }
    let keys: Vec<ResKey> = tab.tree_keys().into_iter().filter(|k| listed(app, k)).collect();
    let Some(&key) = keys.first() else { return };
    if !app.settings.tree_stays {
        app.tree_reveal = Some((key, false));
        // A tab of several (Properties of several blueprints or areas):
        // each of them chosen, the first brought into view.
        app.tree_selected = keys.iter().copied().collect();
        app.tree_anchor = Some(key);
    } else if key.restype == ResType::ARE && keys.len() == 1 {
        app.tree_marked = Some(key);
        app.tree_area = Some(key.resref);
    }
}

/// What marks the tree's Module Properties row ([`Moonglow::tree_marked`],
/// [`Moonglow::tree_reveal`]): the module's `module.ifo`, which the tree
/// lists no other way.
pub(crate) fn module_row() -> ResKey {
    ResKey::new(ResRef::from_str("module").expect("a valid ResRef"), ResType::IFO)
}

/// The tab chosen since the tree last looked, which it goes to
/// ([`reveal_tab`]): the one in front of the pane given the focus (a
/// window's, or the middle's), else one come to the front of its pane
/// (opened, or reached by the keys). One come to the front because a tab
/// was closed was not chosen: the tree stays where it is (it went to the
/// top, to Module Properties' row, as an area or a Properties window
/// opened from it closed: a builder's report). Nor was the pane given the
/// focus whose tab the tree went to last: a tab opened comes to the front
/// without the focus, and the first click in it chose nothing new (it
/// set the rows chosen in the tree since, several to delete together,
/// back to the tab's one).
pub(crate) fn follow_front_tab(app: &mut Moonglow) {
    let front = |leaf: &egui_dock::LeafNode<Tab>| leaf.tabs.get(leaf.active.0).cloned();
    let fronts: Vec<Tab> = (app.dock.iter_leaves()).filter_map(|(_, leaf)| front(leaf)).collect();
    let focused = (app.dock.focused_leaf()).and_then(|p| app.dock.leaf(p).ok()).and_then(front);
    let open = app.dock.iter_all_tabs().count();
    let (was_focused, was, were_open) = std::mem::take(&mut app.tree_front);
    let chosen = if open < were_open {
        None
    } else if focused.is_some() && focused != was_focused && focused != app.tree_went {
        focused.clone()
    } else {
        fronts.iter().find(|t| !was.contains(t)).cloned()
    };
    app.tree_front = (focused, fronts, open);
    if let Some(tab) = chosen {
        reveal_tab(app, &tab);
    }
}

/// A row of the module tree the arrow keys can be at: a group's, a
/// resource's, and under an area opened out a kind of object's and a
/// placed object's (by its place in the kind's list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeAt {
    Group(&'static str),
    Resource(ResKey),
    Kind(ResRef, mg_area::ObjectKind),
    Object(ResRef, mg_area::ObjectKind, usize),
}

/// The areas opened out, for the keys: each one's kinds as listed, whether
/// a kind is open, and its objects as listed.
type Outs = HashMap<ResRef, Vec<(mg_area::ObjectKind, bool, Order)>>;

/// A row the keys go through ([`key_rows`]): where it is, how deep, and,
/// if it opens, whether it is open.
pub(crate) type KeyRow = (TreeAt, usize, Option<bool>);

/// What an area holds as listed ([`contents_ui`]: each kind listed,
/// whether it is open, its objects in order), as rows for the keys, the
/// kinds `depth` deep and their objects one deeper: under the module
/// tree's area row, or the Outliner's own.
pub(crate) fn area_key_rows(
    rows: &mut Vec<KeyRow>,
    area: ResRef,
    listed: &[(mg_area::ObjectKind, bool, Order)],
    depth: usize,
) {
    for (kind, kind_open, objects) in listed {
        rows.push((TreeAt::Kind(area, *kind), depth, Some(*kind_open)));
        let inside = objects.iter().filter(|_| *kind_open);
        rows.extend(inside.map(|i| (TreeAt::Object(area, *kind, *i), depth + 1, None)));
    }
}

/// The rows the keys go through, in order: each with how deep it is and,
/// if it opens, whether it is open.
fn key_rows(shown: &Shown<'_>, outs: &Outs) -> Vec<KeyRow> {
    let mut rows = Vec::new();
    for (name, open, keys) in shown {
        rows.push((TreeAt::Group(name), 0, Some(*open)));
        for k in keys.iter().filter(|_| *open) {
            let out = outs.get(&k.resref).filter(|_| k.restype == ResType::ARE);
            let opens = (k.restype == ResType::ARE).then_some(out.is_some());
            rows.push((TreeAt::Resource(*k), 1, opens));
            area_key_rows(&mut rows, k.resref, out.map_or(&[], |o| o.as_slice()), 2);
        }
    }
    rows
}

/// Where the tree's keys take the cursor from `at`, among the rows shown
/// (in order, each group's row before its resources'): the row to go to,
/// a group to open or close, or the resource to open.
#[derive(Debug, PartialEq)]
pub(crate) enum Step {
    To(TreeAt),
    /// Open or close the row (a group, an area, a kind of object).
    Fold(TreeAt, bool),
    Open(ResKey),
    /// Go to a placed object in its area's view.
    Go(ResRef, mg_area::ObjectKind, usize),
    None,
}

/// The tree's first row, where the keys begin.
pub(crate) fn first_row() -> TreeAt {
    TreeAt::Group(GROUPS[0].0)
}

/// The rows shown: each group, whether it is open, and its resources (the
/// group's list as made, borrowed, unless the Filter keeps some: not
/// copied every frame).
type Shown<'a> = Vec<(&'static str, bool, std::borrow::Cow<'a, [ResKey]>)>;

/// The row the letters typed in quick succession (`typed`) go to, as in
/// the conversation tree ([`crate::dialog_view::type_ahead`]): one
/// letter, or the same again and again, the next row after `at` beginning
/// with it; else the first from `at` on (itself first, round the end)
/// whose name begins with them all, case and accents aside. The rows are
/// those in sight, in order ([`key_rows`]): an area opened out has its
/// kinds' rows, and a kind open its placed objects'. A row's name is what
/// it shows (`text`: a group's or a kind's name without its count; a
/// resource's, an area's or a blueprint's name where they are listed by
/// name, else its ResRef; an object's name, or a waypoint's tag).
fn seek(
    shown: &Shown<'_>,
    outs: &Outs,
    at: Option<TreeAt>,
    typed: &str,
    text: &dyn Fn(TreeAt) -> String,
) -> Option<TreeAt> {
    seek_in(&key_rows(shown, outs), at, typed, text)
}

/// [`seek`] among `rows` ([`KeyRow`]s, in order): the module tree's or the
/// Outliner's.
pub(crate) fn seek_in(
    rows: &[KeyRow],
    at: Option<TreeAt>,
    typed: &str,
    text: &dyn Fn(TreeAt) -> String,
) -> Option<TreeAt> {
    let rows: Vec<TreeAt> = rows.iter().map(|r| r.0).collect();
    // (No row: from the first, itself first.)
    let from = at.and_then(|at| rows.iter().position(|r| *r == at));
    let from = from.unwrap_or(rows.len().saturating_sub(1));
    let texts: Vec<String> = rows.iter().map(|r| mg_core::text::fold(&text(*r))).collect();
    let to = crate::dialog_view::type_ahead(&texts, from, &mg_core::text::fold(typed))?;
    Some(rows[to])
}

/// A row's text as the module tree shows it, for the letters typed
/// ([`seek`]): `named` gives a resource's name where it is listed by name,
/// `contents` an area's placed objects as read.
pub(crate) fn row_text(
    at: TreeAt,
    named: &dyn Fn(&ResKey) -> Option<String>,
    contents: &HashMap<ResRef, (Read, AreaContents)>,
) -> String {
    match at {
        TreeAt::Group(name) => name.to_string(),
        TreeAt::Resource(k) => named(&k).unwrap_or_else(|| k.resref.to_string()),
        TreeAt::Kind(_, kind) => kind.label().to_string(),
        TreeAt::Object(area, kind, index) => {
            let row = (contents.get(&area).into_iter())
                .flat_map(|(_, c)| c.iter())
                .find(|c| c.kind == kind)
                .and_then(|c| c.rows.get(index));
            match row {
                Some(r) if !r.text.is_empty() => r.text.clone(),
                _ => NO_NAME.to_string(),
            }
        }
    }
}

/// What a placed object's row says that has no name (nor tag).
const NO_NAME: &str = "(no name)";

/// A row's text as the Outliner shows it, for the letters typed: a kind's
/// name, an object's name (a waypoint's too, where the module tree shows
/// its tag), `(no name)` where it has none.
pub(crate) fn outliner_text(at: TreeAt, contents: &AreaContents) -> String {
    match at {
        TreeAt::Kind(_, kind) => kind.label().to_string(),
        TreeAt::Object(_, kind, index) => {
            let row = contents.iter().find(|c| c.kind == kind).and_then(|c| c.rows.get(index));
            match row {
                Some(r) if !r.name.is_empty() => r.name.clone(),
                _ => NO_NAME.to_string(),
            }
        }
        TreeAt::Group(name) => name.to_string(),
        TreeAt::Resource(k) => k.resref.to_string(),
    }
}

fn step(shown: &Shown<'_>, outs: &Outs, at: Option<TreeAt>, key: egui::Key) -> Step {
    step_in(&key_rows(shown, outs), at, key)
}

/// [`step`] among `rows` ([`KeyRow`]s, in order): the module tree's or the
/// Outliner's.
pub(crate) fn step_in(rows: &[KeyRow], at: Option<TreeAt>, key: egui::Key) -> Step {
    use egui::Key;
    let Some(first) = rows.first().map(|r| r.0) else { return Step::None };
    let last = rows[rows.len() - 1].0;
    let index = at.and_then(|a| rows.iter().position(|r| r.0 == a));
    let by = |n: isize| match index {
        Some(i) => Step::To(rows[(i as isize + n).clamp(0, rows.len() as isize - 1) as usize].0),
        // (No row yet, or one that is gone: the first.)
        None => Step::To(first),
    };
    match (key, index.map(|i| (i, rows[i]))) {
        (Key::ArrowDown, _) => by(1),
        (Key::ArrowUp, _) => by(-1),
        (Key::PageDown, _) => by(10),
        (Key::PageUp, _) => by(-10),
        (Key::Home, _) => Step::To(first),
        (Key::End, _) => Step::To(last),
        // Right opens a row that opens, then goes to the first row in it;
        // Left closes one that is open, else goes to the row it is in.
        (Key::ArrowRight, Some((_, (at, _, Some(false))))) => Step::Fold(at, true),
        (Key::ArrowRight, Some((i, (_, depth, Some(true))))) => {
            rows.get(i + 1).filter(|r| r.1 > depth).map_or(Step::None, |r| Step::To(r.0))
        }
        (Key::ArrowLeft, Some((_, (at, _, Some(true))))) => Step::Fold(at, false),
        (Key::ArrowLeft, Some((i, (_, depth, _)))) => {
            rows[..i].iter().rev().find(|r| r.1 < depth).map_or(Step::None, |r| Step::To(r.0))
        }
        (Key::Enter, Some((_, (TreeAt::Resource(k), ..)))) => Step::Open(k),
        (Key::Enter, Some((_, (TreeAt::Object(area, kind, i), ..)))) => Step::Go(area, kind, i),
        (Key::Enter, Some((_, (at, _, Some(open))))) => Step::Fold(at, !open),
        _ => Step::None,
    }
}

/// Where the keys' cursor (`cursor`) is once the placed object `deleted`
/// is deleted: at it, on the row after it among its kind's (else the one
/// before, else its kind's row); at another of its kind's, at that one
/// still (the objects after it move up their area's list).
fn past_deleted(shown: &Shown<'_>, outs: &Outs, cursor: TreeAt, deleted: TreeAt) -> TreeAt {
    past_deleted_in(&key_rows(shown, outs), cursor, deleted)
}

/// [`past_deleted`] among `rows` ([`KeyRow`]s, in order): the module
/// tree's or the Outliner's.
pub(crate) fn past_deleted_in(rows: &[KeyRow], cursor: TreeAt, deleted: TreeAt) -> TreeAt {
    let TreeAt::Object(area, kind, index) = deleted else { return cursor };
    let moved_up = |at: TreeAt| match at {
        TreeAt::Object(a, k, i) if a == area && k == kind && i > index => {
            TreeAt::Object(a, k, i - 1)
        }
        at => at,
    };
    if cursor != deleted {
        return moved_up(cursor);
    }
    let at = rows.iter().position(|r| r.0 == deleted);
    let sibling = |r: &&(TreeAt, usize, Option<bool>)| matches!(r.0, TreeAt::Object(a, k, _) if a == area && k == kind);
    let next = at.and_then(|at| rows.get(at + 1)).filter(sibling);
    let before = at.and_then(|at| at.checked_sub(1)).and_then(|at| rows.get(at)).filter(sibling);
    next.or(before).map_or(TreeAt::Kind(area, kind), |r| moved_up(r.0))
}

/// The groups' rows the tree listed the frame before (`found`: under the
/// Filter), for where its keys go when a resource they were at is gone.
#[derive(Clone)]
struct RowsBefore {
    lists: Arc<Vec<GroupList>>,
    found: Option<Arc<Vec<Vec<ResKey>>>>,
}

/// Where the keys' cursor goes once the resource `gone` is no longer in
/// its group: the row that took its place, the next of those its group
/// listed (`was`, with it) that it lists still (`now`), else the one
/// before (the last was gone); `None` when the group lists none of them
/// (its row, then).
fn past_deleted_resource(was: &[ResKey], now: &[ResKey], gone: ResKey) -> Option<ResKey> {
    let still: std::collections::HashSet<&ResKey> = now.iter().collect();
    let Some(at) = was.iter().position(|k| *k == gone) else {
        return now.first().copied();
    };
    let next = was[at + 1..].iter().find(|k| still.contains(k));
    next.or_else(|| was[..at].iter().rev().find(|k| still.contains(k))).copied()
}

fn tree_rows(app: &mut Moonglow, ui: &mut Ui, filter: &Needle, fold: Option<bool>, view: f32) {
    follow_selection(app);
    // A group the keys opened or closed (the rows shown, for the keys,
    // once the groups' lists are in hand).
    let mut outs = Outs::new();
    let folded = app.tree_fold.take();
    let cursor = app.tree_cursor;
    let cursor_moved = std::mem::take(&mut app.tree_cursor_moved);
    let mut clicked = None;
    // The resource row under the pointer, and the placed object's.
    let mut pointed = None;
    let mut pointed_object = None;
    // The plugins' commands for right-click menus, and one chosen on a
    // resource's row.
    let menu_commands = app.plugin_menu_commands();
    let mut plugin_on: Option<(usize, ResKey)> = None;
    // One chosen on a row among several chosen: on them all.
    let mut plugin_several: Option<usize> = None;
    // (A group's: the command, and the group's resources.)
    let mut plugin_group: Option<(usize, Vec<ResKey>)> = None;
    let Some(ws) = &app.ws else { return };
    let revision = ws.revision();
    // Areas opened out whose contents are to be read (after the tree is
    // drawn), and the object clicked.
    let mut read = Vec::new();
    let mut go = None;
    // The object selected is still to be gone to, next frame.
    let mut object_waits = false;
    let (by_name, resrefs) = (app.settings.area_names, app.settings.name_resrefs);
    let hide_empty = app.settings.tree_hides_empty;
    let filter = filter.clone();
    // (Marked while its tab is the one chosen.)
    let module = mg_uikit::TreeRow::new("Module Properties")
        .icon(mg_uikit::Icon::Module)
        .selected(app.tree_marked == Some(module_row()))
        .show(ui);
    if app.tree_reveal.is_some_and(|(k, _)| k == module_row()) {
        app.tree_marked = Some(module_row());
        reveal_into_view(ui, module.rect);
    }
    if module.clicked() {
        app.actions.push(Action::OpenTab(Tab::ModuleProperties));
    }
    // A row clicked (with Ctrl or Shift: chosen with others), resolved
    // once every row is laid out.
    let mut picked: Option<(TreeAt, bool, bool)> = None;
    let mut together = false;
    let mut open = None;
    let mut make = None;
    let mut listed = 0;
    // What the groups list: made when the module or the options that
    // shape the lists change (a module can hold thousands of a kind, and
    // every group was gathered, named and sorted each frame, open or not).
    let shape = (revision, ws.module.len(), by_name, resrefs);
    if app.area_names.groups.as_ref().is_none_or(|g| g.shape != shape) {
        let game = app.game.as_deref();
        let mut lists = Vec::with_capacity(GROUPS.len());
        for (_, types) in GROUPS {
            let mut keys: Vec<ResKey> =
                ws.module.keys().filter(|k| types.contains(&k.restype)).copied().collect();
            keys.sort();
            // Areas by name (Options › General): named, and in the names'
            // order; the filter finds a name or a ResRef.
            let mut names: HashMap<ResKey, String> = HashMap::new();
            let named = types == &[ResType::ARE]
                || types.iter().all(|t| mg_area::ObjectKind::from_restype(*t).is_some());
            if by_name && named {
                for k in &keys {
                    let name = if k.restype == ResType::ARE {
                        app.area_names.name(ws, game, k.resref)
                    } else {
                        app.area_names.blueprint(ws, game, *k)
                    };
                    // (One without a name keeps its ResRef.)
                    if !name.trim().is_empty() {
                        names.insert(*k, named_label(&name, k.resref, resrefs));
                    }
                }
                let shown =
                    |k: &ResKey| names.get(k).map_or_else(|| k.resref.to_string(), String::clone);
                keys.sort_by_cached_key(|k| (mg_core::text::sort_key(&shown(k)), *k));
            }
            lists.push((keys, names));
        }
        let unlisted = (ws.module.keys_of(ResType::ARE).map(|k| k.resref))
            .filter(|a| !app.area_names.listed(ws, *a))
            .collect();
        let unlisted = Arc::new(unlisted);
        app.area_names.groups = Some(Groups { shape, lists: Arc::new(lists), unlisted });
    }
    let groups = app.area_names.groups.as_ref().expect("made above");
    let (lists, unlisted) = (groups.lists.clone(), groups.unlisted.clone());
    let mut shown: Shown<'_> = Vec::new();
    // The resource to bring into view is in a group still opening: gone
    // to when it is open (its rows are where they will be then).
    let mut opening = false;
    // A tab chosen while the Filter has something typed: its rows (a tab
    // of several's, each) are listed under it too, for as long as it has
    // the same typed (the tree goes to them; the Filter stays).
    if let Some((k, _)) = app.tree_reveal.filter(|_| !filter.is_empty()) {
        let mut keys: Vec<ResKey> = app.tree_selected.iter().copied().collect();
        if !keys.contains(&k) {
            keys.push(k);
        }
        app.tree_kept = Some((keys, filter.as_str().to_string()));
    }
    // What the Filter finds: the resources by their own ResRef or name
    // (`matched`), the placed objects of each area read (every area's are
    // read while the Filter has something typed, below), and so each
    // group's rows; looked for when one of those changes, not every frame.
    if !filter.is_empty() {
        // (The rows of a tab chosen while the Filter has this typed stay
        // in sight under it: the tree goes to them.)
        let kept = app.tree_kept.as_ref().filter(|(_, f)| f == filter.as_str());
        let kept = kept.map(|(keys, _)| keys.as_slice());
        app.area_names.find(&filter, shape, &lists, &app.area_contents, kept);
    }
    let (matched, found_rows) =
        (app.area_names.filtered.matched.clone(), app.area_names.filtered.found.clone());
    // The keys' cursor at a resource gone from the module (deleted, several
    // together, its making undone): at the row that takes its place, as for
    // a placed object deleted (`past_deleted`), by the rows its group
    // listed the frame before, which held it.
    let before_id = egui::Id::new("tree-rows-before");
    let before = ui.data(|d| d.get_temp::<RowsBefore>(before_id));
    let gone = match app.tree_cursor {
        Some(TreeAt::Resource(k)) => Some(k),
        // (In an area gone: as the area's row.)
        Some(TreeAt::Kind(area, _) | TreeAt::Object(area, ..)) => {
            Some(ResKey::new(area, ResType::ARE))
        }
        _ => None,
    }
    .filter(|k| !ws.module.contains(k));
    if let Some(k) = gone
        && let Some(group) = GROUPS.iter().position(|(_, types)| types.contains(&k.restype))
    {
        let now: &[ResKey] = match filter.is_empty() {
            true => &lists[group].0,
            false => found_rows.get(group).map_or(&[][..], Vec::as_slice),
        };
        let was: Option<&[ResKey]> = before.as_ref().map(|b| match &b.found {
            None => b.lists[group].0.as_slice(),
            Some(found) => found.get(group).map_or(&[][..], Vec::as_slice),
        });
        let at = past_deleted_resource(was.unwrap_or(&[]), now, k);
        app.tree_cursor = Some(at.map_or(TreeAt::Group(GROUPS[group].0), TreeAt::Resource));
        app.tree_cursor_moved = true;
        ui.ctx().request_repaint();
    }
    let found_before = (!filter.is_empty()).then(|| found_rows.clone());
    ui.data_mut(|d| {
        d.insert_temp(before_id, RowsBefore { lists: lists.clone(), found: found_before })
    });
    // (An area's own, for its row and its objects' rows.)
    let found_of = |area: ResRef| -> Option<Arc<Vec<Order>>> {
        if filter.is_empty() {
            return None;
        }
        app.area_names.filtered.areas.get(&area).cloned().flatten()
    };
    // A group with nothing in it, left out (Options › General: Hide empty
    // groups, a builder's ask). (While the Filter has something typed, the
    // groups are as it leaves them.)
    let left_out = |all: &[ResKey]| hide_empty && all.is_empty() && filter.is_empty();
    // The tree's rows outside the Areas group's well, for its height:
    // Module Properties, each group's row and the line under them.
    let others = 2 + lists.iter().filter(|(all, _)| !left_out(all)).count();
    for (group, ((name, types), (all, names))) in GROUPS.iter().zip(lists.iter()).enumerate() {
        listed += all.len();
        if left_out(all) {
            continue;
        }
        let holds = |k: &ResKey| k.restype == ResType::ARE && found_of(k.resref).is_some();
        let listed_keys: std::borrow::Cow<'_, [ResKey]> = if filter.is_empty() {
            std::borrow::Cow::Borrowed(all.as_slice())
        } else {
            std::borrow::Cow::Borrowed(found_rows[group].as_slice())
        };
        let keys: &[ResKey] = &listed_keys;
        // What makes a new resource of the group (its wizard, or the New
        // window): on the group's and its resources' right-click menus.
        let new = new_command(types);
        let new_label = new.map(|id| new_label(&id.name()));
        let at_group = cursor == Some(TreeAt::Group(name));
        // (The group's row: its kind's icon, its name and its count.)
        let header = mg_uikit::TreeGroup::new(name, *name)
            .icon(group_icon(types))
            .count(keys.len())
            .default_open(*name == "Areas")
            // (The keys' cursor on a group shows as its row marked.)
            .marked(at_group)
            // A resource to bring into view opens its group (a filter
            // typed opens every group, through `fold`); the keys open and
            // close the one they are at.
            .open(
                (app.tree_reveal.is_some_and(|(k, _)| types.contains(&k.restype)))
                    .then_some(true)
                    .or(folded.filter(|(g, _)| *g == TreeAt::Group(name)).map(|(_, open)| open))
                    .or(fold),
            )
            .show(ui, |ui| {
              // (The Areas group's rows scroll by themselves.)
              let well = types == &[ResType::ARE];
              in_well(ui, well, (view, others, !filter.is_empty()), |ui| {
                for &k in keys {
                    let label = match (names.get(&k), k.restype) {
                        (Some(name), _) => name.clone(),
                        (None, ResType::ARE | ResType::DLG | ResType::NSS) => k.resref.to_string(),
                        _ => k.to_string(),
                    };
                    // Blueprints drag into an area, as from the palette.
                    let blueprint = mg_area::ObjectKind::from_restype(k.restype).is_some();
                    let sense = if blueprint {
                        egui::Sense::click_and_drag()
                    } else {
                        egui::Sense::click()
                    };
                    // An area opens out to what is placed in it.
                    let opened = egui::Id::new(("tree-area-open", k.resref));
                    let is_area = k.restype == ResType::ARE;
                    let mut out = is_area && ui.data(|d| d.get_temp(opened).unwrap_or(false));
                    // (Shown opened out while the Filter finds what is
                    // placed in it, until it is closed: then closed until
                    // what is typed changes. It was held open, a click on
                    // its arrow undone the next frame, a builder's report.)
                    let closed_for = egui::Id::new(("tree-area-filter-closed", k.resref));
                    let by_filter = is_area
                        && !filter.is_empty()
                        && holds(&k)
                        && ui.data(|d| {
                            let now = d.get_temp::<u64>(egui::Id::new("tree-filter-changes"));
                            d.get_temp::<Option<u64>>(closed_for) != Some(now)
                        });
                    out |= by_filter;
                    // Opened out or closed by hand (its arrow, the keys):
                    // closed, it stays closed while the Filter has the same.
                    let set_out = |ui: &mut Ui, open: bool| {
                        ui.data_mut(|d| {
                            d.insert_temp(opened, open);
                            if open {
                                d.remove_temp::<Option<u64>>(closed_for);
                            } else {
                                let now = d.get_temp::<u64>(egui::Id::new("tree-filter-changes"));
                                d.insert_temp(closed_for, now);
                            }
                        });
                    };
                    if let Some((_, open)) =
                        folded.filter(|f| is_area && f.0 == TreeAt::Resource(k))
                    {
                        out = open;
                        set_out(ui, out);
                    }
                    // The resource to bring into view (a tab chosen, or
                    // its menu's Show in Module Tree, which opens an area
                    // out too).
                    let reveal = app.tree_reveal.filter(|(key, _)| *key == k);
                    if let Some((key, open_out)) = reveal {
                        app.tree_marked = Some(key);
                        if is_area {
                            app.tree_area = Some(k.resref);
                        }
                        if open_out && is_area && !out {
                            out = true;
                            ui.data_mut(|d| d.insert_temp(opened, true));
                        }
                    }
                    let in_hand = app.tree_marked == Some(k);
                    let at_row = cursor == Some(TreeAt::Resource(k));
                    let chosen = app.tree_selected.len() > 1 && app.tree_selected.contains(&k);
                    // A row out of sight takes its room only (a module can
                    // list thousands of a kind: laid out every frame, they
                    // slowed everything down while their group was open).
                    let height = ui.spacing().interact_size.y;
                    // (Up and down only: scrolled sideways past its start,
                    // a long name's row is still in sight.)
                    let top = ui.cursor().top();
                    let room =
                        egui::Rect::from_x_y_ranges(ui.clip_rect().x_range(), top..=top + height);
                    // (The keys' row out of sight is drawn, to scroll to it.)
                    let sought = at_row && cursor_moved;
                    if !out && reveal.is_none() && !sought && !ui.is_rect_visible(room) {
                        ui.allocate_space(egui::vec2(1.0, height));
                        continue;
                    }
                    let layout = egui::Layout::left_to_right(egui::Align::Center);
                    // (The row's fill under the arrow drawn before it.)
                    let fill = ui.painter().add(egui::Shape::Noop);
                    if is_area {
                        // An area's arrow in the row's arrow column
                        // (centered on it, the name after it), before the
                        // row: the keyboard's focus comes to it first.
                        let arrow = match out {
                            true => mg_uikit::Icon::ChevronDown,
                            false => mg_uikit::Icon::ChevronRight,
                        };
                        let at = egui::Rect::from_center_size(
                            mg_uikit::tree::arrow_column(ui).center(),
                            egui::vec2(mg_uikit::metrics::ICON_BUTTON, height),
                        );
                        let mut on =
                            ui.new_child(egui::UiBuilder::new().max_rect(at).layout(layout));
                        // (Lit only under the pointer: a column of them,
                        // filled, read as a stack of tiles.) (Named with
                        // its area, said open or not.)
                        let mut button = crate::widgets::icon_only(arrow, "Placed Objects")
                            .name(format!("Placed Objects in {label}"))
                            .expanded(out);
                        // (On the marked row's fill, in its words' color,
                        // as the kit's rows draw their arrows.)
                        if in_hand || at_row || chosen {
                            use mg_uikit::TokensExt as _;
                            button = button.fill(ui.tokens().accent_fill);
                        }
                        let toggle =
                            on.add(button).on_hover_text("What is placed in the area, by kind");
                        if toggle.clicked() {
                            out = !out;
                            set_out(ui, out);
                        }
                    }
                    // (The kit's row: its fill across the whole row, the
                    // name after the arrow's column, as the board's.)
                    let marked = in_hand || at_row || chosen;
                    let mut row = mg_uikit::TreeRow::new(label.as_str())
                        .selected(marked)
                        .sense(sense)
                        .fill_under(fill);
                    if is_area {
                        row = row.arrow_apart();
                    }
                    let row = row.show_parts(ui);
                    // (A row out of sight is given this height.)
                    debug_assert!(
                        (row.response.rect.height() - height).abs() < 0.5,
                        "a tree row is {} high, not {height}",
                        row.response.rect.height()
                    );
                    if is_area {
                        // Who holds it (the project's claims table), and
                        // an ARE the module's area list leaves out (the
                        // game doesn't load it), after its name. (As wide
                        // as they take: the room given them is not taken
                        // from the row's layout.)
                        let after = egui::Rect::from_min_size(row.after, egui::vec2(4096.0, height));
                        let mut on =
                            ui.new_child(egui::UiBuilder::new().max_rect(after).layout(layout));
                        // (Their words not selectable: a click on them is
                        // the row's, as their tips are theirs.)
                        on.style_mut().interaction.selectable_labels = false;
                        crate::claims::tree_mark(crate::claims::of(&app.ws), &mut on, k.resref);
                        if unlisted.contains(&k.resref) {
                            on.weak("(not in the area list)").on_hover_text(UNLISTED_TIP);
                        }
                        // (Scrolled sideways to, as the name is.)
                        if on.min_rect().width() > 0.0 {
                            ui.expand_to_include_x(on.min_rect().right());
                        }
                    }
                    let mut r = row.response;
                    // (Opened out with its object marked: the object's row
                    // is brought into view instead, not the area's, which
                    // may be far above it.)
                    let object_here =
                        out && app.tree_object.is_some_and(|(a, ..)| a == k.resref);
                    if reveal.is_some() && object_here {
                        app.tree_object_pending = true;
                    } else if reveal.is_some() {
                        reveal_into_view(ui, r.rect);
                    }
                    if sought {
                        bring_into_view(ui, r.rect);
                    }
                    if r.clicked() {
                        clicked = Some(TreeAt::Resource(k));
                        let m = ui.input(|i| i.modifiers);
                        picked = Some((TreeAt::Resource(k), m.command || m.ctrl, m.shift));
                    }
                    if out {
                        let now = read_of(ws, k.resref);
                        match app.area_contents.get(&k.resref).filter(|c| c.0 == now) {
                            Some((_, contents)) => {
                                let in_hand = app
                                    .tree_object
                                    .filter(|(a, ..)| *a == k.resref)
                                    .map(|(_, kind, i)| (kind, i, app.tree_object_pending));
                                let chosen = (app.tree_objects.as_ref())
                                    .filter(|(a, set)| *a == k.resref && set.len() > 1)
                                    .map(|(_, set)| set);
                                let mut keys = ContentKeys {
                                    cursor,
                                    moved: cursor_moved,
                                    fold: folded,
                                    clicked: None,
                                    hovered: None,
                                    picked: None,
                                    together: false,
                                    listed: Vec::new(),
                                    eye: None,
                                };
                                let found = found_of(k.resref);
                                let went = contents_ui(
                                    ui,
                                    k.resref,
                                    contents,
                                    found.as_ref().map(|f| f.as_slice()),
                                    &mut go,
                                    in_hand,
                                    chosen,
                                    &mut keys,
                                    &menu_commands,
                                    Look::Tree { hide_empty },
                                );
                                picked = keys.picked.or(picked);
                                together |= keys.together;
                                if went {
                                    app.tree_object_pending = false;
                                } else if in_hand.is_some_and(|h| h.2) {
                                    // (Its kind's list is still opening:
                                    // gone to once its rows are laid out.)
                                    object_waits = true;
                                    ui.ctx().request_repaint();
                                }
                                clicked = keys.clicked.or(clicked);
                                pointed_object = keys.hovered.or(pointed_object);
                                outs.insert(k.resref, keys.listed);
                            }
                            None => {
                                read.push(k.resref);
                                outs.insert(k.resref, Vec::new());
                            }
                        }
                    }
                    if names.contains_key(&k) {
                        r = r.on_hover_text(k.resref.to_string());
                    }
                    if blueprint && r.drag_started() {
                        r.dnd_set_drag_payload(crate::palette_view::Dragged(k));
                    }
                    if r.double_clicked() {
                        open = Some(k);
                    }
                    // Delete on the row under the pointer asks, as its
                    // menu's Delete… does (not while text is typed): of
                    // those chosen, when it is one of them. (With the keys
                    // in the tree, the key is read below.)
                    if r.hovered() {
                        pointed = Some(k);
                    }
                    if r.hovered()
                        && cursor.is_none()
                        && keys_free(ui)
                        && ui.input(|i| i.key_pressed(egui::Key::Delete))
                    {
                        app.actions.push(delete_asked(&app.tree_selected, k));
                    }
                    mg_uikit::context_menu(&r, |ui| {
                        if chosen {
                            let keys: Vec<ResKey> = app.tree_selected.iter().copied().collect();
                            let asked = several_menu(ui, &keys, &menu_commands, &mut app.actions);
                            plugin_several = asked.or(plugin_several);
                            return;
                        }
                        if let (Some(id), Some(label)) = (new, &new_label) {
                            if ui.button(label).clicked() {
                                make = Some(id);
                            }
                            mg_uikit::menu_separator(ui);
                        }
                        if k.restype != ResType::ARE {
                            if ui.button("Edit").clicked() {
                                open = Some(k);
                            }
                            if crate::model_view::previewable(k.restype)
                                && ui
                                    .button("Preview")
                                    .on_hover_text("See it in the 3D viewer")
                                    .clicked()
                            {
                                app.actions.push(Action::OpenTab(Tab::Model(k)));
                            }
                            mg_uikit::menu_separator(ui);
                        }
                        if k.restype == ResType::ARE && unlisted.contains(&k.resref) {
                            if ui
                                .button("Add to the Module's Area List")
                                .on_hover_text(
                                    "The game loads only the areas the module's list names",
                                )
                                .clicked()
                                && let Some(add) = app.ws.as_ref().and_then(|w| list_area(w, k.resref))
                            {
                                app.actions.push(Action::Apply(add));
                            }
                            mg_uikit::menu_separator(ui);
                        }
                        if k.restype == ResType::ARE {
                            if ui
                                .button("Edit Areas Together…")
                                .on_hover_text(
                                    "Choose more areas to set their properties with this one's",
                                )
                                .clicked()
                            {
                                app.area_chooser =
                                    Some(crate::area_props::AreaChooser::with(k.resref));
                            }
                            if ui.button("View Area").clicked() {
                                app.actions.push(Action::OpenTab(Tab::Area(k.resref)));
                            }
                            let p = crate::claims::of(&app.ws);
                            crate::claims::menu_items(p, &mut app.actions, ui, k.resref);
                            let areas = [k.resref];
                            placeables_menu(ui, &areas, "of the area", &mut app.actions);
                            // Its two files' fields, as the game reads them.
                            mg_uikit::menu_button(ui, "Fields", |ui| {
                                for t in [ResType::ARE, ResType::GIT] {
                                    let g = ResKey::new(k.resref, t);
                                    if ui.button(format!("Fields of {g}")).clicked() {
                                        app.actions.push(Action::OpenTab(Tab::Gff(g)));
                                    }
                                }
                            });
                            mg_uikit::menu_separator(ui);
                        }
                        if mg_module::rename::renamable(k.restype) {
                            if ui.button("Find References").clicked() {
                                app.actions.push(Action::FindReferences(k));
                            }
                            if ui.button("Rename…").clicked() {
                                app.actions.push(Action::RenameDialog(k));
                            }
                        } else if k.restype == ResType::JUI
                            && ui
                                .button("Rename…")
                                .on_hover_text(
                                    "With its settings and its scripts, and the scripts that open it",
                                )
                                .clicked()
                        {
                            app.actions.push(Action::RenameDialog(k));
                        }
                        if ui
                            .button("Copy…")
                            .on_hover_text("A copy in the module, under a ResRef you give")
                            .clicked()
                        {
                            app.actions.push(Action::CopyDialog(k));
                        }
                        if ui.button("Delete…").clicked() {
                            app.actions.push(Action::DeleteDialog(k));
                        }
                        // The ways of writing it out, together.
                        mg_uikit::menu_button(ui, "Export", |ui| {
                            if ui.button("Export…").clicked() {
                                app.actions.push(Action::ExportDialog(vec![k]));
                            }
                            let with = match k.restype {
                                ResType::NSS => " (with its compiled script)",
                                ResType::ARE => " (with its .git and .gic)",
                                _ => "",
                            };
                            if ui
                                .button("Export as Files…")
                                .on_hover_text(format!("Write {k} into a folder{with}"))
                                .clicked()
                            {
                                app.actions.push(Action::ExportFiles {
                                    keys: vec![k],
                                    dependencies: false,
                                    scratch: false,
                                });
                            }
                            if k.restype == ResType::ARE
                                && ui
                                    .button("Export Model Files…")
                                    .on_hover_text(
                                        "Write the tileset, models and textures the area is drawn \
                                         with into a folder",
                                    )
                                    .clicked()
                            {
                                app.actions.push(Action::ExportAreaFiles(k.resref));
                            }
                            let tip = crate::transfer::scratch_tip(
                                app.settings.scratch_dir.as_deref(),
                                &format!("{k}{with}"),
                            );
                            if k.restype == ResType::ARE {
                                minimap_items(ui, &[k.resref], &mut app.actions);
                            }
                            if ui.button("Copy to Scratch Folder").on_hover_text(tip).clicked() {
                                app.actions.push(Action::ExportFiles {
                                    keys: vec![k],
                                    dependencies: false,
                                    scratch: true,
                                });
                            }
                            story_items(ui, &[k], &mut app.actions);
                        });
                        // The plugins' commands for it (an area, a
                        // blueprint), before an area's Properties.
                        let target = crate::plugins::selected_at(k, &mg_edit::GffPath::root())
                            .target();
                        if let Some(t) = target
                            && let Some(i) = crate::plugins::menu_commands(ui, &menu_commands, &[t])
                        {
                            plugin_on = Some((i, k));
                        }
                        // An area's Properties last, as in Aurora.
                        if k.restype == ResType::ARE {
                            mg_uikit::menu_separator(ui);
                            if ui.button("Properties").clicked() {
                                app.actions.push(Action::OpenTab(Tab::AreaProperties(k.resref)));
                            }
                        }
                    });
                }
              })
            });
        if *name == "Areas" {
            // The object selected was gone to if its area is opened out;
            // if not (or the areas are folded away), the tree stays as it
            // is. (One whose area is being read again is gone to then.)
            if !object_waits && !app.tree_object.is_some_and(|(area, ..)| read.contains(&area)) {
                app.tree_object_pending = false;
            }
        }
        mg_uikit::say_expanded(&header.header_response, header.body_returned.is_some());
        if header.header_response.clicked() {
            clicked = Some(TreeAt::Group(name));
        }
        if at_group && cursor_moved {
            bring_into_view(ui, header.header_response.rect);
        }
        opening |= header.openness < 1.0
            && app.tree_reveal.is_some_and(|(k, _)| types.contains(&k.restype));
        // (The plugins' commands for every one of the group, where they are
        // all of one kind.)
        let group_on: Vec<mg_plugin::Selected> = (listed_keys.iter())
            .map(|k| crate::plugins::selected_at(*k, &mg_edit::GffPath::root()))
            .collect();
        let group_kinds = crate::plugins::kinds_of(&group_on);
        let group_has =
            group_kinds.len() == 1 && menu_commands.iter().any(|c| c.fits(&group_kinds));
        if new.is_none() && group_has {
            mg_uikit::context_menu(&header.header_response, |ui| {
                if let Some(i) = crate::plugins::menu_commands(ui, &menu_commands, &group_kinds) {
                    plugin_group = Some((i, listed_keys.to_vec()));
                }
            });
        }
        shown.push((*name, !header.fully_closed(), listed_keys.clone()));
        if let (Some(id), Some(label)) = (new, &new_label) {
            mg_uikit::context_menu(&header.header_response, |ui| {
                if group_has
                    && let Some(i) = crate::plugins::menu_commands(ui, &menu_commands, &group_kinds)
                {
                    plugin_group = Some((i, listed_keys.to_vec()));
                }
                if ui.button(label).clicked() {
                    make = Some(id);
                }
                if types.contains(&ResType::ARE)
                    && ui
                        .button("Make Placeables Static in Every Area")
                        .on_hover_text(STATIC_TIP)
                        .clicked()
                {
                    let areas = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
                    app.actions.push(Action::StaticPlaceables(areas));
                }
                if types.contains(&ResType::ARE)
                    && ui
                        .button("Make Placeables Dynamic in Every Area")
                        .on_hover_text("Clears Static on every placeable of the module")
                        .clicked()
                {
                    let areas = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
                    app.actions.push(Action::DynamicPlaceables(areas));
                }
            });
        }
    }
    // (Brought into view, or not among the rows shown: done.)
    if !opening {
        app.tree_reveal = None;
    }
    // While the Filter has something typed, what every area holds is
    // read (not only the areas opened out), for what it finds there.
    if !filter.is_empty() {
        // (Those opened out are to be read already: looked up in a set, the
        // list was gone through for each area, 1,500 times 1,500 a frame
        // until a world's areas were all read.)
        let opened: std::collections::HashSet<ResRef> = read.iter().copied().collect();
        for k in ws.module.keys_of(ResType::ARE) {
            let now = read_of(ws, k.resref);
            if app.area_contents.get(&k.resref).is_none_or(|c| c.0 != now)
                && !opened.contains(&k.resref)
            {
                read.push(k.resref);
            }
        }
    }
    let others = ws.module.len() - listed;
    mg_uikit::TreeRow::new(format!("{others} other resources")).dim().show(ui);
    // Down from the Filter: to the first row in sight that it finds (a
    // resource by its ResRef or name, a placed object: an area listed for
    // its objects alone is passed over for them), else the first row.
    // (An object listed under the filter is one it found, or one of an
    // area found itself, which comes first.)
    if std::mem::take(&mut app.tree_to_match) {
        let rows = key_rows(&shown, &outs);
        let found = |at: &TreeAt| match at {
            TreeAt::Resource(k) => matched.contains(k),
            TreeAt::Object(..) => true,
            _ => false,
        };
        let first = rows.iter().map(|r| r.0).find(|at| !filter.is_empty() && found(at));
        app.tree_cursor = first.or(Some(first_row()));
        app.tree_cursor_moved = true;
        ui.ctx().request_repaint();
    }
    // A row clicked: with Ctrl, added to (or taken from) those chosen;
    // with Shift, those from the row clicked before to it, as listed. A
    // placed object's: the area view's selection follows.
    match picked {
        Some((TreeAt::Resource(k), command, shift)) => {
            let rows = key_rows(&shown, &outs);
            let listed: Vec<ResKey> = (rows.iter())
                .filter_map(|r| match r.0 {
                    TreeAt::Resource(k) => Some(k),
                    _ => None,
                })
                .collect();
            let (selected, anchor) = (&mut app.tree_selected, &mut app.tree_anchor);
            crate::widgets::select_in_list(selected, anchor, &listed, k, command, shift);
        }
        Some((TreeAt::Object(area, kind, index), command, shift)) => {
            let rows = key_rows(&shown, &outs);
            let listed: Vec<Placed> = (rows.iter())
                .filter_map(|r| match r.0 {
                    TreeAt::Object(a, k, i) if a == area => Some((k, i)),
                    _ => None,
                })
                .collect();
            // (Another area's chosen before: from the one in hand here.)
            let (mut set, mut anchor) = match app.tree_objects.take() {
                Some((a, set)) if a == area => {
                    (set, Anchor::in_area(app.tree_objects_anchor, &app.area_contents, area))
                }
                _ => {
                    let here = app.tree_object.filter(|o| o.0 == area).map(|o| (o.1, o.2));
                    (here.into_iter().collect(), here)
                }
            };
            crate::widgets::select_in_list(
                &mut set,
                &mut anchor,
                &listed,
                (kind, index),
                command,
                shift,
            );
            app.tree_objects_anchor = anchor.map(|a| Anchor::new(&app.area_contents, area, a));
            let chosen: Vec<Placed> = set.iter().copied().collect();
            show_kinds_of(app, area, &chosen);
            if let Some(view) = app.area_views.get_mut(&area) {
                view.selection = chosen;
            }
            app.tree_objects = (!set.is_empty()).then_some((area, set));
        }
        _ => {}
    }
    // A click in the tree gives it the arrow keys (as the palette's),
    // wherever the pointer goes after until it is in an area's view: Up
    // and Down go from row to row (Page Up and Down by ten, Home and End
    // to the ends), Right opens a group and goes into it, Left goes back
    // to the group and closes it, Enter opens the row's resource, Escape
    // hands the keys back.
    if let Some(at) = clicked {
        app.tree_cursor = Some(at);
        crate::palette_view::give_keys(ui.ctx(), crate::palette_view::Keys::Tree);
    }
    if crate::palette_view::keys_are(ui.ctx(), crate::palette_view::Keys::Tree)
        && keys_free(ui)
        && app.tree_cursor.is_some()
    {
        use egui::Key;
        let keys = [
            Key::ArrowDown,
            Key::ArrowUp,
            Key::PageDown,
            Key::PageUp,
            Key::Home,
            Key::End,
            Key::ArrowRight,
            Key::ArrowLeft,
            Key::Enter,
        ];
        let pressed =
            ui.input_mut(|i| keys.into_iter().find(|k| i.consume_key(egui::Modifiers::NONE, *k)));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            app.tree_cursor = None;
            crate::palette_view::give_arrows(ui.ctx(), false);
        }
        // Letters typed while the tree has the keys, wherever the pointer
        // is, go to the row whose name begins with them, as in Aurora's
        // (Windows') tree: a pause starts over ([`typed_row`]).
        let named = |k: &ResKey| lists.iter().find_map(|(_, names)| names.get(k).cloned());
        let text = |at: TreeAt| row_text(at, &named, &app.area_contents);
        let seek_from = |from: Option<TreeAt>, typed: &str| seek(&shown, &outs, from, typed, &text);
        let to = typed_row(ui, egui::Id::new("tree-typed"), app.tree_cursor, &seek_from);
        if let Some(at) = to {
            app.tree_cursor = Some(at);
            app.tree_cursor_moved = true;
            ui.ctx().request_repaint();
        }
        // F2 and Delete on the row under the pointer, else at the
        // cursor's: a resource's as its menu's Rename… and Delete… (both
        // ask); a placed object's Delete as its menu's (undone as any
        // change; the keys go on to the row after it). F2 does nothing at
        // a placed object: its menu has no Rename (nor has Aurora's), its
        // name and tag are its Properties'.
        let under = pointed.map(TreeAt::Resource).or(pointed_object).or(app.tree_cursor);
        match under {
            Some(TreeAt::Resource(k)) => {
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::F2)) {
                    app.actions.push(Action::RenameDialog(k));
                }
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Delete)) {
                    app.actions.push(delete_asked(&app.tree_selected, k));
                }
            }
            Some(at @ TreeAt::Object(area, kind, index))
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Delete)) =>
            {
                go = Some((area, kind, index, Asked::Delete));
                together = true;
                // (Refused in a read-only area, `act_on_object`: the keys
                // stay at the row, as everything else does.)
                if crate::claims::read_only(app, area).is_none() {
                    app.tree_cursor = app.tree_cursor.map(|c| past_deleted(&shown, &outs, c, at));
                }
                ui.ctx().request_repaint();
            }
            _ => {}
        }
        match pressed.map_or(Step::None, |key| step(&shown, &outs, app.tree_cursor, key)) {
            Step::To(at) => {
                app.tree_cursor = Some(at);
                app.tree_cursor_moved = true;
                ui.ctx().request_repaint();
            }
            Step::Fold(at, open) => {
                app.tree_fold = Some((at, open));
                ui.ctx().request_repaint();
            }
            Step::Open(k) => open = Some(k),
            Step::Go(area, kind, index) => go = Some((area, kind, index, Asked::Go)),
            Step::None => {}
        }
    }
    if let Some(k) = open {
        app.actions.push(Action::OpenResource(k));
    }
    // What the areas opened out hold, read for the next frame (those the
    // Filter looks in after them; a module's hundreds of areas over a
    // few frames, not all in one).
    if !read.is_empty()
        && let Some(ws) = app.ws.as_ref()
    {
        let began = std::time::Instant::now();
        for area in read {
            let contents = area_contents(ws, app.game.as_deref(), area);
            app.area_contents.insert(area, (read_of(ws, area), contents));
            app.area_names.read_again(area);
            if began.elapsed() > std::time::Duration::from_millis(12) {
                break;
            }
        }
        ui.ctx().request_repaint();
    }
    // A plugin's command chosen on a row among several chosen: on each.
    let root = mg_edit::GffPath::root();
    if let Some(i) = plugin_several {
        let on =
            (app.tree_selected.iter()).map(|k| crate::plugins::selected_at(*k, &root)).collect();
        app.actions.push(crate::plugins::run_on(&menu_commands[i], on));
    }
    // A plugin's command on a resource's row: on the area, blueprint or
    // resource.
    if let Some((i, k)) = plugin_on {
        let on = crate::plugins::selected_at(k, &root);
        app.actions.push(crate::plugins::run_on(&menu_commands[i], vec![on]));
    }
    // A plugin's command on a group's row: on every one of the group.
    if let Some((i, keys)) = plugin_group {
        let on = keys.iter().map(|k| crate::plugins::selected_at(*k, &root)).collect();
        app.actions.push(crate::plugins::run_on(&menu_commands[i], on));
    }
    // An object in an area's list: gone to in the area's view, its
    // Properties opened, copied or deleted, or a plugin's command run on
    // it (on those chosen with it, where asked of them together).
    if let Some(go) = go {
        let chosen = (app.tree_objects.as_ref())
            .filter(|(a, _)| together && *a == go.0)
            .map(|(_, set)| set.clone());
        act_on_object(app, ui, go, chosen.as_ref(), &menu_commands, true);
    }
    if let Some(id) = make {
        id.run(app, ui.ctx());
    }
}

/// The row the letters typed this frame go to, as a type-ahead (the module
/// tree's and the Outliner's, while they have the keys), from `cursor`:
/// `seek` finds it from a row by what was typed in quick succession,
/// which is kept under `typed_id` between frames.
pub(crate) fn typed_row(
    ui: &Ui,
    typed_id: egui::Id,
    cursor: Option<TreeAt>,
    seek: &dyn Fn(Option<TreeAt>, &str) -> Option<TreeAt>,
) -> Option<TreeAt> {
    // Letters typed while the tree has the keys, wherever the pointer
    // is, go to the row whose name begins with them, as in Aurora's
    // (Windows') tree: a pause starts over. (The pointer in an area's
    // view gives the keys back to it, and its letters to the camera.)
    // (Not while something is dragged: a blueprint dragged from the
    // tree onto an area, whose view has Q and E to turn it.)
    let dragging = ui.ctx().dragged_id().is_some();
    if !dragging {
        // (Not the camera's too, held down: the area is drawn after.)
        crate::palette_view::take_letters(ui.ctx());
    }
    let now = ui.input(|i| i.time);
    let (mut typed, mut typed_at) =
        ui.data(|d| d.get_temp::<(String, f64)>(typed_id)).unwrap_or_default();
    let going_on = !typed.is_empty() && now - typed_at <= crate::dialog_view::TYPE_AHEAD;
    let letters = ui.input_mut(|i| {
        let m = i.modifiers;
        if dragging || m.ctrl || m.alt || m.command || m.mac_cmd {
            return Vec::new();
        }
        // What is typed, as a name has it ("Beggar's Nest: T"; a
        // space goes on with letters being typed, and starts none).
        let mut letters: Vec<char> = Vec::new();
        i.events.retain(|e| {
            let egui::Event::Text(t) = e else { return true };
            let on = going_on || !letters.is_empty();
            let name_s = |c: char| (!c.is_whitespace() && !c.is_control()) || (c == ' ' && on);
            let taken = !t.is_empty() && t.chars().all(name_s);
            if taken {
                letters.extend(t.chars());
            }
            !taken
        });
        // (A key that types nothing, as a test's: its letter or digit.)
        let letter = |key: &egui::Key| {
            let name = key.name();
            let c = name.chars().next().filter(|_| name.len() == 1)?;
            c.is_ascii_alphanumeric().then(|| c.to_ascii_lowercase())
        };
        if letters.is_empty() && m.is_none() {
            letters = (i.events.iter())
                .filter_map(|e| match e {
                    egui::Event::Key { key, pressed: true, .. } => letter(key),
                    _ => None,
                })
                .collect();
        }
        // (Their keys too: no command has them meanwhile.)
        i.events.retain(|e| match e {
            egui::Event::Key { key: egui::Key::Space, .. } => !letters.contains(&' '),
            egui::Event::Key { key, .. } => {
                letter(key).is_none_or(|c| !letters.iter().any(|t| t.eq_ignore_ascii_case(&c)))
            }
            _ => true,
        });
        letters
    });
    let mut to = None;
    for c in letters.iter().copied() {
        if now - typed_at > crate::dialog_view::TYPE_AHEAD {
            typed.clear();
        }
        typed.push(c);
        typed_at = now;
        to = seek(to.or(cursor), &typed).or(to);
    }
    if !letters.is_empty() {
        ui.data_mut(|d| d.insert_temp(typed_id, (typed, typed_at)));
    }
    to
}

/// What was asked of a placed object's row (`go`: its area, kind, place in
/// its list, and what), from the module tree (`in_tree`) or the Outliner:
/// gone to in the area's view, selected there, its Properties opened, its
/// variables, its model, written out, Adjust Location, copied, deleted, or
/// a plugin's command run on it. Asked of a row among several chosen
/// together (`together`, which holds it), of them all.
pub(crate) fn act_on_object(
    app: &mut Moonglow,
    ui: &Ui,
    (area, kind, index, asked): (ResRef, mg_area::ObjectKind, usize, Asked),
    together: Option<&BTreeSet<Placed>>,
    menu_commands: &[crate::plugins::PluginCommand],
    in_tree: bool,
) {
    let git = ResKey::new(area, ResType::GIT);
    // Asked of a row among several chosen (its menu, the Delete key):
    // of them all.
    let targets: Vec<Placed> = match together {
        Some(set) if set.contains(&(kind, index)) => set.iter().copied().collect(),
        _ => vec![(kind, index)],
    };
    let path = |(kind, index): Placed| mg_edit::GffPath::root().item(kind.list(), index);
    // Gone to or selected, an object of a kind hidden in the view: the kind
    // shown, in every view, as its Show toggle would (the builder asked to
    // see it; a hidden object cannot stay selected). One rule for the
    // module tree and the Outliner.
    if matches!(asked, Asked::Go | Asked::Select | Asked::Adjust) {
        show_kinds_of(app, area, &targets);
    }
    match asked {
        Asked::Plugin(i) => {
            let on = (targets.iter())
                .map(|&(kind, index)| mg_plugin::Selected::Object { area, kind, index })
                .collect();
            app.actions.push(crate::plugins::run_on(&menu_commands[i], on));
        }
        Asked::Properties if targets.len() > 1 => {
            let paths = targets.iter().copied().map(path).collect();
            app.actions.push(Action::OpenTab(Tab::Instances { area, paths }));
        }
        Asked::Properties => {
            let path = path((kind, index));
            app.actions.push(Action::OpenTab(Tab::Instance { area, path }));
        }
        Asked::Variables => crate::widgets::open_variables(app, git, path((kind, index))),
        Asked::Preview => {
            let path = path((kind, index));
            app.actions.push(Action::OpenTab(Tab::InstanceModel { area, path }));
        }
        Asked::Export => app.actions.push(Action::ExportObject { area, kind, index }),
        Asked::Adjust => {
            // Selected in the area's view (once it is open and has
            // them), then Adjust Location for them, as its menu's.
            app.area_focus = Some((area, kind, index));
            app.area_select = Some((area, targets));
            app.area_adjust = Some(area);
            app.actions.push(Action::OpenTab(Tab::Area(area)));
        }
        Asked::Go if targets.len() > 1 => {
            // Each of them selected in the area's view (once it is
            // open), the camera at the one clicked.
            app.area_focus = Some((area, kind, index));
            app.area_select = Some((area, targets.clone()));
            if let Some(view) = app.area_views.get_mut(&area) {
                view.selection = targets;
            }
            app.actions.push(Action::OpenTab(Tab::Area(area)));
        }
        Asked::Go => {
            app.area_focus = Some((area, kind, index));
            // (The row clicked is the one marked: the tree stays as
            // it is, rather than go to the area's row and back.)
            if in_tree {
                app.tree_object = Some((area, kind, index));
                app.tree_object_pending = false;
                // (And the one a Shift+click goes from.)
                app.tree_objects = Some((area, [(kind, index)].into()));
                app.tree_objects_anchor =
                    Some(Anchor::new(&app.area_contents, area, (kind, index)));
            }
            if let Some(view) = app.area_views.get_mut(&area) {
                view.selection = vec![(kind, index)];
            }
            app.actions.push(Action::OpenTab(Tab::Area(area)));
        }
        Asked::Select => select_in_view(app, area, vec![(kind, index)]),
        Asked::Copy => {
            // As Copy in the area's view: pasted in any area (Ctrl+V),
            // each as high above the ground as it stands (the area's
            // ground read for it where its view isn't open).
            let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else {
                return;
            };
            let list = |ws: &mut Workspace, (kind, index): Placed| {
                ws.doc(&git).ok()?.root.list(kind.list())?.get(index).cloned()
            };
            let placed: Vec<(Placed, mg_gff::Struct)> =
                targets.iter().filter_map(|&t| Some((t, list(ws, t)?))).collect();
            let ground = match app.area_views.get(&area) {
                Some(_) => None,
                None => crate::area_view::ground_of(game, ws, area),
            };
            let mut objects = Vec::new();
            for ((kind, index), s) in placed {
                let o = mg_area::AreaObject::read(game, kind, index, &s);
                let lift = match app.area_views.get(&area) {
                    Some(view) => view.lift_of(&o),
                    None => crate::area_view::lift_on(ground.as_ref(), &o),
                };
                objects.push((o, s, lift));
            }
            if let Some(anchor) = objects.first().map(|o| o.0.position) {
                let n = objects.len();
                let what = if n == 1 { "1 object".to_string() } else { format!("{n} objects") };
                app.object_clip = Some(crate::area_view::ObjectClip { objects, anchor });
                crate::widgets::mark_clipboard(ui.ctx(), &what);
                app.log.info(format!("{what} copied: paste in an area (Ctrl+V)"));
            }
        }
        Asked::Delete => {
            let edits = mg_area::edit::delete_edits(git, &targets);
            // (Another builder's area, kept read-only: the delete goes to
            // the claims' gate, which refuses it and says why; what is
            // selected, marked and open there stays as it is.)
            if crate::claims::read_only(app, area).is_some() {
                if !edits.is_empty() {
                    app.actions.push(Action::Apply(mg_edit::Command::new("Delete", edits)));
                }
                return;
            }
            // (Objects after it move up their list: what is selected
            // in the area's view, and the Properties open, would be
            // others'.)
            if let Some(view) = app.area_views.get_mut(&area) {
                view.selection.clear();
            }
            // (Nor is the one marked: the keys' cursor stays where it
            // was put, rather than follow the selection cleared.)
            if app.tree_object.is_some_and(|(a, ..)| a == area) {
                app.tree_object = None;
                app.tree_object_pending = false;
            }
            if app.tree_objects.as_ref().is_some_and(|(a, _)| *a == area) {
                app.tree_objects = None;
                app.tree_objects_anchor = None;
                // (Several gone: the keys go to the kind's row.)
                if targets.len() > 1
                    && let Some(TreeAt::Object(a, k, _)) = app.tree_cursor
                {
                    app.tree_cursor = Some(TreeAt::Kind(a, k));
                }
            }
            crate::tabs::close_tabs(&mut app.dock, |t| {
                !matches!(t, Tab::Instance { area: a, .. } | Tab::Instances { area: a, .. }
                    if *a == area)
            });
            if !edits.is_empty() {
                app.actions.push(Action::Apply(mg_edit::Command::new("Delete", edits)));
            }
        }
    }
}

/// The kinds of `objects` that `area`'s view hides, shown (in every view,
/// as their Show toggles would): for objects gone to or selected.
pub(crate) fn show_kinds_of(app: &mut Moonglow, area: ResRef, objects: &[Placed]) {
    for (kind, _) in objects {
        if !crate::area_view::kinds_shown(app, area)[kind.index()] {
            crate::area_view::show_kind(app, area, *kind, true);
        }
    }
}

/// `selection` selected in `area`'s view, where it is open: as a click on
/// an object there, which leaves Select Tiles for the objects (the tiles
/// selected let go of).
pub(crate) fn select_in_view(app: &mut Moonglow, area: ResRef, selection: Vec<Placed>) {
    if let Some(view) = app.area_views.get_mut(&area) {
        if !selection.is_empty() {
            view.tile_mode = false;
            view.tile_selection.clear();
        }
        view.selection = selection;
    }
}

/// The Delete key on a resource's row: Delete… of those chosen, when it is
/// one of several chosen, else of it alone.
fn delete_asked(selected: &std::collections::BTreeSet<ResKey>, k: ResKey) -> Action {
    match selected.len() > 1 && selected.contains(&k) {
        true => Action::DeleteChosenDialog(selected.iter().copied().collect()),
        false => Action::DeleteDialog(k),
    }
}

/// The Export menu's items for conversations (`keys`, one or several
/// chosen, when each is one): Export as Ink…, Twine…, Text… and CSV…, each
/// a file of its own in a folder asked for, as the conversation editor's
/// Export writes one (`dialog_export`).
fn story_items(ui: &mut Ui, keys: &[ResKey], actions: &mut Vec<Action>) {
    if !crate::dialog_export::all_conversations(keys) {
        return;
    }
    mg_uikit::menu_separator(ui);
    let each = if keys.len() == 1 { "It" } else { "Each" };
    for (format, word) in crate::dialog_export::FORMATS {
        let ext = format.extensions()[0];
        let tip = format!("{each} as {} into a folder (NAME.{ext})", format.name());
        if ui.button(format!("Export as {word}…")).on_hover_text(tip).clicked() {
            actions.push(Action::ExportConversations { keys: keys.to_vec(), format });
            ui.close();
        }
    }
}

/// The Export menu's minimap items for areas (one, or several chosen):
/// Export Minimap… (a PNG chosen) or Export Minimaps… (each `NAME.png` in a
/// folder chosen), and each with Map Notes, the areas' enabled map notes
/// marked as the game's map marks them.
fn minimap_items(ui: &mut Ui, areas: &[ResRef], actions: &mut Vec<Action>) {
    let (what, tip) = match areas {
        [_] => ("Minimap", "The area's map as the game shows it, as a PNG"),
        _ => ("Minimaps", "Each area's map as the game shows it, as NAME.png in a folder"),
    };
    let notes = " with its map notes' pins";
    for (label, tip, notes) in [
        (format!("Export {what}…"), tip.to_string(), false),
        (format!("Export {what} with Map Notes…"), format!("{tip},{notes}"), true),
    ] {
        if ui.button(label).on_hover_text(tip).clicked() {
            actions.push(Action::ExportMinimaps { areas: areas.to_vec(), notes });
            ui.close();
        }
    }
}

/// The right-click menu of a row among several resources chosen in the
/// module tree (`keys`): what is done to them together. Edit Together for
/// blueprints of one kind or for areas (Properties of several); Export…,
/// Export as Files…, Copy to Scratch Folder (and, for conversations,
/// Export as Ink…, Twine…, Text… and CSV…); Delete…; for areas, Make
/// Placeables Static and Dynamic; the plugins' commands for them all (the
/// one chosen returned).
fn several_menu(
    ui: &mut Ui,
    keys: &[ResKey],
    commands: &[crate::plugins::PluginCommand],
    actions: &mut Vec<Action>,
) -> Option<usize> {
    let n = keys.len();
    mg_uikit::menu_note(ui, format!("{n} resources chosen"));
    mg_uikit::menu_separator(ui);
    let first = keys.first()?.restype;
    let one_type = keys.iter().all(|k| k.restype == first);
    let areas: Vec<ResRef> = keys.iter().map(|k| k.resref).collect();
    let together = "One editor: what you change is set on each";
    if one_type && first == ResType::ARE {
        if ui.button(format!("Edit {n} Together")).on_hover_text(together).clicked() {
            actions.push(Action::OpenTab(Tab::AreasProperties(areas.clone())));
            ui.close();
        }
        placeables_menu(ui, &areas, "of the areas", actions);
        mg_uikit::menu_separator(ui);
    } else if one_type
        && mg_area::ObjectKind::from_restype(first).is_some()
        && ui.button(format!("Edit {n} Together")).on_hover_text(together).clicked()
    {
        actions.push(Action::OpenTab(Tab::Blueprints(keys.to_vec())));
        ui.close();
    }
    // The ways of writing them out, together (as one's menu has them).
    mg_uikit::menu_button(ui, "Export", |ui| {
        mg_uikit::menu_rows(ui);
        if ui.button("Export…").clicked() {
            actions.push(Action::ExportDialog(keys.to_vec()));
            ui.close();
        }
        let files = "Write them into a folder as loose files";
        if ui.button("Export as Files…").on_hover_text(files).clicked() {
            let keys = keys.to_vec();
            actions.push(Action::ExportFiles { keys, dependencies: false, scratch: false });
            ui.close();
        }
        if ui.button("Copy to Scratch Folder").clicked() {
            let keys = keys.to_vec();
            actions.push(Action::ExportFiles { keys, dependencies: false, scratch: true });
            ui.close();
        }
        if one_type && first == ResType::ARE {
            mg_uikit::menu_separator(ui);
            minimap_items(ui, &areas, actions);
        }
        story_items(ui, keys, actions);
    });
    let delete = "Delete them from the module in one step, asked first";
    if ui.button("Delete…").on_hover_text(delete).clicked() {
        actions.push(Action::DeleteChosenDialog(keys.to_vec()));
        ui.close();
    }
    // The plugins' commands for them, where they are of one kind (areas,
    // or blueprints of one type).
    let root = mg_edit::GffPath::root();
    let target = (one_type).then(|| crate::plugins::selected_at(keys[0], &root).target()).flatten();
    target.and_then(|t| crate::plugins::menu_commands(ui, commands, &[t]))
}

/// Make Placeables Static and Make Placeables Dynamic on `areas`, in a
/// submenu of their own (an area's menu, and several areas'): `whose`
/// ends the second's tip ("of the area").
fn placeables_menu(ui: &mut Ui, areas: &[ResRef], whose: &str, actions: &mut Vec<Action>) {
    mg_uikit::menu_button(ui, "Placeables", |ui| {
        mg_uikit::menu_rows(ui);
        if ui.button("Make Placeables Static").on_hover_text(STATIC_TIP).clicked() {
            actions.push(Action::StaticPlaceables(areas.to_vec()));
            ui.close();
        }
        if ui
            .button("Make Placeables Dynamic")
            .on_hover_text(format!("Clears Static on every placeable {whose}"))
            .clicked()
        {
            actions.push(Action::DynamicPlaceables(areas.to_vec()));
            ui.close();
        }
    });
}

/// A named resource in the tree: its name, and its ResRef in parentheses
/// when asked for (Options › General).
fn named_label(name: &str, resref: ResRef, resrefs: bool) -> String {
    if resrefs { format!("{name} ({resref})") } else { name.to_string() }
}

/// The menu entry for a command that makes something new, from its name:
/// "Area Wizard" is "New Area…", "New Script" is "New Script…".
pub(crate) fn new_label(command: &str) -> String {
    let what = command.trim_end_matches(" Wizard");
    if what.starts_with("New ") { format!("{what}…") } else { format!("New {what}…") }
}

/// The command that makes a new resource of a tree group's types: the
/// Wizards menu's for it.
fn new_command(types: &[ResType]) -> Option<crate::commands::Id> {
    use crate::commands::Id;
    Some(match *types.first()? {
        ResType::ARE => Id::AreaWizard,
        ResType::DLG => Id::NewConversation,
        ResType::NSS => Id::NewScript,
        ResType::JUI => Id::NewNui,
        t => Id::new_blueprint(ObjectKind::from_restype(t)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keys_steps() {
        use egui::Key;
        let key = |name: &str, t| ResKey::parse(name, t).unwrap();
        let (a, b, c) = (key("a", ResType::ARE), key("b", ResType::ARE), key("c", ResType::NSS));
        let shown: Shown = vec![
            ("Areas", true, vec![a, b].into()),
            ("Conversations", false, vec![].into()),
            ("Scripts", false, vec![c].into()),
        ];
        let (areas, scripts) = (TreeAt::Group("Areas"), TreeAt::Group("Scripts"));
        let at = |r| Some(TreeAt::Resource(r));
        let outs = Outs::new();
        // Down and up, row by row, past a closed group's resources.
        assert_eq!(step(&shown, &outs, Some(areas), Key::ArrowDown), Step::To(TreeAt::Resource(a)));
        assert_eq!(
            step(&shown, &outs, at(b), Key::ArrowDown),
            Step::To(TreeAt::Group("Conversations"))
        );
        assert_eq!(step(&shown, &outs, Some(scripts), Key::ArrowDown), Step::To(scripts));
        assert_eq!(step(&shown, &outs, Some(areas), Key::ArrowUp), Step::To(areas));
        assert_eq!(step(&shown, &outs, at(a), Key::End), Step::To(scripts));
        assert_eq!(step(&shown, &outs, at(b), Key::Home), Step::To(areas));
        assert_eq!(step(&shown, &outs, Some(areas), Key::PageDown), Step::To(scripts));
        // Right opens a group, then goes into it; Left the other way.
        assert_eq!(step(&shown, &outs, Some(scripts), Key::ArrowRight), Step::Fold(scripts, true));
        assert_eq!(
            step(&shown, &outs, Some(areas), Key::ArrowRight),
            Step::To(TreeAt::Resource(a))
        );
        assert_eq!(step(&shown, &outs, at(b), Key::ArrowLeft), Step::To(areas));
        assert_eq!(step(&shown, &outs, Some(areas), Key::ArrowLeft), Step::Fold(areas, false));
        // Enter opens a resource, and opens or closes a group.
        assert_eq!(step(&shown, &outs, at(a), Key::Enter), Step::Open(a));
        // A letter: the next row that begins with it, round the end.
        let contents = HashMap::new();
        let by = |named: &dyn Fn(&ResKey) -> Option<String>, at, typed: &str| {
            seek(&shown, &outs, at, typed, &|r| row_text(r, named, &contents))
        };
        let none = |_: &ResKey| None;
        assert_eq!(by(&none, Some(areas), "s"), Some(scripts));
        assert_eq!(by(&none, Some(scripts), "A"), Some(areas));
        assert_eq!(by(&none, None, "c"), Some(TreeAt::Group("Conversations")));
        assert_eq!(by(&none, Some(areas), "z"), None);
        // Listed by name, a row is found by its name's letter, not its
        // ResRef's.
        let named = |k: &ResKey| (*k == b).then(|| "Zhentil Keep (b)".to_string());
        assert_eq!(by(&named, Some(areas), "z"), at(b));
        assert_eq!(by(&named, at(a), "b"), None);
        // Letters typed in quick succession: the row that begins with them
        // all, case and accents aside, a space among them.
        let names =
            |k: &ResKey| Some(if *k == a { "Shop" } else { "Shop Thing, Árbol" }.to_string());
        assert_eq!(by(&names, Some(areas), "sho"), at(a), "the first, itself first");
        assert_eq!(by(&names, Some(areas), "shopt"), None);
        assert_eq!(by(&names, Some(areas), "SHOP T"), at(b));
        assert_eq!(by(&names, Some(areas), "shop thing, arb"), at(b));
        assert_eq!(by(&names, at(a), "shop"), at(a));
        assert_eq!(by(&names, at(a), "ss"), at(b), "the same letter: the next");
        assert_eq!(step(&shown, &outs, Some(scripts), Key::Enter), Step::Fold(scripts, true));
        // A row that is gone (its group closed, a filter typed): the first.
        assert_eq!(step(&shown, &outs, at(c), Key::ArrowDown), Step::To(areas));
        assert_eq!(step(&Vec::new(), &outs, None, Key::ArrowDown), Step::None);
    }

    /// Right opens an area out and goes into it, to a kind and to its
    /// objects; Left comes back out, a row at a time (GitHub issue 12).
    #[test]
    fn the_keys_go_into_an_area_opened_out() {
        use egui::Key;
        use mg_area::ObjectKind::{Creature, Door};
        let key = |name: &str, t| ResKey::parse(name, t).unwrap();
        let (a, b) = (key("a", ResType::ARE), key("b", ResType::ARE));
        let shown: Shown = vec![("Areas", true, vec![a, b].into())];
        let (area, next) = (TreeAt::Resource(a), TreeAt::Resource(b));
        let (creatures, doors) = (TreeAt::Kind(a.resref, Creature), TreeAt::Kind(a.resref, Door));
        let object = |i| TreeAt::Object(a.resref, Creature, i);
        // Closed: Right opens it out; Left goes to its group.
        let mut outs = Outs::new();
        assert_eq!(step(&shown, &outs, Some(area), Key::ArrowRight), Step::Fold(area, true));
        assert_eq!(
            step(&shown, &outs, Some(area), Key::ArrowLeft),
            Step::To(TreeAt::Group("Areas"))
        );
        assert_eq!(step(&shown, &outs, Some(area), Key::ArrowDown), Step::To(next));
        // Opened out, its kinds closed.
        outs.insert(
            a.resref,
            vec![(Creature, false, Arc::from([0, 2])), (Door, false, Arc::from([0]))],
        );
        assert_eq!(step(&shown, &outs, Some(area), Key::ArrowRight), Step::To(creatures));
        assert_eq!(step(&shown, &outs, Some(area), Key::ArrowDown), Step::To(creatures));
        assert_eq!(step(&shown, &outs, Some(creatures), Key::ArrowDown), Step::To(doors));
        assert_eq!(step(&shown, &outs, Some(doors), Key::ArrowDown), Step::To(next));
        assert_eq!(
            step(&shown, &outs, Some(creatures), Key::ArrowRight),
            Step::Fold(creatures, true)
        );
        assert_eq!(step(&shown, &outs, Some(creatures), Key::ArrowLeft), Step::To(area));
        assert_eq!(step(&shown, &outs, Some(area), Key::ArrowLeft), Step::Fold(area, false));
        // A kind open: into its objects (those listed), and back.
        outs.insert(
            a.resref,
            vec![(Creature, true, Arc::from([0, 2])), (Door, false, Arc::from([0]))],
        );
        assert_eq!(step(&shown, &outs, Some(creatures), Key::ArrowRight), Step::To(object(0)));
        assert_eq!(step(&shown, &outs, Some(object(0)), Key::ArrowDown), Step::To(object(2)));
        assert_eq!(step(&shown, &outs, Some(object(2)), Key::ArrowDown), Step::To(doors));
        assert_eq!(step(&shown, &outs, Some(object(2)), Key::ArrowRight), Step::None);
        assert_eq!(step(&shown, &outs, Some(object(2)), Key::ArrowLeft), Step::To(creatures));
        assert_eq!(
            step(&shown, &outs, Some(creatures), Key::ArrowLeft),
            Step::Fold(creatures, false)
        );
        // Enter goes to an object in its area, and opens or closes a kind.
        assert_eq!(
            step(&shown, &outs, Some(object(2)), Key::Enter),
            Step::Go(a.resref, Creature, 2)
        );
        assert_eq!(step(&shown, &outs, Some(doors), Key::Enter), Step::Fold(doors, true));
        assert_eq!(step(&shown, &outs, Some(area), Key::Enter), Step::Open(a));
    }

    /// An area opened out and the placed objects of a kind open, for the
    /// keys: area `a`'s creatures Goblin (0) and Bandit (1), listed Bandit
    /// first, its door Gate in a kind closed and its waypoint by its tag
    /// WP_GATE; area `b` after it.
    fn opened_out() -> (Shown<'static>, Outs, HashMap<ResRef, (Read, AreaContents)>, ResKey, ResKey)
    {
        use mg_area::ObjectKind::{Creature, Door, Waypoint};
        let key = |name: &str| ResKey::parse(name, ResType::ARE).unwrap();
        let (a, b) = (key("a"), key("b"));
        let shown: Shown = vec![("Areas", true, vec![a, b].into())];
        let mut outs = Outs::new();
        outs.insert(
            a.resref,
            vec![
                (Creature, true, Arc::from([1, 0])),
                (Door, false, Arc::from([0])),
                (Waypoint, true, Arc::from([0])),
            ],
        );
        let row = |text: &str| ObjectRow {
            text: text.to_string(),
            sought: mg_core::text::fold(text),
            tip: String::new(),
            name: text.to_string(),
            tag: String::new(),
        };
        let kind = |kind, rows: Vec<ObjectRow>, order: Vec<usize>| KindContents {
            kind,
            rows,
            order: order.into(),
        };
        let contents = vec![
            kind(Creature, vec![row("Goblin"), row("Bandit")], vec![1, 0]),
            kind(Door, vec![row("Gate")], vec![0]),
            kind(Waypoint, vec![row("WP_GATE")], vec![0]),
        ];
        let contents = HashMap::from([(a.resref, (Read::Revision(0), contents))]);
        (shown, outs, contents, a, b)
    }

    /// Letters typed go to the rows in sight as they show, an area's kinds
    /// and placed objects among them: a kind by its name, an object by its
    /// name (a waypoint by its tag), in the order listed; not to one in a
    /// kind closed.
    #[test]
    fn letters_go_to_kinds_and_placed_objects_in_sight() {
        use mg_area::ObjectKind::{Creature, Door, Waypoint};
        let (shown, outs, contents, a, b) = opened_out();
        let none = |_: &ResKey| None;
        let by =
            |at, typed: &str| seek(&shown, &outs, at, typed, &|r| row_text(r, &none, &contents));
        let areas = Some(TreeAt::Group("Areas"));
        let object = |kind, i| Some(TreeAt::Object(a.resref, kind, i));
        assert_eq!(by(areas, "c"), Some(TreeAt::Kind(a.resref, Creature)));
        assert_eq!(by(areas, "goblin"), object(Creature, 0));
        assert_eq!(by(areas, "ba"), object(Creature, 1));
        // (The door is in a kind closed: its kind's row is found.)
        assert_eq!(by(areas, "g"), object(Creature, 0));
        assert_eq!(by(areas, "d"), Some(TreeAt::Kind(a.resref, Door)));
        assert_eq!(by(areas, "wp"), object(Waypoint, 0), "a waypoint by its tag");
        // From an object on, in the order listed, round the end.
        assert_eq!(by(object(Creature, 1), "w"), Some(TreeAt::Kind(a.resref, Waypoint)));
        let waypoints = Some(TreeAt::Kind(a.resref, Waypoint));
        assert_eq!(by(waypoints, "ww"), object(Waypoint, 0), "the same letter: the next");
        assert_eq!(by(object(Waypoint, 0), "b"), Some(TreeAt::Resource(b)));
        assert_eq!(by(Some(TreeAt::Resource(b)), "b"), object(Creature, 1));
    }

    /// What is kept of what the Filter finds ([`AreaNames::find`]) is what
    /// a look through every resource and every placed object finds, as the
    /// tree looked each frame: for each thing typed, typed again, after an
    /// area's objects are read again, with a tab's rows kept under the
    /// Filter and with the groups' lists made again.
    #[test]
    fn what_the_filter_keeps_is_what_a_look_through_finds() {
        use mg_area::ObjectKind::Creature;
        let (_, _, mut contents, a, b) = opened_out();
        let script = ResKey::parse("gate_open", ResType::NSS).unwrap();
        let names = HashMap::from([(b, "The Bandits' Camp".to_string())]);
        let mut lists: Vec<GroupList> = vec![(vec![a, b], names), (vec![script], HashMap::new())];
        type Held = HashMap<ResRef, (Read, AreaContents)>;
        let look = |lists: &[GroupList], filter: &Needle, contents: &Held, also: &[ResKey]| {
            let own = |k: &ResKey, names: &HashMap<ResKey, String>| {
                let name = names.get(k).map(|n| mg_core::text::fold(n)).unwrap_or_default();
                filter.finds_in(&[&k.to_string().to_ascii_lowercase(), &name])
            };
            let holds = |k: &ResKey| {
                k.restype == ResType::ARE
                    && contents.get(&k.resref).is_some_and(|(_, c)| {
                        c.iter().any(|c| c.rows.iter().any(|r| filter.finds_folded(&r.sought)))
                    })
            };
            (lists.iter())
                .map(|(all, names)| {
                    (all.iter().filter(|k| own(k, names) || holds(k) || also.contains(k)))
                        .copied()
                        .collect::<Vec<ResKey>>()
                })
                .collect::<Vec<_>>()
        };
        let mut kept = AreaNames::default();
        let mut shape = (1, 3, true, false);
        let typed = ["gate", "GOBLIN", "gate wp", "b", "bandit", "nothing", "gate open", "a"];
        for typed in typed {
            let filter = Needle::new(typed);
            for _ in 0..2 {
                kept.find(&filter, shape, &lists, &contents, None);
                assert_eq!(*kept.filtered.found, look(&lists, &filter, &contents, &[]), "{typed}");
            }
        }
        // The objects found, kind by kind in the order listed.
        let both: Option<Vec<Order>> = Some(vec![Arc::from([1, 0]), Arc::from([]), Arc::from([])]);
        assert_eq!(found_in(&contents[&a.resref].1, &Needle::new("b")), both);
        let filter = Needle::new("bandit");
        assert_eq!(found_in(&contents[&a.resref].1, &Needle::new("ogre")), None);
        assert_eq!(found_in(&contents[&a.resref].1, &Needle::new("")), None);
        kept.find(&filter, shape, &lists, &contents, None);
        assert_eq!(kept.filtered.found[0], [a, b]);
        assert!(kept.filtered.matched.contains(&b) && !kept.filtered.matched.contains(&a));
        // The area's objects read again, its creatures gone: found no more.
        let (_, held) = contents.get_mut(&a.resref).unwrap();
        held.retain(|c| c.kind != Creature);
        kept.read_again(a.resref);
        kept.find(&filter, shape, &lists, &contents, None);
        assert_eq!(*kept.filtered.found, look(&lists, &filter, &contents, &[]));
        assert_eq!(kept.filtered.found[0], [b]);
        // A tab's rows kept under the Filter, then let go.
        kept.find(&filter, shape, &lists, &contents, Some(&[script]));
        assert_eq!(*kept.filtered.found, look(&lists, &filter, &contents, &[script]));
        assert_eq!(kept.filtered.found[1], [script]);
        kept.find(&filter, shape, &lists, &contents, None);
        assert!(kept.filtered.found[1].is_empty());
        // The groups' lists made again (a resource renamed), and what the
        // areas hold let go (the game data read again).
        lists[0].1.insert(b, "The Camp".to_string());
        shape.0 += 1;
        kept.find(&filter, shape, &lists, &contents, None);
        assert!(kept.filtered.found[0].is_empty(), "{:?}", kept.filtered.found);
        let filter = Needle::new("gate");
        kept.find(&filter, shape, &lists, &contents, None);
        assert_eq!(kept.filtered.found[0], [a]);
        contents.clear();
        kept.find(&filter, shape, &lists, &contents, None);
        assert_eq!(*kept.filtered.found, look(&lists, &filter, &contents, &[]));
        assert!(kept.filtered.found[0].is_empty());
    }

    /// What was read from a resource's stored data is told apart by more
    /// than where the data is and how long: data at the same place and of
    /// the same length is another's once a command, its undo or a file
    /// read again changed the resource (the review's S26: replaced twice,
    /// it can come back to where the first was). Another resource's
    /// change leaves it.
    #[test]
    fn stored_data_is_told_apart_by_its_resource_s_changes_too() {
        let (a, b) =
            (ResKey::parse("a", ResType::NSS).unwrap(), ResKey::parse("b", ResType::NSS).unwrap());
        let mut m = mg_module::Module::new();
        m.set(a, b"// one".to_vec());
        m.set(b, b"// two".to_vec());
        let mut ws = Workspace::new(m);
        // (One place and length throughout, as data come back to it.)
        let data = b"// one";
        let first = stored_stamp(&ws, &a, data);
        assert_eq!(stored_stamp(&ws, &a, data), first);
        let set = |key, text: &[u8]| {
            let edit = mg_edit::Edit::SetResource { key, data: Some(text.to_vec()) };
            mg_edit::Command::new("edit", vec![edit])
        };
        ws.apply(set(b, b"// 2")).unwrap();
        assert_eq!(stored_stamp(&ws, &a, data), first, "another's change");
        ws.apply(set(a, b"// uno")).unwrap();
        let second = stored_stamp(&ws, &a, data);
        assert_ne!(second, first, "changed, at the same place and length");
        ws.undo().unwrap();
        assert_ne!(stored_stamp(&ws, &a, data), second, "and undone");
        assert_ne!(stored_stamp(&ws, &a, &data[..5]), stored_stamp(&ws, &a, data));
    }

    /// Deleted by Delete, a placed object the keys were at leaves them at
    /// the row after it among its kind's, else the one before, else at the
    /// kind's; at another of its kind's after it, they follow it up its
    /// area's list.
    #[test]
    fn the_keys_go_past_a_placed_object_deleted() {
        use mg_area::ObjectKind::{Creature, Waypoint};
        let (shown, outs, _, a, _) = opened_out();
        let object = |kind, i| TreeAt::Object(a.resref, kind, i);
        let (bandit, goblin) = (object(Creature, 1), object(Creature, 0));
        // Bandit, listed first: Goblin after it (still 0).
        assert_eq!(past_deleted(&shown, &outs, bandit, bandit), goblin);
        // Goblin, listed last: Bandit before it, now 0.
        assert_eq!(past_deleted(&shown, &outs, goblin, goblin), object(Creature, 0));
        // The only waypoint: its kind.
        let wp = object(Waypoint, 0);
        assert_eq!(past_deleted(&shown, &outs, wp, wp), TreeAt::Kind(a.resref, Waypoint));
        // The keys at Bandit while Goblin is deleted: Bandit moves up.
        assert_eq!(past_deleted(&shown, &outs, bandit, goblin), object(Creature, 0));
        assert_eq!(past_deleted(&shown, &outs, goblin, bandit), goblin);
        let group = TreeAt::Group("Areas");
        assert_eq!(past_deleted(&shown, &outs, group, goblin), group);
    }

    /// Gone from its group, a resource the keys were at leaves them at the
    /// next row its group listed that is there still (past several gone
    /// together), else the one before (the last was gone), else none (the
    /// group's row).
    #[test]
    fn the_keys_go_past_a_resource_deleted() {
        let k = |n: &str| ResKey::parse(n, ResType::NSS).unwrap();
        let was = [k("a"), k("b"), k("c"), k("d")];
        assert_eq!(past_deleted_resource(&was, &[k("a"), k("c"), k("d")], k("b")), Some(k("c")));
        // Two together, the keys at the first: past both.
        assert_eq!(past_deleted_resource(&was, &[k("a"), k("d")], k("b")), Some(k("d")));
        // The last: the one before it.
        assert_eq!(past_deleted_resource(&was, &[k("a"), k("b"), k("c")], k("d")), Some(k("c")));
        assert_eq!(past_deleted_resource(&was, &[k("a")], k("c")), Some(k("a")));
        // The only one: none (its group's row).
        assert_eq!(past_deleted_resource(&[k("a")], &[], k("a")), None);
    }

    #[test]
    fn each_group_of_resources_has_its_new_command() {
        use crate::commands::Id;
        let label = |types: &[ResType]| new_command(types).map(|id| new_label(&id.name()));
        assert_eq!(label(&[ResType::ARE]).as_deref(), Some("New Area…"));
        assert_eq!(label(&[ResType::UTC]).as_deref(), Some("New Creature…"));
        assert_eq!(label(&[ResType::UTM]).as_deref(), Some("New Store…"));
        assert_eq!(label(&[ResType::NSS]).as_deref(), Some("New Script…"));
        assert_eq!(label(&[ResType::DLG]).as_deref(), Some("New Conversation…"));
        // The journal and factions are one of each: nothing to make.
        assert_eq!(new_command(&[ResType::JRL, ResType::FAC]), None);
        // Every group of blueprints has one.
        for (_, types) in GROUPS.iter().filter(|(n, _)| *n != "Journal and factions") {
            assert!(new_command(types).is_some(), "{types:?}");
        }
        // Each kind of blueprint is a group by the name the toolset calls
        // it by, and its New… is its wizard's, as the palettes' is.
        for k in ObjectKind::ALL {
            assert!(GROUPS.contains(&(k.label(), &[k.restype()])), "{k:?}");
            assert_eq!(new_command(&[k.restype()]), Some(Id::new_blueprint(k)));
        }
        assert_eq!(GROUPS.iter().filter(|(n, _)| n.is_empty()).count(), 0);
        assert_eq!(label(&[ResType::UTW]).as_deref(), Some("New Waypoint…"));
        assert_eq!(
            new_command(&[ResType::UTI]),
            Some(Id::Wizard(mg_module::palette::BlueprintKind::Item))
        );
    }
}
