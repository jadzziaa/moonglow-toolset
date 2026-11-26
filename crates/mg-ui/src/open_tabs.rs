//! A module's open tabs, from one session to the next (Options › General:
//! Open a module with the tabs it had): the areas, scripts, blueprints,
//! conversations, side editors and tools that were open when it was
//! closed open again with it, in their order and their places (the
//! middle, a pane split off the middle, a window of their own where it
//! was, split as it was), each pane's tab in front in
//! front, and each view as it was left: an area's camera, a model view's,
//! a script's scroll and cursor, a conversation's selected line,
//! bookmarks and folded branches, a Properties window's page, a hak's
//! selected resources, a 2DA's selected cell, a tileset's page and tile.
//!
//! They are kept in Moonglow's data folder, never in the module: a small
//! JSON file a module (`open-tabs/<name>-<hash>.json`, by its canonical
//! path, a module folder's or a project's by its folder), written when its
//! tabs change and a second after a window was last moved or resized (so
//! that a crash loses neither), when the module is closed and when
//! Moonglow quits. What each view shows is read from it then, not each
//! frame. A tab whose resource or file is gone is passed over, said in the
//! debug log. An area's view is made when its tab is first drawn, so the
//! areas behind the one in front cost nothing until they are shown.

use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use egui_dock::{NodeIndex, NodePath, Split as DockSplit, Tree};
use mg_core::{ResRef, ResType};
use mg_edit::{GffPath, Step};
use mg_module::dialog::{Kind, Parent};
use mg_resman::ResKey;
use serde::{Deserialize, Serialize};

use crate::dialog_view::{DialogView, Row};
use crate::references::Query;
use crate::table_view::Home;
use crate::{Action, Moonglow, Place, Tab, tabs, trace};

/// A module's tabs as they are kept.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Record {
    /// The panes with tabs to keep, in the order they lie (the main
    /// window's, then the windows').
    panes: Vec<Pane>,
    /// The tab in front of the pane that had the keys.
    #[serde(skip_serializing_if = "Option::is_none")]
    front: Option<String>,
    /// What each tab's view showed, by the tab's text.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    views: BTreeMap<String, View>,
    /// The placed objects whose Properties are open, by `area:path`: what
    /// finds each again in its area's list if the list changed since.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    objects: BTreeMap<String, Placed>,
    /// Whether its windows' rectangles were taken with the theme's rows
    /// (22 points): absent (false) in a record from before the theme,
    /// whose windows' heights are made as much taller as their kind's
    /// windows are now as it is read (`Tab::taller`), once; this version
    /// writes it. (4.1.0 passes over it.)
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    rects_scaled: bool,
}

/// A pane's tabs (a window's: all its panes'), as [`Moonglow::tab_text`]
/// names them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Pane {
    /// `middle`, `split left 300` (the side of the middle, and thousandths
    /// of the two) or `window`. (`palettes`, the palettes' pane's, written
    /// before they were the Right Dock's panel, is read as "where its kind
    /// opens".)
    place: String,
    tabs: Vec<String>,
    /// The tab in front.
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<String>,
    /// A window's place and size: left, top, width, height in points.
    #[serde(skip_serializing_if = "Option::is_none")]
    rect: Option<[f32; 4]>,
    /// A window of several panes: how they lie (`tabs` has all of theirs,
    /// in that order).
    #[serde(skip_serializing_if = "Option::is_none")]
    split: Option<Split>,
}

/// A window's panes: a pane's tabs, or two parts beside each other or one
/// above the other.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Split {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tabs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<String>,
    /// `left-right` or `above-below`, for two parts.
    #[serde(skip_serializing_if = "String::is_empty")]
    way: String,
    /// The first part's share, thousandths.
    #[serde(skip_serializing_if = "is_zero")]
    share: u16,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parts: Vec<Split>,
}

fn is_zero(n: &u16) -> bool {
    *n == 0
}

impl Split {
    /// Its panes' tabs, the first part's first.
    fn flat(&self) -> Vec<String> {
        let parts = self.parts.iter().flat_map(Split::flat);
        self.tabs.iter().cloned().chain(parts).collect()
    }
}

/// What a tab's view showed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct View {
    /// An area's camera (or a model view's): the point it looks at, then
    /// its yaw, pitch and distance; and whether the view showed the night.
    #[serde(skip_serializing_if = "Option::is_none")]
    camera: Option<[f32; 6]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    night: Option<bool>,
    /// A script's scroll (points) and its cursor (characters: start, end).
    #[serde(skip_serializing_if = "Option::is_none")]
    scroll: Option<[f32; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<[usize; 2]>,
    /// A script's bookmarked lines (from 0), and its numbered bookmarks
    /// (Ctrl and a digit): each digit and its line.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lines: Vec<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    numbered: Vec<[usize; 2]>,
    /// The talk table's line chosen (its StrRef in the table).
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    /// A Properties window's page (or the Options'), by its name.
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<String>,
    /// A conversation as it was (its fingerprint): what follows is kept
    /// by line number, so it is let go if the conversation is another now.
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<u64>,
    /// The selected row (`root 0`, `entry 3 1`: its parent and place
    /// there), the bookmarked lines and the folded ones (`reply 5`).
    #[serde(skip_serializing_if = "Option::is_none")]
    selected: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    bookmarks: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    folded: Vec<String>,
    /// A hak's selected resources (`name.ext`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    chosen: Vec<String>,
    /// A 2DA's selected cell: its row and column.
    #[serde(skip_serializing_if = "Option::is_none")]
    cell: Option<[usize; 2]>,
    /// A tileset's chosen tile and group (its page is `page`).
    #[serde(skip_serializing_if = "Option::is_none")]
    tile: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    group: Option<usize>,
    /// Advanced Search's question.
    #[serde(skip_serializing_if = "Option::is_none")]
    search: Option<crate::search::KeptSearch>,
}

/// A placed object, as it is found again: its tag, its blueprint and
/// where it stands (its kind is its list's).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Placed {
    tag: String,
    resref: String,
    at: [f32; 3],
}

/// A kept tab, before it is opened: most are a [`Tab`] as they are; the
/// side editors' are files to open (their tabs are by a session's ids),
/// and some tools have what they showed to set up.
#[derive(Debug, Clone, PartialEq)]
enum Want {
    Tab(Tab),
    Hak(PathBuf),
    /// A 2DA: the module's, a file's, or one in a hak (the hak's file).
    Table(TableAt),
    Tileset(PathBuf),
    References(Option<Query>),
    /// Tile Properties of these tiles (by index in the area's tile list).
    TileProperties(ResRef, Vec<usize>),
    /// The Talk Table tab showing the game's `dialog.tlk`: a record of
    /// U17's own tab for it (`game-talk-table`), which the owner's ruling
    /// folded into the Talk Table tab. Nothing writes it now: the tab
    /// remembers it showed `dialog.tlk` no more than a file it showed by
    /// Open File….
    GameTalk,
}

#[derive(Debug, Clone, PartialEq)]
enum TableAt {
    Module(ResKey),
    File(PathBuf),
    Hak(PathBuf, ResKey),
}

/// A kept pane, its tabs found again.
#[derive(Debug, Clone)]
struct Kept {
    /// `None`: a place not understood; the tabs open where their kind does.
    place: Option<Place>,
    tabs: Vec<(String, Want)>,
    active: Option<String>,
    rect: Option<egui::Rect>,
    split: Option<Split>,
}

/// The kept tabs, read and waiting for the module's first frame: the
/// panes, the tab in front, and what the views showed (by tab text).
#[derive(Debug)]
struct Pending {
    panes: Vec<Kept>,
    front: Option<String>,
    views: BTreeMap<String, View>,
}

/// An older record's Tile Properties tab, waiting for its area's view:
/// its tiles (by index in the area's tile list) selected in Select Tiles
/// then ([`Moonglow::tiles_again`]).
#[derive(Debug, Clone)]
struct TilesWaiting {
    area: ResRef,
    tiles: Vec<usize>,
}

/// The open module's tabs as kept and as they are now.
#[derive(Debug, Default)]
pub(crate) struct OpenTabs {
    /// The tabs kept, to open once the module's first frame has drawn
    /// its panes, the one in front, and what
    /// the views showed and the placed objects were.
    pending: Option<Pending>,
    /// What the views opened again showed, until each is first drawn and
    /// takes its own (written as it is until then).
    views: HashMap<Tab, View>,
    /// The placed objects of the tabs opened again, until their area is
    /// read: what the record says of them meanwhile.
    objects: HashMap<(ResRef, GffPath), Placed>,
    tiles: Option<TilesWaiting>,
    /// The panes and their tabs at the last frame, hashed: a change is
    /// written.
    shape: u64,
    /// Where the windows were at the last frame, hashed, and when they
    /// were last seen to move: written a second after they stop.
    places: u64,
    moved: Option<f64>,
    /// What was last written (or read): not written again unchanged.
    written: Option<String>,
    /// A failure to write was said in the log (once).
    told: bool,
}

impl OpenTabs {
    /// What an area's view (or a script's) showed when its tab was kept,
    /// once: the view takes it as it is first drawn.
    pub(crate) fn take_view(&mut self, tab: &Tab) -> Option<View> {
        if self.views.is_empty() {
            return None;
        }
        self.views.remove(tab)
    }

    /// The page a Properties window `tab` showed when it was kept, of
    /// `pages` (once).
    pub(crate) fn page(&mut self, tab: &Tab, pages: &[&'static str]) -> Option<&'static str> {
        let page = self.take_view(tab)?.page?;
        pages.iter().copied().find(|p| *p == page)
    }
}

impl View {
    /// An area's camera kept: the orbit, and whether it showed the night.
    pub(crate) fn orbit(&self) -> Option<(crate::area_view::Orbit, Option<bool>)> {
        let [x, y, z, yaw, pitch, distance] = self.camera?;
        let ok = [x, y, z, yaw, pitch, distance].iter().all(|v| v.is_finite()) && distance > 0.0;
        let target = glam::Vec3::new(x, y, z);
        ok.then_some((crate::area_view::Orbit { target, yaw, pitch, distance }, self.night))
    }

    /// A script's scroll and cursor kept.
    pub(crate) fn script(&self) -> (Option<egui::Vec2>, Option<(usize, usize)>) {
        (self.scroll.map(|[x, y]| egui::vec2(x, y)), self.cursor.map(|[a, b]| (a, b)))
    }

    /// The page kept (a NUI source group's switch: M4's U19).
    pub(crate) fn page(&self) -> Option<&str> {
        self.page.as_deref()
    }

    /// The talk table's line kept.
    pub(crate) fn talk_line(&self) -> Option<usize> {
        self.line
    }

    /// A script's bookmarks kept: its bookmarked lines and its numbered
    /// ones, those within its `lines` lines (its text may have changed
    /// since).
    pub(crate) fn script_marks(
        &self,
        lines: usize,
    ) -> (std::collections::BTreeSet<usize>, [Option<usize>; 10]) {
        let marked = self.lines.iter().copied().filter(|l| *l < lines).collect();
        let mut numbered = [None; 10];
        for &[n, line] in &self.numbered {
            if let Some(slot) = numbered.get_mut(n).filter(|_| line < lines) {
                *slot = Some(line);
            }
        }
        (marked, numbered)
    }
}

/// The file in `dir` that keeps the tabs of the module at `module` (named
/// as the locks' files are, by its canonical path).
pub fn file(dir: &Path, module: &Path) -> PathBuf {
    mg_module::locks::file(dir, module)
}

fn key_text(key: &ResKey) -> String {
    key.to_string()
}

fn parse_key(text: &str) -> Option<ResKey> {
    let (name, ext) = text.rsplit_once('.')?;
    ResKey::parse(name, ResType::from_extension(ext)?)
}

fn parse_resref(text: &str) -> Option<ResRef> {
    ResRef::from_str(text).ok()
}

/// A path to a struct in a GFF, as every path is written:
/// `/Creature List[3]`.
fn path_text(path: &GffPath) -> String {
    path.to_string()
}

/// A path as [`path_text`] writes it, or as Moonglow 3.1.0 and before kept
/// it (`Creature List#3`, steps joined by `/`, the root empty): a file of
/// theirs is read, and written anew in the one form.
fn parse_path(text: &str) -> Option<GffPath> {
    if text.starts_with('/') {
        return text.parse().ok();
    }
    if text.is_empty() {
        return Some(GffPath::root());
    }
    let steps = text.split('/').map(|s| match s.rsplit_once('#') {
        Some((label, index)) => index.parse().ok().map(|i| Step::Item(label.to_string(), i)),
        None => Some(Step::Field(s.to_string())),
    });
    steps.collect::<Option<Vec<_>>>().map(GffPath)
}

/// An object placed in an area, as `area:path`.
fn object_text(area: ResRef, path: &GffPath) -> String {
    format!("{area}:{}", path_text(path))
}

/// An `area:path` key of the objects kept, its path in the one form (a
/// file 3.1.0 and before wrote has them as they wrote paths).
fn object_key(text: &str) -> String {
    let Some((area, path)) = text.split_once(':') else { return text.to_string() };
    match parse_path(path) {
        Some(p) => format!("{area}:{}", path_text(&p)),
        None => text.to_string(),
    }
}

fn list<T>(items: &[T], text: impl Fn(&T) -> String) -> String {
    items.iter().map(text).collect::<Vec<_>>().join(",")
}

fn parse_list<T>(text: &str, parse: impl Fn(&str) -> Option<T>) -> Option<Vec<T>> {
    let items = text.split(',').map(parse).collect::<Option<Vec<_>>>()?;
    (!items.is_empty()).then_some(items)
}

fn line_text((kind, index): (Kind, u32)) -> String {
    let kind = match kind {
        Kind::Entry => "entry",
        Kind::Reply => "reply",
    };
    format!("{kind} {index}")
}

fn parse_line(text: &str) -> Option<(Kind, u32)> {
    let (kind, index) = text.split_once(' ')?;
    let kind = match kind {
        "entry" => Kind::Entry,
        "reply" => Kind::Reply,
        _ => return None,
    };
    Some((kind, index.parse().ok()?))
}

fn row_text(row: Row) -> String {
    match row.parent {
        Parent::Root => format!("root {}", row.pos),
        Parent::Node(k, i) => format!("{} {}", line_text((k, i)), row.pos),
    }
}

fn parse_row(text: &str) -> Option<Row> {
    let (parent, pos) = text.rsplit_once(' ')?;
    let parent = match parent {
        "root" => Parent::Root,
        line => {
            let (k, i) = parse_line(line)?;
            Parent::Node(k, i)
        }
    };
    Some(Row { parent, pos: pos.parse().ok()? })
}

/// How a tab that is a [`Tab`] as it is, is kept; `None` for those that
/// are not: the start page, the pane that says no area is open, what
/// shows the result of a search, and those whose text needs the app
/// ([`Moonglow::tab_text`]).
fn plain_text(tab: &Tab) -> Option<String> {
    Some(match tab {
        Tab::ModuleProperties => "module-properties".into(),
        Tab::Factions => "factions".into(),
        Tab::Journal => "journal".into(),
        Tab::TalkTable => "talk-table".into(),
        Tab::Manual => "manual".into(),
        Tab::AdvancedSearch => "advanced-search".into(),
        Tab::Options => "options".into(),
        Tab::PlaceableGallery => "placeable-gallery".into(),
        Tab::Plugins => "plugins".into(),
        Tab::Script(k) => format!("script:{}", key_text(k)),
        Tab::Nui(k) => format!("nui:{}", key_text(k)),
        Tab::NuiSource(k) => format!("nui-source:{}", key_text(k)),
        Tab::Gff(k) => format!("gff:{}", key_text(k)),
        Tab::Dialog(k) => format!("dialog:{}", key_text(k)),
        Tab::Resource(k) => format!("resource:{}", key_text(k)),
        Tab::ResourceCopy { key, layer } => format!("resource-copy:{}:{layer}", key_text(key)),
        Tab::ModelCopy { key, layer } => format!("model-copy:{}:{layer}", key_text(key)),
        Tab::Model(k) => format!("model:{}", key_text(k)),
        Tab::Blueprint(k) => format!("blueprint:{}", key_text(k)),
        Tab::Blueprints(keys) => format!("blueprints:{}", list(keys, key_text)),
        Tab::Area(r) => format!("area:{r}"),
        Tab::AreaProperties(r) => format!("area-properties:{r}"),
        Tab::AreasProperties(rs) => format!("areas-properties:{}", list(rs, |r| r.to_string())),
        Tab::Instance { area, path } => format!("instance:{area}:{}", path_text(path)),
        Tab::InstanceModel { area, path } => {
            format!("instance-model:{area}:{}", path_text(path))
        }
        // (Of one area as before; of several, each object its area's.)
        Tab::Instances { objects } => match Tab::one_area(objects) {
            Some(area) => format!("instances:{area}:{}", list(objects, |(_, p)| path_text(p))),
            None => format!("instances-of-areas:{}", list(objects, |(a, p)| object_text(*a, p))),
        },
        Tab::Tile { tileset, tile } => format!("tile:{tileset}:{tile}"),
        Tab::PluginPanel { plugin, panel } => format!("plugin-panel:{plugin}/{panel}"),
        Tab::Welcome
        | Tab::NoArea
        | Tab::References
        | Tab::Hak(_)
        | Tab::Table(_)
        | Tab::Tileset(_) => return None,
    })
}

/// How the resource browser's tab was kept, before the browser was the
/// Left Dock's Resources panel (the redesign's M3 T4): a record that has
/// it shows the panel ([`Moonglow::read_open_tabs`]); nothing writes it.
const RESOURCES_TAB: &str = "resources";

/// The palettes' tab, before the palettes were the Right Dock's Palette
/// panel (M3 T5). No Moonglow wrote it (the palettes were shown or folded
/// for every module, not kept with a module's tabs); a record that has it
/// shows the panel all the same, and nothing writes it.
const PALETTE_TAB: &str = "palette";

/// The tab a text names.
fn parse_tab(text: &str) -> Option<Want> {
    let (kind, rest) = text.split_once(':').unwrap_or((text, ""));
    let area_and = |rest: &str| {
        let (area, rest) = rest.split_once(':')?;
        Some((parse_resref(area)?, rest.to_string()))
    };
    let file = |rest: &str| (!rest.is_empty()).then(|| PathBuf::from(rest));
    Some(Want::Tab(match kind {
        "module-properties" => Tab::ModuleProperties,
        "factions" => Tab::Factions,
        "journal" => Tab::Journal,
        "talk-table" => Tab::TalkTable,
        // (The game's dialog.tlk in a tab of its own, as U17 first kept it
        // until the owner's ruling: the Talk Table tab, showing it.)
        "game-talk-table" => return Some(Want::GameTalk),
        "manual" => Tab::Manual,
        "advanced-search" => Tab::AdvancedSearch,
        "options" => Tab::Options,
        "placeable-gallery" => Tab::PlaceableGallery,
        "plugins" => Tab::Plugins,
        "plugin-panel" => {
            let (plugin, panel) = rest.split_once('/')?;
            Tab::PluginPanel { plugin: plugin.to_string(), panel: panel.to_string() }
        }
        "script" => Tab::Script(parse_key(rest)?),
        "nui" => Tab::Nui(parse_key(rest)?),
        "nui-source" => Tab::NuiSource(parse_key(rest)?),
        "gff" => Tab::Gff(parse_key(rest)?),
        "dialog" => Tab::Dialog(parse_key(rest)?),
        "resource" => Tab::Resource(parse_key(rest)?),
        "resource-copy" => {
            let (key, layer) = rest.split_once(':')?;
            Tab::ResourceCopy { key: parse_key(key)?, layer: layer.to_string() }
        }
        "model" => Tab::Model(parse_key(rest)?),
        "model-copy" => {
            let (key, layer) = rest.split_once(':')?;
            Tab::ModelCopy { key: parse_key(key)?, layer: layer.to_string() }
        }
        "blueprint" => Tab::Blueprint(parse_key(rest)?),
        "blueprints" => Tab::Blueprints(parse_list(rest, parse_key)?),
        "area" => Tab::Area(parse_resref(rest)?),
        "area-properties" => Tab::AreaProperties(parse_resref(rest)?),
        "areas-properties" => Tab::AreasProperties(parse_list(rest, parse_resref)?),
        "instance" => {
            let (area, path) = area_and(rest)?;
            Tab::Instance { area, path: parse_path(&path)? }
        }
        "instance-model" => {
            let (area, path) = area_and(rest)?;
            Tab::InstanceModel { area, path: parse_path(&path)? }
        }
        "instances" => {
            let (area, paths) = area_and(rest)?;
            Tab::instances(area, parse_list(&paths, parse_path)?)
        }
        "instances-of-areas" => Tab::Instances {
            objects: parse_list(rest, |o| {
                let (area, path) = area_and(o)?;
                Some((area, parse_path(&path)?))
            })?
            .into(),
        },
        "tile" => {
            let (tileset, tile) = area_and(rest)?;
            Tab::Tile { tileset, tile: tile.parse().ok()? }
        }
        "hak" => return Some(Want::Hak(file(rest)?)),
        "tileset" => return Some(Want::Tileset(file(rest)?)),
        "table" => return Some(Want::Table(TableAt::Module(parse_key(rest)?))),
        "table-file" => return Some(Want::Table(TableAt::File(file(rest)?))),
        "table-hak" => {
            let (key, hak) = rest.split_once(':')?;
            return Some(Want::Table(TableAt::Hak(file(hak)?, parse_key(key)?)));
        }
        "references" => {
            let query = match rest.split_once(':') {
                None if rest.is_empty() => None,
                Some(("resource", key)) => Some(Query::Resource(parse_key(key)?)),
                Some(("tag", tag)) => Some(Query::Tag(tag.to_string())),
                Some(("strref", n)) => Some(Query::StrRef(n.parse().ok()?)),
                _ => return None,
            };
            return Some(Want::References(query));
        }
        "tile-properties" => {
            let (area, tiles) = area_and(rest)?;
            return Some(Want::TileProperties(area, parse_list(&tiles, |t| t.parse().ok())?));
        }
        _ => return None,
    }))
}

/// A placed object as it is found again (`None`: a list of no kind of
/// object, found by its place alone).
fn placed(list: &str, index: usize, s: &mg_gff::Struct) -> Option<Placed> {
    let kind = mg_area::ObjectKind::from_list(list)?;
    let at = mg_area::AreaObject::placed(kind, index, s).position;
    Some(Placed {
        tag: crate::text::decode(s.string("Tag").unwrap_or_default()),
        resref: s.resref(kind.resref_field()).map(|r| r.to_string()).unwrap_or_default(),
        at: at.to_array(),
    })
}

/// Where the object a kept tab's `path` named is in the area's GIT now:
/// the one there if it is the same (its tag, blueprint and place), else
/// the one that is (the nearest such to its old place), else the only one
/// of its tag and blueprint (moved since); `None` if none is. Kept without
/// what it was (an older record), or a path deeper than a list's item, by
/// its place alone.
fn found_again(git: &mg_gff::Gff, path: &GffPath, kept: Option<&Placed>) -> Option<GffPath> {
    let (Some(kept), [Step::Item(list, index)]) = (kept, path.0.as_slice()) else {
        return path.get(&git.root).is_some().then(|| path.clone());
    };
    let items = git.root.list(list)?;
    let near = |a: &[f32; 3], b: &[f32; 3]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.01);
    // (A record written before 5.2.0 kept the tag read as UTF-8 and the
    // blueprint as `TemplateResRef`, empty for a store: taken as well.)
    let old = |s: &mg_gff::Struct| {
        let tag = String::from_utf8_lossy(s.string("Tag").unwrap_or_default()).into_owned();
        (tag, s.resref("TemplateResRef").map(|r| r.to_string()).unwrap_or_default())
    };
    let like = |i: usize, exact: bool| {
        placed(list, i, &items[i]).is_some_and(|p| {
            let (old_tag, old_resref) = old(&items[i]);
            (p.tag == kept.tag || old_tag == kept.tag)
                && (p.resref == kept.resref || old_resref == kept.resref)
                && (!exact || near(&p.at, &kept.at))
        })
    };
    let item = |i: usize| Some(GffPath::root().item(list, i));
    let same: Vec<usize> = (0..items.len()).filter(|&i| like(i, true)).collect();
    if let Some(&i) = same.iter().min_by_key(|i| i.abs_diff(*index)) {
        return item(i);
    }
    match (0..items.len()).filter(|&i| like(i, false)).collect::<Vec<_>>()[..] {
        [i] => item(i),
        _ => None,
    }
}

/// The window's panes from `at` down, as they are kept (`None`: no tabs
/// to keep there).
fn split_of(app: &Moonglow, tree: &Tree<Tab>, at: NodeIndex) -> Option<Split> {
    use egui_dock::Node;
    if at.0 >= tree.len() {
        return None;
    }
    match &tree[at] {
        Node::Empty => None,
        Node::Leaf(leaf) => {
            let tabs: Vec<String> = leaf.tabs.iter().filter_map(|t| app.tab_text(t)).collect();
            let active = leaf.tabs.get(leaf.active.0).and_then(|t| app.tab_text(t));
            (!tabs.is_empty()).then_some(Split { tabs, active, ..Default::default() })
        }
        Node::Horizontal(s) | Node::Vertical(s) => {
            let way =
                if matches!(tree[at], Node::Vertical(_)) { "above-below" } else { "left-right" };
            match (split_of(app, tree, at.left()), split_of(app, tree, at.right())) {
                (Some(a), Some(b)) => Some(Split {
                    way: way.into(),
                    share: (s.fraction * 1000.0).round().clamp(1.0, 999.0) as u16,
                    parts: vec![a, b],
                    ..Default::default()
                }),
                (one, None) | (None, one) => one,
            }
        }
    }
}

/// A kept window's panes with their tabs found again: a pane's tabs and
/// the one in front, or two parts (beside each other, or `above` one
/// another) and the first's share. Panes left with no tab go.
#[derive(Debug)]
enum Laid {
    Pane(Vec<Tab>, Option<Tab>),
    Parts(bool, f32, Box<Laid>, Box<Laid>),
}

impl Laid {
    fn of(split: &Split, opened: &HashMap<String, Tab>) -> Option<Laid> {
        if let [a, b] = split.parts.as_slice() {
            return match (Laid::of(a, opened), Laid::of(b, opened)) {
                (Some(a), Some(b)) => {
                    let share = (f32::from(split.share) / 1000.0).clamp(0.05, 0.95);
                    Some(Laid::Parts(split.way == "above-below", share, a.into(), b.into()))
                }
                (one, None) | (None, one) => one,
            };
        }
        let tabs: Vec<Tab> = split.tabs.iter().filter_map(|t| opened.get(t).cloned()).collect();
        let active = split.active.as_ref().and_then(|t| opened.get(t).cloned());
        (!tabs.is_empty()).then_some(Laid::Pane(tabs, active))
    }

    fn first(&self) -> &Tab {
        match self {
            Laid::Pane(tabs, _) => &tabs[0],
            Laid::Parts(_, _, a, _) => a.first(),
        }
    }

    fn tabs(&self) -> Vec<Tab> {
        match self {
            Laid::Pane(tabs, _) => tabs.clone(),
            Laid::Parts(_, _, a, b) => [a.tabs(), b.tabs()].concat(),
        }
    }

    /// Lays the panes out in the window's tree from `at`, a pane that has
    /// the first tab already.
    fn build(&self, tree: &mut Tree<Tab>, at: NodeIndex) {
        match self {
            Laid::Pane(tabs, active) => {
                let Ok(leaf) = tree.leaf_mut(at) else { return };
                for tab in tabs {
                    if !leaf.tabs.contains(tab) {
                        leaf.tabs.push(tab.clone());
                    }
                }
                let front = active.as_ref().and_then(|a| leaf.tabs.iter().position(|t| t == a));
                leaf.active = egui_dock::TabIndex(front.unwrap_or(0));
            }
            Laid::Parts(above, share, a, b) => {
                let way = if *above { DockSplit::Below } else { DockSplit::Right };
                let new = egui_dock::Node::leaf(b.first().clone());
                let [first, second] = tree.split(at, way, *share, new);
                a.build(tree, first);
                b.build(tree, second);
            }
        }
    }
}

impl Moonglow {
    /// Whether the open module's tabs are kept: it has a file, Moonglow a
    /// data folder, and Options › General keeps them.
    fn tabs_file(&self) -> Option<PathBuf> {
        if self.settings.no_reopen_tabs {
            return None;
        }
        Some(file(self.open_tabs_dir.as_deref()?, &self.module_path()?))
    }

    /// How a tab is kept; `None` for one that is not (see [`plain_text`]),
    /// a hak never saved, and a 2DA in one.
    fn tab_text(&self, tab: &Tab) -> Option<String> {
        let hak = |id: u32| self.haks.iter().find(|d| d.id == id)?.hak.path.clone();
        Some(match tab {
            Tab::Hak(id) => format!("hak:{}", hak(*id)?.display()),
            Tab::Tileset(id) => {
                format!("tileset:{}", self.tilesets.iter().find(|d| d.id == *id)?.path.display())
            }
            Tab::Table(id) => {
                let doc = self.tables.iter().find(|d| d.id == *id)?;
                match &doc.home {
                    Home::Module => format!("table:{}", key_text(&doc.key)),
                    Home::File(path) => format!("table-file:{}", path.display()),
                    Home::Hak(h) => {
                        format!("table-hak:{}:{}", key_text(&doc.key), hak(*h)?.display())
                    }
                }
            }
            Tab::References => match &self.references.query {
                None => "references".into(),
                Some(Query::Resource(k)) => format!("references:resource:{}", key_text(k)),
                Some(Query::Tag(t)) => format!("references:tag:{t}"),
                Some(Query::StrRef(n)) => format!("references:strref:{n}"),
            },
            tab => plain_text(tab)?,
        })
    }

    /// Whether a kept tab can be shown still: its resource is in the
    /// module (a read-only one's, in the load order), a placed object's
    /// in its area, a side editor's file where it was.
    fn tab_there(&self, want: &Want) -> bool {
        let Some(ws) = &self.ws else { return false };
        let module = &ws.module;
        let game = self.game.as_deref();
        let area = |a: &ResRef| module.contains(&ResKey::new(*a, ResType::ARE));
        let tab = match want {
            Want::Hak(path) | Want::Tileset(path) => return path.is_file(),
            Want::Table(TableAt::Module(key)) => return module.contains(key),
            Want::Table(TableAt::File(path) | TableAt::Hak(path, _)) => return path.is_file(),
            Want::References(_) => return true,
            Want::TileProperties(a, _) => return area(a),
            Want::GameTalk => return true,
            Want::Tab(tab) => tab,
        };
        match tab {
            Tab::Resource(k) | Tab::Model(k) => {
                module.contains(k) || game.is_some_and(|g| g.resman.contains(k))
            }
            Tab::ResourceCopy { key, layer } | Tab::ModelCopy { key, layer } => {
                game.is_some_and(|g| {
                    let rm = &g.resman;
                    (rm.layers().iter().position(|l| l.label == *layer))
                        .is_some_and(|i| rm.get_in(i, key).is_ok())
                })
            }
            Tab::Tile { tileset, .. } => {
                game.is_some_and(|g| g.resman.contains(&ResKey::new(*tileset, ResType::SET)))
            }
            Tab::Blueprints(keys) => keys.iter().all(|k| module.contains(k)),
            Tab::AreasProperties(areas) => areas.iter().all(area),
            // (A plugin's panel, while its plugin runs.)
            Tab::PluginPanel { plugin, panel } => self.enabled_plugins().iter().any(|p| {
                p.manifest.id == *plugin && p.manifest.panels.iter().any(|d| d.id == *panel)
            }),
            // (Found again when the record is read: `read_open_tabs`.)
            Tab::Instance { area: a, .. } | Tab::InstanceModel { area: a, .. } => area(a),
            Tab::Instances { objects } => objects.iter().all(|(a, _)| area(a)),
            tab => tab.renamable().is_none_or(|k| module.contains(&k)),
        }
    }

    /// A placed object's tab with its objects found again in their areas
    /// (`None`: one of them is not there now), and what each was, by its
    /// path now, into `found`.
    fn instance_again(
        &self,
        tab: Tab,
        objects: &BTreeMap<String, Placed>,
        found: &mut Vec<((ResRef, GffPath), Placed)>,
    ) -> Option<Tab> {
        let places: Vec<(ResRef, GffPath)> = match &tab {
            Tab::Instance { area, path } | Tab::InstanceModel { area, path } => {
                vec![(*area, path.clone())]
            }
            Tab::Instances { objects } => objects.to_vec(),
            _ => return Some(tab),
        };
        let ws = self.ws.as_ref()?;
        let mut gits = BTreeMap::new();
        let mut again: Vec<(ResRef, GffPath)> = Vec::new();
        let mut known = Vec::new();
        for (area, p) in &places {
            if !gits.contains_key(area) {
                gits.insert(*area, ws.module.gff(&ResKey::new(*area, ResType::GIT))?.ok()?);
            }
            let kept = objects.get(&object_text(*area, p));
            let now = found_again(&gits[area], p, kept)?;
            if let Some(o) = kept {
                known.push(((*area, now.clone()), o.clone()));
            }
            again.push((*area, now));
        }
        // (Only once every one of them is found again.)
        found.extend(known);
        let (area, first) = again[0].clone();
        Some(match tab {
            Tab::Instance { .. } => Tab::Instance { area, path: first },
            Tab::InstanceModel { .. } => Tab::InstanceModel { area, path: first },
            _ => Tab::Instances { objects: again.into() },
        })
    }

    /// Reads the tabs kept for the module just opened, to open at its
    /// first frame: whether there are any (the area opened last is not
    /// opened then).
    pub(crate) fn read_open_tabs(&mut self) -> bool {
        self.open_tabs = OpenTabs::default();
        let Some(path) = self.tabs_file() else { return false };
        let Ok(text) = std::fs::read_to_string(&path) else { return false };
        let record: Record = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(e) => {
                trace::note(format!("open tabs: {} not read: {e}", path.display()));
                return false;
            }
        };
        let mut views = record.views;
        // (The objects' keys in the one form, as the tabs' are named now.)
        let kept: BTreeMap<String, Placed> =
            record.objects.into_iter().map(|(k, p)| (object_key(&k), p)).collect();
        let mut objects = Vec::new();
        // (The resource browser, a tab before it was the Left Dock's
        // Resources panel: the panel shown, the tab never kept again.)
        let mut resources = false;
        // (And the palettes', the Palette panel.)
        let mut palette = false;
        let mut found = |text: &str| -> Option<(String, Want)> {
            if text == RESOURCES_TAB {
                resources = true;
                return None;
            }
            if text == PALETTE_TAB {
                palette = true;
                return None;
            }
            let passed = |why: &str| trace::note(format!("open tabs: {text} passed over: {why}"));
            let Some(want) = parse_tab(text) else {
                passed("not understood");
                return None;
            };
            if !self.tab_there(&want) {
                passed("not in the module now, or its file is gone");
                return None;
            }
            let Want::Tab(tab) = want else { return Some((text.to_string(), want)) };
            let Some(again) = self.instance_again(tab, &kept, &mut objects) else {
                passed("its object is not in its area now");
                return None;
            };
            // (Found elsewhere in its area's list: named anew, its view
            // with it.)
            let now = plain_text(&again).unwrap_or_else(|| text.to_string());
            if let Some(view) = views.remove(text) {
                views.insert(now.clone(), view);
            }
            Some((now, Want::Tab(again)))
        };
        // (Each kept text, as it is named now.)
        let mut renamed: HashMap<String, String> = HashMap::new();
        // (Every pane's tabs found first: the middle's pane's tree of the
        // editor groups names the other groups' tabs.)
        let mut found_tabs = Vec::new();
        for pane in &record.panes {
            let mut tabs = Vec::new();
            for text in &pane.tabs {
                if let Some((now, want)) = found(text) {
                    renamed.insert(text.clone(), now.clone());
                    tabs.push((now, want));
                }
            }
            if !tabs.is_empty() {
                found_tabs.push((pane, tabs));
            }
        }
        let mut panes = Vec::new();
        for (pane, tabs) in found_tabs {
            let rename = |t: &Option<String>| t.as_ref().and_then(|t| renamed.get(t)).cloned();
            let mut split = pane.split.clone();
            fn rename_split(s: &mut Split, renamed: &HashMap<String, String>) {
                s.tabs = s.tabs.iter().filter_map(|t| renamed.get(t).cloned()).collect();
                s.active = s.active.as_ref().and_then(|t| renamed.get(t)).cloned();
                s.parts.iter_mut().for_each(|p| rename_split(p, renamed));
            }
            if let Some(s) = &mut split {
                rename_split(s, &renamed);
            }
            // (A record from before the theme: the window as much taller as
            // its first tab's kind's are now, once, as the remembered sizes
            // are: `Settings::scale_window_sizes`.)
            let taller = match (record.rects_scaled, tabs.first()) {
                (false, Some((_, Want::Tab(tab)))) => tab.taller(),
                _ => 1.0,
            };
            let rect = pane.rect.map(|[x, y, w, h]| {
                egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, (h * taller).round()))
            });
            let active = rename(&pane.active);
            panes.push(Kept { place: Place::parse(&pane.place), tabs, active, rect, split });
        }
        // (The record says which panel each dock had in front, never how
        // the builder left the dock: its fold, `hide_tree` and
        // `hide_palettes`, stands, as do a narrow window's yields. The
        // Right Dock on the Palette is not asked for by name: it is drawn
        // as the active editor has it, beside an area.)
        if resources {
            self.left_panel = crate::Panel::Resources;
        }
        if palette {
            self.right_panel = Some(crate::Panel::Palette);
        }
        if panes.is_empty() {
            return false;
        }
        let front = (record.front.as_ref()).and_then(|t| renamed.get(t)).cloned();
        trace::note(format!("open tabs: {} panes from {}", panes.len(), path.display()));
        self.open_tabs.objects = objects.into_iter().collect();
        self.open_tabs.pending = Some(Pending { panes, front, views });
        self.open_tabs.written = Some(text);
        true
    }

    /// What a tab's view shows now, as it is kept: what was kept of it if
    /// it has not been drawn since it was opened again.
    fn view_of(&self, tab: &Tab) -> Option<View> {
        // (A conversation's editor made again is its own once drawn.)
        let drawn = |k: &ResKey| self.dialog_views.get(k).is_some_and(|v| v.content().is_some());
        if let Some(view) = self.open_tabs.views.get(tab)
            && !matches!(tab, Tab::Dialog(k) if drawn(k))
        {
            return Some(view.clone());
        }
        let page = |key: ResKey, path: GffPath| {
            let page = self.blueprint_pages.get(&(key, path))?;
            Some(View { page: Some(page.to_string()), ..Default::default() })
        };
        let git = |a: &ResRef| ResKey::new(*a, ResType::GIT);
        let are = |a: &ResRef| ResKey::new(*a, ResType::ARE);
        match tab {
            // (A NUI designer's source group: its switch, M4's U19.)
            Tab::NuiSource(k) => {
                let page = crate::nui_view::source::switch_name(self.nui_views.get(k)?);
                Some(View { page: Some(page.to_string()), ..Default::default() })
            }
            Tab::Area(r) => {
                let view = self.area_views.get(r)?;
                let o = view.orbit?;
                let [x, y, z] = o.target.to_array();
                let camera = Some([x, y, z, o.yaw, o.pitch, o.distance]);
                Some(View { camera, night: Some(view.night), ..Default::default() })
            }
            Tab::Script(k) => {
                let buf = self.scripts.get(k)?;
                let (scroll, cursor) = buf.seen?;
                let (scroll, cursor) = (Some([scroll.x, scroll.y]), cursor.map(|(a, b)| [a, b]));
                let lines = buf.bookmarks.iter().copied().collect();
                let numbered = (buf.numbered.iter().enumerate())
                    .filter_map(|(n, l)| Some([n, (*l)?]))
                    .collect();
                Some(View { scroll, cursor, lines, numbered, ..Default::default() })
            }
            Tab::Dialog(k) => {
                let view = self.dialog_views.get(k)?;
                let mut folded: Vec<(Kind, u32)> = view.collapsed.iter().copied().collect();
                folded.sort_by_key(|(k, i)| (*k == Kind::Reply, *i));
                Some(View {
                    content: Some(view.content()?),
                    selected: view.selected.map(row_text),
                    bookmarks: view.bookmarks.iter().copied().map(line_text).collect(),
                    folded: folded.into_iter().map(line_text).collect(),
                    ..Default::default()
                })
            }
            Tab::Blueprint(k) => page(*k, GffPath::root()),
            Tab::Blueprints(keys) => page(*keys.first()?, GffPath::root()),
            Tab::Instance { area, path } => page(git(area), path.clone()),
            Tab::Instances { objects } => {
                let (area, first) = objects.first()?;
                page(git(area), first.clone())
            }
            Tab::AreaProperties(r) => page(are(r), GffPath::root()),
            Tab::AreasProperties(rs) => page(are(rs.first()?), crate::area_props::several_page(rs)),
            Tab::TalkTable => {
                Some(View { line: Some(self.talk_view.selected?), ..Default::default() })
            }
            Tab::ModuleProperties => {
                let page = crate::module_props::PAGES.iter().find(|p| p.0 == self.module_page);
                Some(View { page: Some(page?.1.to_string()), ..Default::default() })
            }
            Tab::Options => {
                let now = self.options.as_ref()?.page;
                let page = crate::options::PAGES.iter().find(|p| p.0 == now);
                Some(View { page: Some(page?.1.to_string()), ..Default::default() })
            }
            Tab::Model(_) | Tab::InstanceModel { .. } | Tab::ModelCopy { .. } => {
                let o = self.model_views.get(&crate::model_view::Source::of_tab(tab)?)?.orbit;
                let [x, y, z] = o.target.to_array();
                Some(View {
                    camera: Some([x, y, z, o.yaw, o.pitch, o.distance]),
                    ..Default::default()
                })
            }
            Tab::AdvancedSearch => {
                Some(View { search: Some(self.advanced_search.kept()?), ..Default::default() })
            }
            Tab::Hak(id) => {
                let d = self.haks.iter().find(|d| d.id == *id)?;
                let chosen: Vec<String> = d.selected.iter().map(key_text).collect();
                (!chosen.is_empty()).then(|| View { chosen, ..Default::default() })
            }
            Tab::Table(id) => {
                let (row, column) = self.tables.iter().find(|d| d.id == *id)?.selected?;
                Some(View { cell: Some([row, column]), ..Default::default() })
            }
            Tab::Tileset(id) => {
                let d = self.tilesets.iter().find(|d| d.id == *id)?;
                Some(View {
                    page: Some(crate::tileset_view::page_name(d.page).to_string()),
                    tile: d.tile,
                    group: d.group,
                    ..Default::default()
                })
            }
            _ => None,
        }
    }

    /// The placed objects a tab shows, as they are found again (by
    /// `area:path`).
    fn objects_of(&self, tab: &Tab, into: &mut BTreeMap<String, Placed>) {
        let places: Vec<(ResRef, GffPath)> = match tab {
            Tab::Instance { area, path } | Tab::InstanceModel { area, path } => {
                vec![(*area, path.clone())]
            }
            Tab::Instances { objects } => objects.to_vec(),
            _ => return,
        };
        for (area, path) in &places {
            let area = *area;
            // (The area as it is edited, if it was read; else as it was
            // kept.)
            let git = (self.ws.as_ref()).and_then(|ws| ws.loaded(&ResKey::new(area, ResType::GIT)));
            let now = match (git, path.0.as_slice()) {
                (Some(git), [Step::Item(list, index)]) => git
                    .root
                    .list(list)
                    .and_then(|l| l.get(*index))
                    .and_then(|s| placed(list, *index, s)),
                (Some(_), _) => None,
                (None, _) => self.open_tabs.objects.get(&(area, path.clone())).cloned(),
            };
            if let Some(p) = now {
                into.insert(object_text(area, path), p);
            }
        }
    }

    /// The open module's tabs, as they are kept.
    fn tabs_record(&self) -> Record {
        let mut panes: Vec<Pane> = Vec::new();
        // (The windows' panes, by the window: one pane each, the way they
        // are split with it.)
        let mut windows: Vec<(egui_dock::SurfaceIndex, usize)> = Vec::new();
        let mut kept: Vec<&Tab> = Vec::new();
        let focused = self.dock.focused_leaf();
        for (path, leaf) in self.dock.iter_leaves() {
            let tabs: Vec<String> = leaf.tabs.iter().filter_map(|t| self.tab_text(t)).collect();
            if tabs.is_empty() {
                continue;
            }
            kept.extend(leaf.tabs.iter());
            let active = leaf.tabs.get(leaf.active.0).and_then(|t| self.tab_text(t));
            if !path.surface.is_main() {
                if let Some(&(_, i)) = windows.iter().find(|(s, _)| *s == path.surface) {
                    // (A split window's tabs are all there, in the
                    // split's order.)
                    if panes[i].split.is_none() {
                        panes[i].tabs.extend(tabs);
                    }
                    if focused == Some(path) {
                        panes[i].active = active;
                    }
                    continue;
                }
                // (As it was before it was maximized.)
                let rect = leaf.tabs.iter().find_map(|t| {
                    (self.maximized.get(t).map(|m| m.0)).or(self.windows.get(t).map(|w| w.now))
                });
                let rect = rect.map(|r| [r.left(), r.top(), r.width(), r.height()]);
                let tree = self.dock.get_surface(path.surface).and_then(|s| s.node_tree());
                let split = tree
                    .and_then(|t| split_of(self, t, NodeIndex::root()))
                    .filter(|s| !s.parts.is_empty());
                // (Split, its tabs in the split's order: past two panes,
                // not the order egui_dock's tree has its leaves in.)
                let tabs = split.as_ref().map_or(tabs, Split::flat);
                windows.push((path.surface, panes.len()));
                panes.push(Pane { place: Place::Window.text(), tabs, active, rect, split });
                continue;
            }
            let place = self.split_of(path.node).unwrap_or(Place::Middle);
            panes.push(Pane { place: place.text(), tabs, active, ..Default::default() });
        }
        // The editor groups as they lie, on the middle's pane, where there
        // are two or more: each group has its place too (`split right
        // 500`), which an older Moonglow reads, and which is the whole of it
        // for two groups; the tree keeps three or more in their order.
        if self.editor_groups().len() > 1
            && let Some(middle) = panes.iter_mut().find(|p| p.place == Place::Middle.text())
        {
            middle.split = split_of(self, self.dock.main_surface(), NodeIndex::root())
                .filter(|s| !s.parts.is_empty());
        }
        // An older record's Tile Properties still waiting for its area's
        // view (`tiles_again`): kept as it was read, in a pane of its own,
        // until the area is drawn; not once the area has gone from the
        // module (deleted since: written back for ever, else).
        let area_kept = |a: ResRef| {
            self.ws.as_ref().is_some_and(|ws| ws.module.contains(&ResKey::new(a, ResType::ARE)))
        };
        if let Some(w) = self.open_tabs.tiles.as_ref().filter(|w| area_kept(w.area)) {
            let text = format!("tile-properties:{}:{}", w.area, list(&w.tiles, |t| t.to_string()));
            panes.push(Pane {
                place: Place::Window.text(),
                tabs: vec![text],
                ..Default::default()
            });
        }
        let pane = focused.or_else(|| self.tab_pane());
        let leaf = pane.and_then(|p| self.dock.leaf(p).ok());
        let front = leaf.and_then(|l| l.tabs.get(l.active.0)).and_then(|t| self.tab_text(t));
        let mut views = BTreeMap::new();
        let mut objects = BTreeMap::new();
        for tab in kept {
            if let (Some(text), Some(view)) = (self.tab_text(tab), self.view_of(tab)) {
                views.insert(text, view);
            }
            self.objects_of(tab, &mut objects);
        }
        Record { panes, front, views, objects, rects_scaled: true }
    }

    /// Writes the open module's tabs, if they are kept and changed since
    /// they were last written.
    pub(crate) fn keep_open_tabs(&mut self) {
        if self.open_tabs.pending.is_some() || self.ws.is_none() {
            return;
        }
        let Some(path) = self.tabs_file() else { return };
        let Ok(text) = serde_json::to_string_pretty(&self.tabs_record()) else { return };
        if self.open_tabs.written.as_deref() == Some(text.as_str()) {
            return;
        }
        // (Whole or not at all: written beside it, then put in its place.)
        let written = mg_module::write_replacing_one_of_many(&path, text.as_bytes());
        match written {
            Ok(()) => {
                trace::note(format!("open tabs written: {}", path.display()));
                self.open_tabs.told = false;
                self.open_tabs.written = Some(text);
            }
            Err(e) if !std::mem::replace(&mut self.open_tabs.told, true) => self.log.warn(format!(
                "The open tabs could not be kept for the next time: {}: {e}",
                path.display()
            )),
            Err(_) => {}
        }
    }

    /// At the end of each frame: the kept tabs opened, at the module's
    /// first; then the tabs written a second after they last changed or
    /// the windows were last moved or resized (at once the first time,
    /// and on Close, Open Module and Exit, which write what is pending).
    pub(crate) fn open_tabs_frame(&mut self, ctx: &egui::Context) {
        if let Some(Pending { panes, front, views }) = self.open_tabs.pending.take() {
            if self.ws.is_some() {
                self.restore_tabs(panes, front, views);
                ctx.request_repaint();
            }
            return;
        }
        if self.open_tabs.tiles.is_some() {
            self.tiles_again();
        }
        if self.ws.is_none() || self.open_tabs_dir.is_none() || self.settings.no_reopen_tabs {
            return;
        }
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for (path, leaf) in self.dock.iter_leaves() {
            (path.surface.0, path.node.0, leaf.active.0, &leaf.tabs).hash(&mut h);
        }
        self.dock.focused_leaf().map(|p| (p.surface.0, p.node.0)).hash(&mut h);
        let shape = h.finish();
        let places = if self.windows.is_empty() {
            0
        } else {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            let mut places: Vec<[i32; 4]> = (self.windows.iter())
                .map(|(t, w)| self.maximized.get(t).map_or(w.now, |m| m.0))
                .map(|r| [r.left(), r.top(), r.width(), r.height()].map(|v| v.round() as i32))
                .collect();
            places.sort_unstable();
            places.hash(&mut h);
            h.finish()
        };
        let now = ctx.input(|i| i.time);
        if std::mem::replace(&mut self.open_tabs.shape, shape) != shape {
            // Establish initial snapshot immediately when none exists yet; once
            // written, debounce interactive tab switches on the same 1.0s timer
            // as window moves (flushed synchronously on Close/OpenModule/Exit).
            if self.open_tabs.written.is_none() {
                self.keep_open_tabs();
            } else {
                self.open_tabs.moved = Some(now);
                ctx.request_repaint_after_secs(1.0);
            }
        } else if std::mem::replace(&mut self.open_tabs.places, places) != places {
            self.open_tabs.moved = Some(now);
            ctx.request_repaint_after_secs(1.0);
        } else if let Some(moved) = self.open_tabs.moved {
            // (Asked for again until it is due: a frame may come a little
            // early.)
            match 1.0 - (now - moved) {
                left if left > 0.0 => crate::jobs::repaint_in(ctx, left),
                _ => {
                    self.open_tabs.moved = None;
                    self.keep_open_tabs();
                }
            }
        }
    }

    /// A kept tab opened: a side editor's file opened in it, the Options'
    /// draft made, the search References showed made again (`None`: it
    /// could not be, or it waits: Tile Properties, for its area's view).
    fn open_kept(&mut self, want: Want) -> Option<Tab> {
        // (Opened where kept, not where their kind opens: the opening's
        // own tab is not asked for.)
        let asked = self.actions.len();
        let tab = match want {
            Want::Tab(Tab::Options) => {
                if self.options.is_none() {
                    self.run_now(Action::OptionsDialog);
                }
                Some(Tab::Options)
            }
            Want::Tab(tab) => Some(tab),
            Want::Hak(path) => crate::hak_view::open_at(self, &path).map(Tab::Hak),
            Want::Tileset(path) => {
                crate::tileset_view::open_path(self, path.clone());
                self.tilesets.iter().find(|d| d.path == path).map(|d| Tab::Tileset(d.id))
            }
            Want::Table(TableAt::Module(key)) => {
                crate::table_view::open_in_module(self, key).map(Tab::Table)
            }
            Want::Table(TableAt::File(path)) => {
                crate::table_view::open_path(self, &path).map(Tab::Table)
            }
            Want::Table(TableAt::Hak(path, key)) => {
                let hak = crate::hak_view::open_at(self, &path)?;
                crate::table_view::open_in_hak(self, hak, key).map(Tab::Table)
            }
            Want::References(query) => {
                if let Some(q) = query {
                    self.find_references(q);
                }
                Some(Tab::References)
            }
            Want::TileProperties(..) => None,
            Want::GameTalk => {
                // (Without the game's data, the module's table alone.)
                let _ = crate::talk_view::game_table(self);
                Some(Tab::TalkTable)
            }
        };
        self.actions.truncate(asked);
        tab
    }

    /// Puts tabs in a pane at `place` (a window: at `rect`, its panes laid
    /// out as `split`), the tab `active` in front.
    fn place_pane(
        &mut self,
        place: Option<Place>,
        rect: Option<egui::Rect>,
        tabs: Vec<Tab>,
        active: Option<Tab>,
        split: Option<Laid>,
    ) {
        let middle = |app: &Moonglow| app.middle_pane();
        let pane_of = |app: &Moonglow, tab: &Tab| {
            app.dock.find_tab(tab).map(|p| NodePath { surface: p.surface, node: p.node })
        };
        let tabs: Vec<Tab> = tabs.into_iter().filter(|t| self.dock.find_tab(t).is_none()).collect();
        let Some(first) = tabs.first().cloned() else { return };
        let at = match place {
            Some(Place::Middle) => (self.dock.find_tab(&Tab::NoArea))
                .map(|p| NodePath { surface: p.surface, node: p.node })
                .or_else(|| middle(self)),
            Some(Place::Split(side, share)) => self.split_middle(side, share, &first),
            Some(Place::Window) => {
                // (Its panes as they were, if all their tabs are to come.)
                let split = split.filter(|s| {
                    let laid = s.tabs();
                    laid.len() == tabs.len() && laid.iter().all(|t| tabs.contains(t))
                });
                let mut first = split.as_ref().map_or(first.clone(), |s| s.first().clone());
                self.open_window(first.clone());
                if let (Some(split), Some(path)) = (&split, self.dock.find_tab(&first))
                    && let Some(tree) =
                        self.dock.get_surface_mut(path.surface).and_then(|s| s.node_tree_mut())
                {
                    split.build(tree, path.node);
                    // (A window is followed by the first tab of the pane
                    // egui_dock lists first, which is another past two.)
                    let lead = (self.dock.iter_leaves())
                        .find(|(p, l)| p.surface == path.surface && !l.tabs.is_empty())
                        .map(|(_, l)| l.tabs[0].clone());
                    if let Some(lead) = lead.filter(|l| *l != first) {
                        if let Some(track) = self.windows.remove(&first) {
                            self.windows.insert(lead.clone(), track);
                        }
                        first = lead;
                    }
                }
                if let Some(rect) = rect {
                    let screen = self.screen.unwrap_or(rect);
                    let size = rect.size().min(screen.size());
                    let at = rect.min.clamp(screen.min, (screen.max - size).max(screen.min));
                    self.put_window(&first, egui::Rect::from_min_size(at, size));
                }
                pane_of(self, &first)
            }
            None => None,
        };
        for tab in tabs {
            match at {
                _ if self.dock.find_tab(&tab).is_some() => {}
                Some(at) => {
                    self.dock.set_focused_node_and_surface(at);
                    self.dock.push_to_focused_leaf(tab);
                }
                // (Where its kind opens.)
                None => self.run_now(Action::OpenTab(tab)),
            }
        }
        if let Some(path) = active.and_then(|t| self.dock.find_tab(&t)) {
            let _ = self.dock.set_active_tab(path);
        }
    }

    /// Opens the kept tabs in place of the module's first (Module
    /// Properties, docked in the middle), pane by pane: each where it
    /// was, or where its kind opens if that place is not there now; and
    /// gives their views what they showed.
    fn restore_tabs(
        &mut self,
        panes: Vec<Kept>,
        front: Option<String>,
        mut views: BTreeMap<String, View>,
    ) {
        // (The pane that says no area is open keeps the middle's place
        // while they come, and goes once they have: `keep_middle`.)
        let middle = self.middle_pane();
        if self.dock.find_tab(&Tab::NoArea).is_none()
            && let Some(at) = middle
        {
            self.dock.set_focused_node_and_surface(at);
            self.dock.push_to_focused_leaf(Tab::NoArea);
        }
        // (A side editor's tab open already, a hak opened with the module
        // say, is not the module's to close: back below, if it is not
        // among those kept.)
        let side: Vec<Tab> = (self.dock.iter_all_tabs())
            .map(|(_, t)| t.clone())
            .filter(|t| crate::side_doc::is_side(t) && self.side_doc(t).is_some())
            .collect();
        tabs::close_tabs(&mut self.dock, |t| *t != Tab::ModuleProperties);
        // The side editors' files opened first (a 2DA of a hak's needs the
        // hak open), each tab by its text.
        let mut opened: HashMap<String, Tab> = HashMap::new();
        for pane in &panes {
            for (text, want) in &pane.tabs {
                if let Want::TileProperties(area, tiles) = want {
                    let tiles = tiles.clone();
                    self.open_tabs.tiles = Some(TilesWaiting { area: *area, tiles });
                    continue;
                }
                match self.open_kept(want.clone()) {
                    Some(tab) => _ = opened.insert(text.clone(), tab),
                    None => trace::note(format!("open tabs: {text} passed over: not opened")),
                }
            }
        }
        // The editor groups laid out first, as they lay, from the middle's
        // tree where the record has one (where the pane that says no area
        // is open holds the middle's place): the groups' own places then
        // find their tabs placed. (Module Properties, docked as the module
        // opened, goes where the tree has it.)
        let groups = (panes.iter())
            .find(|p| p.place == Some(Place::Middle))
            .and_then(|p| p.split.as_ref())
            .and_then(|s| Laid::of(s, &opened));
        if let Some(groups) = groups {
            for tab in groups.tabs() {
                if let Some(path) = self.dock.find_tab(&tab) {
                    self.dock.remove_tab(path);
                }
            }
            let at = (self.dock.find_tab(&Tab::NoArea))
                .map(|p| NodePath { surface: p.surface, node: p.node })
                .or_else(|| self.middle_pane())
                .filter(|p| p.surface.is_main());
            if let Some(at) = at {
                self.dock.set_focused_node_and_surface(at);
                self.dock.push_to_focused_leaf(groups.first().clone());
                groups.build(self.dock.main_surface_mut(), at.node);
            }
        }
        for pane in panes {
            let tabs: Vec<Tab> =
                pane.tabs.iter().filter_map(|(t, _)| opened.get(t).cloned()).collect();
            let active = pane.active.as_ref().and_then(|t| opened.get(t).cloned());
            let split = pane.split.as_ref().and_then(|s| Laid::of(s, &opened));
            self.place_pane(pane.place, pane.rect, tabs, active, split);
        }
        for tab in side {
            if self.dock.find_tab(&tab).is_none() {
                self.dock.main_surface_mut().push_to_first_leaf(tab);
            }
        }
        // (A NUI designer's source kept without its design: opened as Split
        // source opens it, the design first. M4's U19 review.)
        let alone: Vec<ResKey> = (self.dock.iter_all_tabs())
            .filter_map(|(_, t)| match t {
                Tab::NuiSource(k) => Some(*k),
                _ => None,
            })
            .filter(|k| self.dock.find_tab(&Tab::Nui(*k)).is_none())
            .collect();
        for key in alone {
            if let Some(path) = self.dock.find_tab(&Tab::NuiSource(key)) {
                self.dock.remove_tab(path);
            }
            self.actions.push(crate::Action::NuiSource(key, true));
        }
        // (A side editor's changed document whose tab was closed above, not
        // among those kept: its tab, rather than none to save it from.)
        self.side_docs_follow_tabs();
        // What the views showed: the conversations' and the pages of
        // Module Properties and the Options now, the rest as each is
        // first drawn.
        for (text, tab) in &opened {
            let Some(view) = views.remove(text) else { continue };
            match tab {
                Tab::Dialog(key) => self.dialog_again(*key, &view),
                Tab::ModuleProperties => {
                    let page =
                        crate::module_props::PAGES.iter().find(|p| view.page == Some(p.1.into()));
                    if let Some(page) = page {
                        self.module_page = page.0;
                    }
                }
                Tab::Options => {
                    let page = crate::options::PAGES.iter().find(|p| view.page == Some(p.1.into()));
                    if let (Some(page), Some(draft)) = (page, &mut self.options) {
                        draft.page = page.0;
                    }
                }
                Tab::AdvancedSearch => {
                    if let Some(k) = &view.search {
                        self.advanced_search.ask_again(k);
                    }
                }
                // The side editors are open now: what they showed is set.
                Tab::Hak(id) => {
                    if let Some(d) = self.haks.iter_mut().find(|d| d.id == *id) {
                        let have = |k: &ResKey| d.hak.items().iter().any(|e| e.key == *k);
                        let keys = view.chosen.iter().filter_map(|t| parse_key(t));
                        d.selected = keys.filter(|k| have(k)).collect();
                    }
                }
                Tab::Table(id) => {
                    if let (Some(d), Some([row, column])) =
                        (self.tables.iter_mut().find(|d| d.id == *id), view.cell)
                    {
                        let t = d.edit.table();
                        if row < t.len() && column < t.columns().len() {
                            d.go_to(row, column);
                        }
                    }
                }
                Tab::Tileset(id) => {
                    if let Some(d) = self.tilesets.iter_mut().find(|d| d.id == *id) {
                        d.show(view.page.as_deref(), view.tile, view.group);
                    }
                }
                tab => _ = self.open_tabs.views.insert(tab.clone(), view),
            }
        }
        // (The main surface's pane with the focus, whose tab in front is the
        // active editor that the docks follow: the middle's, not the pane
        // made last; the tab in front, where it is in the main surface,
        // takes it to its own.)
        if let Some(middle) = self.middle_pane() {
            self.dock.main_surface_mut().set_focused_node(middle.node);
        }
        if let Some(front) = front.and_then(|t| opened.get(&t).cloned()) {
            self.show_tab(front);
        }
        trace::note(format!("open tabs restored: {}", self.panes()));
    }

    /// A conversation's editor as it was left: its selected line,
    /// bookmarks and folded branches, if the conversation is as it was
    /// then (they are by line number: another's lines are other lines).
    fn dialog_again(&mut self, key: ResKey, view: &View) {
        let Some(ws) = &mut self.ws else { return };
        let Ok(g) = ws.doc(&key) else { return };
        if view.content != Some(crate::dialog_view::fingerprint(g)) {
            trace::note(format!("open tabs: {key} changed since: its lines' marks let go"));
            return;
        }
        let line = |t: &String| parse_line(t);
        let mut again = DialogView::default();
        again.selected = view.selected.as_deref().and_then(parse_row);
        again.reveal = again.selected.is_some();
        again.bookmarks = view.bookmarks.iter().filter_map(line).collect();
        again.collapsed = view.folded.iter().filter_map(line).collect();
        self.dialog_views.insert(key, again);
        // (Kept as it was until the editor is drawn.)
        self.open_tabs.views.insert(Tab::Dialog(key), view.clone());
    }

    /// A record from before Tile Properties was a panel (M4's U5) that
    /// kept its tab, once its area's view has its tiles: Select Tiles on
    /// in the area, those of the tiles it was of that the area still has
    /// selected, and the Right Dock's Tile Properties panel shown (its
    /// header toggle turned on), as the tile menu's Tile Properties
    /// does. Nothing else opens. (Records written since keep no such tab.)
    fn tiles_again(&mut self) {
        let Some(w) = &self.open_tabs.tiles else { return };
        let area = w.area;
        let ready = (self.area_views.get(&area))
            .filter(|v| v.terrain.is_some())
            .and_then(|v| v.model.as_ref().map(|m| (m.width, m.height)));
        let Some((width, height)) = ready else { return };
        let Some(w) = self.open_tabs.tiles.take() else { return };
        let width = width.max(1);
        let cells: Vec<(u32, u32)> = (w.tiles.iter())
            .filter_map(|&i| u32::try_from(i).ok())
            .map(|i| (i % width, i / width))
            .filter(|&(_, y)| y < height)
            .collect();
        if cells.is_empty() {
            return;
        }
        // (As the tile menu's Tile Properties shows them: the panel shown
        // whatever its header toggle said.)
        if let Some(mut view) = self.area_views.remove(&area) {
            crate::tile_select::properties_of(self, &mut view, &cells);
            self.area_views.insert(area, view);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Side;

    #[test]
    fn tabs_are_named_and_read_back() {
        let key = |s: &str| parse_key(s).unwrap();
        let area = ResRef::from_str("field").unwrap();
        let path = GffPath::root().item("Creature List", 3);
        let tabs = [
            Tab::ModuleProperties,
            Tab::Options,
            Tab::PlaceableGallery,
            Tab::Plugins,
            Tab::Script(key("hello.nss")),
            Tab::Blueprint(key("nw_chest.utp")),
            Tab::Blueprints(vec![key("a.uti"), key("b.uti")]),
            Tab::Dialog(key("talk.dlg")),
            Tab::ResourceCopy { key: key("appearance.2da"), layer: "hak:my:hak".into() },
            Tab::Area(area),
            Tab::AreasProperties(vec![area, ResRef::from_str("cave").unwrap()]),
            Tab::Instance { area, path: path.clone() },
            Tab::InstanceModel { area, path: path.clone() },
            Tab::instances(area, vec![path.clone(), GffPath::root().item("Door List", 0)]),
            // (Of several areas, each object its area's: a held item too.)
            Tab::Instances {
                objects: std::sync::Arc::from([
                    (area, path.clone()),
                    (ResRef::from_str("cave").unwrap(), path.item("ItemList", 2)),
                ]),
            },
            Tab::Tile { tileset: ResRef::from_str("ttr01").unwrap(), tile: 12 },
        ];
        for tab in tabs {
            let text = plain_text(&tab).unwrap();
            assert_eq!(parse_tab(&text), Some(Want::Tab(tab)), "{text}");
        }
        assert_eq!(parse_tab("script:"), None);
        assert_eq!(parse_tab("something-new:x"), None);
        // (The resource browser's tab of before: not a tab now, the
        // Resources panel shown by `read_open_tabs`.)
        assert_eq!(parse_tab(RESOURCES_TAB), None);
        // (Nor the palettes': the Palette panel shown.)
        assert_eq!(parse_tab(PALETTE_TAB), None);
        // The side editors' files, and what the tools showed.
        let wants = [
            ("hak:/x/my.hak", Want::Hak("/x/my.hak".into())),
            ("tileset:C:\\sets\\zzz01.set", Want::Tileset("C:\\sets\\zzz01.set".into())),
            ("table:spells.2da", Want::Table(TableAt::Module(key("spells.2da")))),
            ("table-file:/x/a.2da", Want::Table(TableAt::File("/x/a.2da".into()))),
            (
                "table-hak:a.2da:/x/my.hak",
                Want::Table(TableAt::Hak("/x/my.hak".into(), key("a.2da"))),
            ),
            ("references", Want::References(None)),
            ("references:tag:A:B", Want::References(Some(Query::Tag("A:B".into())))),
            ("references:strref:7", Want::References(Some(Query::StrRef(7)))),
            ("tile-properties:field:3,4", Want::TileProperties(area, vec![3, 4])),
        ];
        for (text, want) in wants {
            assert_eq!(parse_tab(text), Some(want), "{text}");
        }
        assert_eq!(parse_tab("hak:"), None);
    }

    #[test]
    fn places_are_named_and_read_back() {
        for place in [Place::Window, Place::Middle, Place::Split(Side::Above, 300)] {
            assert_eq!(Place::parse(&place.text()), Some(place));
        }
        // (The palettes' pane, a place before the palettes were the Right
        // Dock's panel: no place now, "where its kind opens".)
        assert_eq!(Place::parse("palettes"), None);
        assert_eq!(Place::parse("split sideways 300"), None);
        assert_eq!(Place::parse("split left 2000"), Some(Place::Split(Side::Left, 950)));
    }

    #[test]
    fn lines_and_rows_are_named_and_read_back() {
        for row in [
            Row { parent: Parent::Root, pos: 2 },
            Row { parent: Parent::Node(Kind::Reply, 7), pos: 0 },
        ] {
            assert_eq!(parse_row(&row_text(row)), Some(row));
        }
        assert_eq!(parse_line(&line_text((Kind::Entry, 4))), Some((Kind::Entry, 4)));
        assert_eq!(parse_line("node 4"), None);
    }

    /// A record of an older version (no views, objects nor splits) reads.
    #[test]
    fn an_older_record_reads() {
        let old = r#"{"panes":[{"place":"window","tabs":["script:a.nss"],"rect":[1,2,3,4]}]}"#;
        let r: Record = serde_json::from_str(old).unwrap();
        assert_eq!(r.panes[0].tabs, ["script:a.nss"]);
        assert!(r.views.is_empty() && r.objects.is_empty() && r.panes[0].split.is_none());
    }

    #[test]
    fn placed_store_reads_resref_and_decodes_tag() {
        let mut s = mg_gff::Struct::new(0);
        s.set("ResRef", mg_gff::Value::ResRef(b"shop_01".to_vec()));
        s.set("Tag", mg_gff::Value::String(b"Caf\xe9".to_vec()));
        s.set("XPosition", mg_gff::Value::Float(1.0));
        s.set("YPosition", mg_gff::Value::Float(2.0));
        s.set("ZPosition", mg_gff::Value::Float(3.0));
        let p = placed("StoreList", 0, &s).expect("store placed");
        assert_eq!(p.resref, "shop_01");
        assert_eq!(p.tag, crate::text::decode(b"Caf\xe9"));
        assert!(!p.tag.contains('\u{fffd}'), "{:?}", p.tag);
    }

    #[test]
    fn keep_open_tabs_write_failure_does_not_cache_written() {
        let dir = mg_testkit::scratch_dir("ui-open-tabs-write-fail");
        let blocker = dir.join("not_a_dir");
        std::fs::write(&blocker, b"x").unwrap();
        let mut m = mg_module::Module::new();
        let m_path = dir.join("m.mod");
        m.save_as(&mg_module::ModuleLocation::Archive(m_path.clone())).unwrap();

        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.open_module(&m_path);
        app.open_tabs_dir = Some(blocker);
        app.open_tabs.written = None;
        app.open_tabs.told = false;
        app.keep_open_tabs();

        assert!(app.open_tabs.written.is_none());
        assert!(app.open_tabs.told);
    }

    /// A kept tab's object recorded before 5.2.0 (the tag read as UTF-8, a
    /// store's blueprint kept empty) is found again.
    #[test]
    fn an_object_kept_in_the_old_form_is_found_again() {
        let mut s = mg_gff::Struct::new(0);
        s.set("ResRef", mg_gff::Value::ResRef(b"shop_01".to_vec()));
        s.set("Tag", mg_gff::Value::String(b"Caf\xe9".to_vec()));
        s.set("XPosition", mg_gff::Value::Float(1.0));
        s.set("YPosition", mg_gff::Value::Float(2.0));
        let mut git = mg_gff::Gff::new(*b"GIT ");
        git.root.set("StoreList", mg_gff::Value::List(vec![s]));
        let path = GffPath::root().item("StoreList", 0);
        let at = placed("StoreList", 0, &git.root.list("StoreList").unwrap()[0]).unwrap().at;
        let kept = Placed {
            tag: String::from_utf8_lossy(b"Caf\xe9").into_owned(),
            resref: String::new(),
            at,
        };
        assert_eq!(found_again(&git, &path, Some(&kept)), Some(path.clone()));
        // And as 5.2.0 keeps it.
        let now = placed("StoreList", 0, &git.root.list("StoreList").unwrap()[0]).unwrap();
        assert_eq!(found_again(&git, &path, Some(&now)), Some(path.clone()));
        // Another object's record is not.
        let other = Placed { tag: "Other".into(), resref: String::new(), at };
        assert_eq!(found_again(&git, &path, Some(&other)), None);
    }
}
