//! The blueprint palettes, as Aurora's right-hand pane: a blueprint type,
//! Standard (the game's) or Custom (the module's), and the categories with
//! their blueprints. Edit opens a custom blueprint; Edit Copy copies any
//! blueprint into the module as a new custom one; Delete removes a custom
//! one; Preview shows it in the 3D viewer.

use mg_uikit::Controls as _;
use std::collections::HashMap;
use std::sync::Arc;

use mg_core::ResRef;
use mg_core::text::Needle;
use mg_edit::{Command, Edit};
use mg_gff::{Gff, Value};
use mg_module::palette::{
    BlueprintKind, Palette, PaletteBlueprint, PaletteNode, update_custom_palette,
};
use mg_module::palette_groups::{Grouped, Grouping};
use mg_resman::ResKey;

use crate::{Action, Moonglow, Tab};

/// A collapsing header's words (`text`), cut short with an ellipsis
/// where they are wider than the room beside its arrow in `ui` (egui lays
/// a header's words out on one line, as wide as they are). Its name for a
/// screen reader is the whole of them. `reserve`: the points left at the
/// row's end (a category's count, drawn there).
fn fitted_beside(ui: &egui::Ui, text: String, reserve: f32) -> egui::WidgetText {
    let spacing = ui.spacing();
    let room = ui.available_width()
        - reserve
        - spacing.icon_width
        - spacing.icon_spacing
        - 2.0 * spacing.button_padding.x;
    let font = egui::TextStyle::Button.resolve(ui.style());
    let whole = ui.fonts_mut(|f| {
        f.layout_no_wrap(text.clone(), font.clone(), egui::Color32::PLACEHOLDER).size().x
    });
    if whole <= room || room <= 0.0 {
        return text.into();
    }
    let mut job = egui::text::LayoutJob::simple_singleline(text, font, egui::Color32::PLACEHOLDER);
    job.wrap = egui::text::TextWrapping::truncate_at_width(room);
    ui.fonts_mut(|f| f.layout_job(job)).into()
}

/// Whether blueprints of a kind have pictures (a model to draw: their own,
/// or a waypoint's flag).
fn pictured(kind: BlueprintKind) -> bool {
    use BlueprintKind as K;
    matches!(kind, K::Creature | K::Door | K::Item | K::Placeable | K::Waypoint)
}

/// A blueprint dragged from the palette (dropped in an area view, it is
/// placed there).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Dragged(pub(crate) ResKey);

/// Blueprints' tags, by resref.
type Tags = Arc<HashMap<ResRef, String>>;

/// What a palette was grouped for: the palette itself (held, and told by
/// `Arc::ptr_eq`: kept by its address alone, a later palette made at the
/// address of one let go of was shown as the old one was grouped), the
/// groupings as read, the grouping and whether by category within.
type GroupedFor = (Arc<Palette>, u64, String, bool);

/// Cached folded search strings for a palette's blueprints and tags.
#[derive(Debug)]
struct FoldCache {
    palette: Arc<Palette>,
    tags: Arc<HashMap<ResRef, String>>,
    kind: BlueprintKind,
    custom: bool,
    revision: Option<u64>,
    names: (u64, u64),
    fields: Arc<Vec<(ResRef, [String; 3])>>,
}

/// Flattened pre-order metadata for a palette node slice so visibility and
/// subtree counts run in O(1) per node during drawing.
#[derive(Debug, Clone, Copy, Default)]
struct NodeMeta {
    total: u32,
    shown: u32,
    subtree_len: u32,
    /// Where the node's own blueprints a search finds begin in
    /// [`MetaCache::picks`] (their places in the node), and how many.
    picks_from: u32,
    picks: u32,
}

#[derive(Debug)]
struct MetaCache {
    palette: Arc<Palette>,
    grouped: Option<Arc<Grouped>>,
    found: Option<Arc<std::collections::HashSet<ResRef>>>,
    meta: Arc<[NodeMeta]>,
    /// The places of each node's blueprints the search finds, node after
    /// node ([`NodeMeta::picks_from`]): found once for the search, not
    /// looked up row by row each frame (8,000 found, 0.3 ms a frame).
    picks: Arc<[u32]>,
}

/// Zero-allocation view over either a raw blueprint slice or a filtered subset.
#[derive(Clone, Copy)]
enum NodeRows<'a> {
    Slice(&'a [PaletteBlueprint]),
    Refs(&'a [&'a PaletteBlueprint]),
}

impl<'a> NodeRows<'a> {
    fn len(self) -> usize {
        match self {
            NodeRows::Slice(s) => s.len(),
            NodeRows::Refs(r) => r.len(),
        }
    }

    fn get(self, i: usize) -> Option<&'a PaletteBlueprint> {
        match self {
            NodeRows::Slice(s) => s.get(i),
            NodeRows::Refs(r) => r.get(i).copied(),
        }
    }

    fn iter(self) -> NodeRowsIter<'a> {
        match self {
            NodeRows::Slice(s) => NodeRowsIter::Slice(s.iter()),
            NodeRows::Refs(r) => NodeRowsIter::Refs(r.iter()),
        }
    }
}

enum NodeRowsIter<'a> {
    Slice(std::slice::Iter<'a, PaletteBlueprint>),
    Refs(std::slice::Iter<'a, &'a PaletteBlueprint>),
}

impl<'a> Iterator for NodeRowsIter<'a> {
    type Item = &'a PaletteBlueprint;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            NodeRowsIter::Slice(it) => it.next(),
            NodeRowsIter::Refs(it) => it.next().copied(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            NodeRowsIter::Slice(it) => it.size_hint(),
            NodeRowsIter::Refs(it) => it.size_hint(),
        }
    }

    // (Skipped at once: the rows above those in sight, `Tree::rows`.)
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        match self {
            NodeRowsIter::Slice(it) => it.nth(n),
            NodeRowsIter::Refs(it) => it.nth(n).copied(),
        }
    }
}

/// The Palette panel's state (the Right Dock's).
#[derive(Debug)]
pub struct PaletteView {
    pub kind: BlueprintKind,
    /// Custom (module) blueprints rather than the game's.
    pub custom: bool,
    /// Shows only blueprints whose name or resref contains this.
    pub filter: String,
    /// Expand All or Collapse All was clicked this frame: every category
    /// opens (true) or closes.
    pub(crate) fold: Option<bool>,
    standard: HashMap<BlueprintKind, Arc<Palette>>,
    /// Custom palettes, each built for a workspace revision.
    custom_cache: HashMap<BlueprintKind, (u64, Arc<Palette>)>,
    /// For a module that has no custom palette of a type: the one made
    /// for it, and the module it was made from with it in
    /// ([`custom_palette_now`]).
    made: HashMap<BlueprintKind, mg_module::Module>,
    /// The module's own blueprints of a type (their ResRefs), and the
    /// workspace revision they were listed at: the custom palette asks
    /// which of its rows are the module's every frame.
    #[allow(clippy::type_complexity)]
    own: Option<((u64, BlueprintKind), Arc<std::collections::HashSet<ResRef>>)>,
    /// Each palette grouped as Group by asks (`grouped`), and what for.
    grouped: HashMap<(BlueprintKind, bool), (GroupedFor, Arc<Grouped>)>,
    pub selected: Option<ResKey>,
    /// The hovered blueprint's thumbnail, once rendered.
    thumb: Option<(ResKey, Option<egui::TextureId>)>,
    /// The blueprint under the pointer and since when (the window's
    /// clock): its picture and what there is to say of it are asked for
    /// only once the pointer has rested on its row for [`HOVER_REST`], not
    /// for each row the list scrolls under a resting pointer (each would
    /// make a picture: M3 T14's review).
    hover_since: Option<(ResKey, f64)>,
    /// What there is to say of the hovered blueprint, where it has no
    /// model to show (a sound, a trigger, an encounter, a store, a
    /// waypoint).
    about: Option<(ResKey, Vec<String>)>,
    /// The last search's finds: (palette, search, revision, the game's
    /// names), blueprints, and how found.
    #[allow(clippy::type_complexity)]
    found: Option<(
        (BlueprintKind, bool, String, Option<u64>, (u64, u64)),
        Arc<std::collections::HashSet<ResRef>>,
        Needle,
    )>,
    /// The blueprints the search found that it hadn't before the module
    /// changed (one added, here or outside, or named so it is found):
    /// their categories open, as typing the search opened the others.
    arrived: std::collections::HashSet<ResRef>,
    /// Blueprints' tags, for the search: by palette (and module revision).
    tags: HashMap<(BlueprintKind, bool), (Option<u64>, Tags)>,
    /// Each blueprint's tag as it was read, for [`tags_read`] and
    /// [`tag_of`].
    tags_read: HashMap<ResKey, TagRead>,
    /// The hovered blueprint's Tag (none: it has none), for its tooltip or
    /// the Gallery's enlargement ([`tag_of`]).
    hover_tag: Option<(ResKey, Option<String>)>,
    /// How many blueprints' Tags were read one by one for a hover
    /// ([`tag_of`]; counted for the tests).
    pub tags_read_one_by_one: usize,
    /// More blueprints chosen with Ctrl+click (with `selected`): custom
    /// ones for bulk edits, placeables to paint (`paint_mode`).
    pub chosen: Vec<ResKey>,
    /// The palette shown last was brought back from the settings (once,
    /// when the palettes are first drawn).
    restored: bool,
    /// The keys were given to the palette (Next Pane, F6): its cursor goes
    /// to its first row, where it is at none.
    pub(crate) to_first: bool,
    /// The tileset palette is shown, rather than blueprints.
    pub tiles: bool,
    /// The prefabs are shown (groups of placed objects saved to place
    /// again), rather than blueprints.
    pub prefabs: bool,
    /// The start location's page is shown (Aurora's Paint Start
    /// Location), rather than blueprints.
    pub start: bool,
    /// The module's start location is about to be placed: a click in an
    /// area's view puts it there.
    pub placing_start: bool,
    /// The lights' page is shown (the game's invisible light placeables,
    /// as the area's menu's Add Light Here has them), rather than
    /// blueprints.
    pub lights: bool,
    /// A light is about to be placed: its placeables.2da row. A click in
    /// an area's view puts one there.
    pub placing_light: Option<usize>,
    /// An appearance taken from the Appearance Gallery to place (a click
    /// on a placeable's picture there): its ghost follows the pointer, and
    /// a click in an area's view puts a placeable of it there.
    pub placing_appearance: Option<crate::appearance_gallery::DraggedAppearance>,
    /// The tileset brush chosen.
    pub tile_brush: Option<crate::terrain_mode::TileBrush>,
    /// The tile chosen last among a terrain's or crosser's pictures (the
    /// tiles palette's Gallery), of the tileset shown: Refine Tile, chosen
    /// after it, makes tiles that one.
    pub tile_choice: Option<u32>,
    /// The area shown last: its tileset's palette is the one shown.
    pub area: Option<ResRef>,
    pub(crate) tile_palettes: HashMap<ResRef, Arc<mg_area::terrain::TilesetPalette>>,
    /// Tilesets' names as Area Properties shows them, read once each (a
    /// big tileset takes milliseconds to read: too long for every frame).
    pub(crate) tileset_names: HashMap<ResRef, String>,
    /// Tilesets' grass (`[GRASS]`) as Area Properties › Grass shows it and
    /// tries it out, read once each (`area_grass`).
    pub(crate) tileset_grass:
        std::cell::RefCell<HashMap<ResRef, Option<mg_area::grass::GrassLook>>>,
    /// Each type's categories as a blueprint's Category lists them
    /// ([`categories`]), and what they were made for: the workspace's
    /// revision (none without a module) and whether by name.
    #[allow(clippy::type_complexity)]
    categories: HashMap<BlueprintKind, ((Option<u64>, bool), Arc<Vec<mg_rules::Choice>>)>,
    /// The row the arrow keys are at: the one clicked last, or come to
    /// with them.
    pub(crate) cursor: Option<At>,
    /// The tileset palette's rows, and the arrow keys' cursor in them.
    pub(crate) tile_list: FlatList<crate::terrain_mode::TileRow>,
    /// The Prefabs list's rows (by name), and the arrow keys' cursor.
    pub(crate) prefab_list: FlatList<String>,
    /// A category to open or close when the palette is drawn next (the
    /// Left and Right keys).
    toggle: Option<(Branch, bool)>,
    /// The cursor's row is to be brought into view: the keys moved it.
    reveal: bool,
    /// Blueprints and their names as the editors read them, kept.
    pub(crate) kept: crate::blueprint::picker::Kept,
    fold_fields: Option<FoldCache>,
    node_meta: Option<MetaCache>,
    palette_total: Option<(Arc<Palette>, usize)>,
    open_synced: Vec<String>,
    open_set: std::collections::HashSet<String>,
}

/// A branch of a palette's tree: a category (by its place among the
/// categories, down from the top) or the Favorites or Recent list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Branch {
    Category(Vec<usize>),
    Remembered(&'static str),
}

/// A row of a palette's tree: a branch's own, or a blueprint's under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct At {
    kind: BlueprintKind,
    custom: bool,
    /// The grouping the palette was shown by (its id, and whether by
    /// category within): none, by Category.
    grouping: Option<(String, bool)>,
    pub(crate) branch: Branch,
    pub(crate) blueprint: Option<ResKey>,
}

/// The palette's rows as they were drawn, top to bottom, for the arrow
/// keys to move through: listed only in a frame a key is pressed.
#[derive(Debug, Default)]
struct Listed {
    branches: Vec<Listing>,
    /// Each row's branch, and the blueprint if it is one's.
    rows: Vec<(usize, Option<ResKey>)>,
    /// In the Gallery, where each blueprint's picture is (by its row):
    /// Up and Down go a row of pictures at a time.
    places: HashMap<usize, egui::Rect>,
    /// Where each row's top is (by its row), and the list's height in
    /// sight: Page Up and Page Down go a list's height at a time.
    tops: HashMap<usize, f32>,
    page: f32,
}

/// A branch as listed: the branch it is in, its own row, and whether it
/// is open.
#[derive(Debug)]
struct Listing {
    branch: Branch,
    parent: Option<usize>,
    row: usize,
    open: bool,
}

/// What an arrow key does from a row.
#[derive(Debug, PartialEq)]
enum Step {
    /// The cursor goes to this row.
    To(usize),
    /// This branch opens or closes.
    Fold(usize, bool),
    Stay,
}

impl Listed {
    fn branch(&mut self, branch: Branch, parent: Option<usize>, top: f32) -> usize {
        self.branches.push(Listing { branch, parent, row: self.rows.len(), open: false });
        self.tops.insert(self.rows.len(), top);
        self.rows.push((self.branches.len() - 1, None));
        self.branches.len() - 1
    }

    /// The branch a row is in (a branch's own row: its parent).
    fn within(&self, row: usize) -> Option<usize> {
        let (branch, blueprint) = self.rows[row];
        if blueprint.is_some() { Some(branch) } else { self.branches[branch].parent }
    }

    /// Whether a row shows: every branch it is in is open. (A branch
    /// closing still draws its rows for a moment.)
    fn shows(&self, row: usize) -> bool {
        let mut at = self.within(row);
        while let Some(b) = at {
            if !self.branches[b].open {
                return false;
            }
            at = self.branches[b].parent;
        }
        true
    }

    /// The row of `at`, if it is listed.
    fn find(&self, at: &At) -> Option<usize> {
        let branch = self.branches.iter().position(|b| b.branch == at.branch)?;
        match at.blueprint {
            None => Some(self.branches[branch].row),
            key => self.rows.iter().position(|r| *r == (branch, key)),
        }
    }

    /// A blueprint's row, in its category before Favorites or Recent.
    fn blueprint(&self, key: ResKey) -> Option<usize> {
        let of = |category: bool| {
            self.rows.iter().position(|(b, k)| {
                *k == Some(key)
                    && matches!(self.branches[*b].branch, Branch::Category(_)) == category
            })
        };
        of(true).or_else(|| of(false))
    }

    /// What `key` does with the cursor at `row`, as in any tree: Up and
    /// Down go through the rows showing; Left closes an open branch, and
    /// from a closed one or a blueprint goes to the branch it is in;
    /// Right opens a closed branch, and from an open one goes into it.
    fn step(&self, row: usize, key: egui::Key) -> Step {
        if let Some(step) = self.step_pictures(row, key) {
            return step;
        }
        if let Some(step) = self.step_far(row, key) {
            return step;
        }
        let (branch, blueprint) = self.rows[row];
        let next = (row + 1..self.rows.len()).find(|r| self.shows(*r));
        match key {
            egui::Key::ArrowDown => next.map_or(Step::Stay, Step::To),
            egui::Key::ArrowUp => {
                (0..row).rev().find(|r| self.shows(*r)).map_or(Step::Stay, Step::To)
            }
            egui::Key::ArrowLeft if blueprint.is_none() && self.branches[branch].open => {
                Step::Fold(branch, false)
            }
            egui::Key::ArrowLeft => {
                self.within(row).map_or(Step::Stay, |b| Step::To(self.branches[b].row))
            }
            egui::Key::ArrowRight if blueprint.is_none() && !self.branches[branch].open => {
                Step::Fold(branch, true)
            }
            egui::Key::ArrowRight if blueprint.is_none() => {
                next.filter(|r| self.within(*r) == Some(branch)).map_or(Step::Stay, Step::To)
            }
            _ => Step::Stay,
        }
    }
}

impl Listed {
    /// [`Listed::step`] by Home and End (the first and the last row
    /// showing) and by Page Up and Page Down (the farthest row showing a
    /// list's height in sight away, headings and closed headings counted
    /// as rows; in a row of pictures, the one nearest across; at least the
    /// next row). `None`: another key.
    fn step_far(&self, row: usize, key: egui::Key) -> Option<Step> {
        use egui::Key::{End, Home, PageDown, PageUp};
        let mut showing = (0..self.rows.len()).filter(|r| self.shows(*r));
        let to = match key {
            Home => showing.next(),
            End => showing.next_back(),
            PageDown | PageUp => {
                let down = key == PageDown;
                let here = self.tops.get(&row).copied().unwrap_or(0.0);
                let reach = here + if down { self.page } else { -self.page };
                let within = |r: &usize| {
                    let top = self.tops.get(r).copied().unwrap_or(here);
                    if down { top <= reach + 0.5 } else { top >= reach - 0.5 }
                };
                let past: Vec<usize> = match down {
                    true => (row + 1..self.rows.len()).filter(|r| self.shows(*r)).collect(),
                    false => (0..row).rev().filter(|r| self.shows(*r)).collect(),
                };
                let reached: Vec<usize> = past.iter().copied().take_while(within).collect();
                match reached.last() {
                    // (The farthest line reached: of a row of pictures, the
                    // picture nearest across.)
                    Some(&far) => {
                        let line = self.tops.get(&far).copied();
                        let x = self.places.get(&row).map(|r| r.center().x);
                        let across = |r: &usize| match (x, self.places.get(r)) {
                            (Some(x), Some(p)) => (p.center().x - x).abs(),
                            _ => 0.0,
                        };
                        // (Left to right: from a heading, the first.)
                        let mut same: Vec<usize> = (reached.iter().copied())
                            .filter(|r| self.tops.get(r).copied() == line)
                            .collect();
                        same.sort_unstable();
                        same.into_iter().min_by(|a, b| across(a).total_cmp(&across(b)))
                    }
                    None => past.first().copied(),
                }
            }
            _ => return None,
        };
        Some(to.filter(|to| *to != row).map_or(Step::Stay, Step::To))
    }

    /// [`Listed::step`] from a picture in the Gallery: Up and Down to the
    /// nearest picture of the row of pictures above or below (past the
    /// first or the last row, out of the category's pictures), Left and
    /// Right to the picture before or after it (from the first, Left goes
    /// to the category). `None`: not a picture's row.
    fn step_pictures(&self, row: usize, key: egui::Key) -> Option<Step> {
        let here = *self.places.get(&row)?;
        let (branch, _) = self.rows[row];
        let mut same: Vec<(usize, egui::Rect)> = (self.rows.iter().enumerate())
            .filter(|(r, (b, k))| *b == branch && k.is_some() && self.places.contains_key(r))
            .map(|(r, _)| (r, self.places[&r]))
            .collect();
        same.sort_by_key(|(r, _)| *r);
        let at = same.iter().position(|(r, _)| *r == row)?;
        let past = |down: bool| {
            let found = if down {
                let last = same.last().map_or(row, |s| s.0);
                (last + 1..self.rows.len()).find(|r| self.shows(*r))
            } else {
                let first = same.first().map_or(row, |s| s.0);
                (0..first).rev().find(|r| self.shows(*r))
            };
            found.map_or(Step::Stay, Step::To)
        };
        let line = |down: bool| {
            // (The next row of pictures: the nearest top past this one's,
            // then the picture nearest across.)
            let beyond = |r: &egui::Rect| {
                if down { r.top() > here.bottom() - 1.0 } else { r.bottom() < here.top() + 1.0 }
            };
            let top = same
                .iter()
                .filter(|(_, r)| beyond(r))
                .map(|(_, r)| r.top())
                .min_by(|a, b| if down { a.total_cmp(b) } else { b.total_cmp(a) })?;
            same.iter()
                .filter(|(_, r)| (r.top() - top).abs() < 1.0)
                .min_by(|a, b| {
                    let off = |r: &egui::Rect| (r.center().x - here.center().x).abs();
                    off(&a.1).total_cmp(&off(&b.1))
                })
                .map(|(r, _)| *r)
        };
        Some(match key {
            egui::Key::ArrowDown => line(true).map_or_else(|| past(true), Step::To),
            egui::Key::ArrowUp => line(false).map_or_else(|| past(false), Step::To),
            egui::Key::ArrowRight => same.get(at + 1).map_or(Step::Stay, |s| Step::To(s.0)),
            egui::Key::ArrowLeft if at > 0 => Step::To(same[at - 1].0),
            _ => return None,
        })
    }
}

/// A row of a [`FlatList`] as it was drawn.
#[derive(Debug, Clone, Copy)]
struct FlatRow {
    /// How many headings it is inside.
    depth: usize,
    /// A heading: whether it is open.
    open: Option<bool>,
    /// Enter opens or closes it (a heading with no action of its own).
    enter_folds: bool,
    /// Where its top is.
    top: f32,
}

/// The arrow keys' cursor in a list drawn as a flat run of rows (the
/// tileset palette's headings and brushes, the Prefabs list), moved through
/// the rows as they were drawn the frame before: Up and Down go a row, Page
/// Up and Page Down a list's height, Home and End to the first and the last;
/// Left and Right close and open a heading (Left from a row, or from a
/// closed heading, goes to the heading it is in); Enter acts on the row, as
/// a click does. A row asks the list for a place as it is drawn
/// ([`FlatList::slot`]), says where it is ([`FlatList::seen`], which marks
/// the cursor) and what the keys asked of it ([`FlatList::asked`]); a click
/// puts the cursor on it ([`FlatList::click`]); [`FlatList::keys`] at the
/// frame's start reads the keys and [`FlatList::end`] closes the frame.
#[derive(Debug)]
pub(crate) struct FlatList<K> {
    /// The row the keys are at: the one clicked last, or come to with them.
    pub(crate) cursor: Option<K>,
    rows: Vec<(K, FlatRow)>,
    next: Vec<(K, FlatRow)>,
    /// A heading to open or close this frame.
    fold: Option<(K, bool)>,
    /// A row Enter acts on this frame.
    enter: Option<K>,
    /// The cursor's row is to be brought into view: the keys moved it.
    reveal: bool,
    /// The list's height in sight, for Page Up and Page Down.
    page: f32,
    /// The frame the cursor was asked to go to the first row in (Down out
    /// of the search, Next Pane): asked for that frame alone.
    first: Option<u64>,
    /// A widget in the list (a grid of pictures) has the arrow keys of its
    /// own, as drawn last, and this: as drawn now.
    busy: bool,
    busy_now: bool,
}

impl<K> Default for FlatList<K> {
    fn default() -> Self {
        FlatList {
            cursor: None,
            rows: Vec::new(),
            next: Vec::new(),
            fold: None,
            enter: None,
            reveal: false,
            page: 0.0,
            first: None,
            busy: false,
            busy_now: false,
        }
    }
}

impl<K: Clone + PartialEq> FlatList<K> {
    /// The frame's start: the keys pressed, as the palette's have them (the
    /// palette has the keys and no text is typed), done on the rows as they
    /// were drawn last.
    pub(crate) fn keys(&mut self, ui: &egui::Ui) {
        self.fold = None;
        self.enter = None;
        let ctx = ui.ctx();
        // (Not behind a dialog, a menu or a popover.)
        let free = !crate::shell::keys_blocked(ctx)
            && !mg_uikit::popover::Popover::any_open(ctx)
            && !egui::Popup::is_any_open(ctx);
        let first = self.first.take() == Some(ctx.cumulative_frame_nr())
            && free
            && keys_are(ctx, Keys::Palette)
            && !self.rows.is_empty();
        let live =
            free && keys_are(ctx, Keys::Palette) && !ctx.egui_wants_keyboard_input() && !self.busy;
        let key = match first {
            true => Some(egui::Key::Home),
            false => live.then(|| list_key(ui)).flatten(),
        };
        let Some(key) = key.filter(|_| !self.rows.is_empty()) else { return };
        ctx.request_repaint();
        let at = self.cursor.as_ref().and_then(|c| self.rows.iter().position(|(k, _)| k == c));
        let Some(at) = at else {
            // (No row: the first, or from Up, End and Page Up the last. A
            // cursor whose row is no longer drawn was moved to the nearest
            // one as the rows were drawn: [`FlatList::end`].)
            let up = matches!(key, egui::Key::ArrowUp | egui::Key::End | egui::Key::PageUp);
            let to = if up { self.rows.len() - 1 } else { 0 };
            if key != egui::Key::Enter {
                self.cursor = Some(self.rows[to].0.clone());
                self.reveal = true;
            }
            return;
        };
        let rows: Vec<FlatRow> = self.rows.iter().map(|(_, r)| *r).collect();
        if key == egui::Key::Enter {
            match rows[at] {
                FlatRow { open: Some(open), enter_folds: true, .. } => {
                    self.fold = Some((self.rows[at].0.clone(), !open));
                }
                _ => self.enter = Some(self.rows[at].0.clone()),
            }
            return;
        }
        match flat_step(&rows, at, key, self.page) {
            Step::To(row) => {
                self.cursor = Some(self.rows[row].0.clone());
                self.reveal = true;
            }
            Step::Fold(row, open) => self.fold = Some((self.rows[row].0.clone(), open)),
            Step::Stay => {}
        }
    }

    /// Asks for the cursor to go to the first row, as the frame is drawn
    /// (and not after it).
    pub(crate) fn ask_first(&mut self, ctx: &egui::Context) {
        self.first = Some(ctx.cumulative_frame_nr());
    }

    /// Whether the list has rows, and no widget of its own has the arrow
    /// keys: the keys move its cursor.
    pub(crate) fn takes_keys(&self) -> bool {
        !self.rows.is_empty() && !self.busy
    }

    /// The row the cursor is at, as it was drawn: whether it is a heading
    /// (open or closed) that Enter opens and closes.
    pub(crate) fn cursor_row(&self) -> Option<(Option<bool>, bool)> {
        let at = self.cursor.as_ref()?;
        let (_, row) = self.rows.iter().find(|(k, _)| k == at)?;
        Some((row.open, row.enter_folds))
    }

    /// A row of `key` about to be drawn, `depth` headings deep: its slot.
    pub(crate) fn slot(&mut self, key: K, depth: usize) -> usize {
        let row = FlatRow { depth, open: None, enter_folds: false, top: 0.0 };
        self.next.push((key, row));
        self.next.len() - 1
    }

    /// Slot `slot` drawn at `rect`: marked where the cursor is, in the
    /// keys' ring while the palette has them, and brought into view if the
    /// keys moved the cursor there. `open`: a heading, open or closed;
    /// `enter_folds`: Enter opens or closes it.
    pub(crate) fn seen(
        &mut self,
        ui: &egui::Ui,
        slot: usize,
        rect: egui::Rect,
        open: Option<bool>,
        enter_folds: bool,
    ) {
        let (key, row) = &mut self.next[slot];
        *row = FlatRow { depth: row.depth, open, enter_folds, top: rect.top() };
        if self.cursor.as_ref() != Some(&*key) {
            return;
        }
        if keys_are(ui.ctx(), Keys::Palette) {
            let t = mg_uikit::tokens(ui.ctx());
            let stroke = egui::Stroke::new(1.0, t.accent);
            let radius = ui.visuals().widgets.inactive.corner_radius;
            ui.painter().rect_stroke(rect.shrink(0.5), radius, stroke, egui::StrokeKind::Inside);
        }
        if self.reveal {
            ui.scroll_to_rect(rect, None);
        }
    }

    /// What the keys ask of slot `slot`: whether Enter acts on it, and
    /// whether a heading is to open or close.
    pub(crate) fn asked(&self, slot: usize) -> (bool, Option<bool>) {
        let key = &self.next[slot].0;
        let enter = self.enter.as_ref() == Some(key);
        let fold = self.fold.as_ref().filter(|(k, _)| k == key).map(|(_, open)| *open);
        (enter, fold)
    }

    /// Slot `slot` clicked: the cursor goes there and the palette has the
    /// keys.
    pub(crate) fn click(&mut self, ctx: &egui::Context, slot: usize) {
        self.cursor = Some(self.next[slot].0.clone());
        self.reveal = false;
        // (A picture grid that had the keyboard's focus lets it go.)
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
        self.busy = false;
        give_keys(ctx, Keys::Palette);
    }

    /// A widget in the list has the arrow keys (a grid of pictures with the
    /// keyboard's focus): the cursor keeps still while it does.
    pub(crate) fn busy(&mut self) {
        self.busy_now = true;
    }

    /// The frame's end: the rows drawn are the ones the keys move through.
    /// `page`: the list's height in sight.
    pub(crate) fn end(&mut self, page: f32) {
        let was = std::mem::replace(&mut self.rows, std::mem::take(&mut self.next));
        // (A cursor whose row is not drawn any more (searched out, under a
        // heading closed) goes to the heading it was in, else the row that
        // took its place.)
        let at = self.cursor.as_ref();
        if let Some(i) = at.and_then(|c| was.iter().position(|(k, _)| k == c))
            && !self.rows.iter().any(|(k, _)| Some(k) == at)
            && !self.rows.is_empty()
        {
            let depth = was[i].1.depth;
            let parent = (0..i)
                .rev()
                .filter(|&j| was[j].1.depth < depth)
                .find(|&j| self.rows.iter().any(|(k, _)| *k == was[j].0));
            let to = match parent {
                Some(j) => self.rows.iter().position(|(k, _)| *k == was[j].0).unwrap_or(0),
                None => i.min(self.rows.len() - 1),
            };
            self.cursor = Some(self.rows[to].0.clone());
        }
        self.busy = std::mem::take(&mut self.busy_now);
        self.reveal = false;
        self.page = page;
    }
}

/// The key of a list the palette has the keys of (an arrow, Enter, Home,
/// End, Page Up or Page Down, pressed alone).
fn list_key(ui: &egui::Ui) -> Option<egui::Key> {
    use egui::Key::{
        ArrowDown, ArrowLeft, ArrowRight, ArrowUp, End, Enter, Home, PageDown, PageUp,
    };
    ui.input(|i| {
        [ArrowUp, ArrowDown, ArrowLeft, ArrowRight, Enter, Home, End, PageUp, PageDown]
            .into_iter()
            .find(|k| i.modifiers.is_none() && i.key_pressed(*k))
    })
}

/// What `key` does at row `at` of a flat list's rows (all of them drawn:
/// the children of a closed heading are not there): as [`Listed::step`] for
/// the blueprint palette.
fn flat_step(rows: &[FlatRow], at: usize, key: egui::Key, page: f32) -> Step {
    use egui::Key::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp, End, Home, PageDown, PageUp};
    let last = rows.len() - 1;
    let to = |row: usize| if row == at { Step::Stay } else { Step::To(row) };
    match key {
        ArrowDown => to((at + 1).min(last)),
        ArrowUp => to(at.saturating_sub(1)),
        Home => to(0),
        End => to(last),
        PageDown | PageUp => {
            let down = key == PageDown;
            let reach = rows[at].top + if down { page } else { -page };
            let within = |r: &usize| match down {
                true => rows[*r].top <= reach + 0.5,
                false => rows[*r].top >= reach - 0.5,
            };
            let farthest = match down {
                true => (at + 1..=last).take_while(within).last(),
                false => (0..at).rev().take_while(within).last(),
            };
            // (At least the next row.)
            let next = if down { (at + 1).min(last) } else { at.saturating_sub(1) };
            to(farthest.unwrap_or(next))
        }
        ArrowLeft | ArrowRight => match (rows[at].open, key) {
            (Some(true), ArrowLeft) => Step::Fold(at, false),
            (Some(false), ArrowRight) => Step::Fold(at, true),
            (Some(true), ArrowRight) => (at < last && rows[at + 1].depth > rows[at].depth)
                .then_some(at + 1)
                .map_or(Step::Stay, Step::To),
            // (Left: out to the heading it is in.)
            (_, _) if key == ArrowLeft => {
                (0..at).rev().find(|r| rows[*r].depth < rows[at].depth).map_or(Step::Stay, Step::To)
            }
            _ => Step::Stay,
        },
        _ => Step::Stay,
    }
}

/// Whether the arrow keys are the palette's: from a click in its tree
/// until the pointer is in an area's view again (they move its camera
/// there, as W, A, S and D go on doing all along). The palette moves by
/// them with the pointer over it.
pub(crate) fn has_arrows(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new("palette-arrow-keys"))).unwrap_or(false)
}

/// Gives the arrow keys to the palette, or back to the area's view.
pub(crate) fn give_arrows(ctx: &egui::Context, palette: bool) {
    give_keys(ctx, if palette { Keys::Palette } else { Keys::Area });
}

/// Which list the keys are: the one clicked last, until the pointer is in
/// an area's view again. They are that list's wherever the pointer is
/// meanwhile (GitHub issue 12: they were only with the pointer over it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Keys {
    Area,
    Palette,
    Tree,
    /// A list of a window's own (Module Properties' haks).
    List,
    /// The talk table editor's lines.
    Talk,
    /// The resource browser's list.
    Browser,
    /// A table's cells, in the 2DA editor.
    Table,
    /// A conversation's lines, in the conversation editor.
    Dialog,
    /// The log, scrolled by them (its lines are text to select and copy).
    Log,
    /// The Right Dock's Preview: they turn its model
    /// (`area_tools::preview_body`).
    Preview,
    /// The Outliner's rows, under the Left Dock's tabs (`outliner.rs`).
    Outliner,
    /// A conversation's bookmarks in the Outliner (M4's U15), under the
    /// Left Dock's tabs or in its window where it floats: the conversation
    /// `Outliner::marks_keys` names (`outliner::bookmarks`).
    Marks,
    /// The Right Dock's Tile Properties (M4's U5): no arrows of its own;
    /// F6 gives its first control the keyboard's focus, and Tab goes on
    /// through its controls (`tile_select::panel`).
    TileProperties,
    /// The Right Dock's Properties (M4's U6): no arrows of its own; F6
    /// gives its page rail's page shown the keyboard's focus, and Tab goes
    /// on through its controls (`properties_panel::panel`).
    Properties,
    /// A script's Reference (M4's U12): no arrows of its own; F6 gives its
    /// filter the keyboard's focus (`script_reference::reference`).
    Reference,
    /// A script's Help (M4's U12): text to read, nothing to take them.
    Help,
    /// The Bottom Dock's Compiler, Bookmarks and Search Results (M4's
    /// U13): no arrows of their own; F6 gives the panel the keyboard's
    /// focus, from the script's text (`script_panels::begin`).
    Compiler,
    Bookmarks,
    SearchResults,
    /// A conversation's Data, Search and Conversation panels (M4's U14):
    /// no arrows of their own; F6 gives the panel's first field the
    /// keyboard's focus (`dialog_view::node_data` and the others).
    NodeData,
    NodeSearch,
    Conversation,
    /// A table's Cell and References panels (M4's U16): no arrows of their
    /// own; F6 gives Cell's Value and References' button the keyboard's
    /// focus (`table_view::panels`).
    Cell,
    TableReferences,
    /// A talk table's String and Talk Table panels (M4's U17): no arrows of
    /// their own; F6 gives String's Text and the Talk Table panel's first
    /// control the keyboard's focus (`talk_view::panels`). Its References
    /// are a table's, [`Keys::TableReferences`].
    TalkString,
    TalkFile,
    /// A NUI designer's Components and Bindings panels (M4's U18): no
    /// arrows of their own; F6 gives Components' Find and the Bindings
    /// panel's first field the keyboard's focus (`nui_view::panels`). Its
    /// Properties are [`Keys::Properties`], its Layers [`Keys::Marks`].
    NuiComponents,
    NuiBindings,
    /// A NUI designer's Checks (M4's U19), in the Bottom Dock: no arrows of
    /// its own; F6 gives its first row's control the focus.
    NuiChecks,
    /// Problems (M4's U20), in the Bottom Dock: F6 gives its first row's
    /// link the focus; Up, Down, Page Up, Page Down, Home and End choose a
    /// finding, Enter goes to it (M4's U28).
    Problems,
    /// A tileset's Textures (M4's U20): no arrows of their own; F6 gives
    /// the first texture's Copy the focus.
    Textures,
    /// A tileset's tile table (M4's U20): Up, Down, Page Up, Page Down,
    /// Home and End choose a tile, while its tileset is the active editor.
    Tiles,
    /// A hak's table of resources (M4's U21): Up, Down, Page Up, Page Down,
    /// Home and End choose a resource, Enter views it, while its hak is the
    /// active editor.
    Hak,
    /// A hak's Details (M4's U21): no arrows of its own; F6 gives its
    /// Description the keyboard's focus.
    HakDetails,
    /// A journal's Entry and Category (M4's U22): no arrows of their own;
    /// F6 gives the panel's first field the keyboard's focus.
    JournalEntry,
    JournalCategory,
    /// The faction editor's matrix (M4's U22): the arrows choose a cell,
    /// a digit starts typing its value, Enter types or sets it, while the
    /// faction editor is the active editor.
    Factions,
    /// The faction editor's Faction, Reaction and Reactions (M4's U22): F6
    /// gives the panel's first control the keyboard's focus.
    FactionPanel,
    Reaction,
    Reactions,
    /// The Appearance Gallery's Appearance panel (M4's U26): facts to
    /// read, no arrows; F6 there takes the keys from the panes.
    AppearanceFacts,
    /// The Bottom Dock's report tabs (M4's U28): Hak Pak Conflict
    /// Analysis', Resources Used's and the Claims Table's tables, whose
    /// arrows choose a row and Ctrl + C copies it.
    HakReport,
    ResourcesUsed,
    ClaimsTable,
    /// The area editor's tool sections of the Right Dock (M4's U29): Edit
    /// Areas Together's areas list (Up, Down, Page Up, Page Down, Home and
    /// End, Space ticks), Custom Light's and Bake Walkmesh to Transform's
    /// Enter (their main action) and Escape (their Cancel), held over the
    /// area's view from their command until a press elsewhere.
    AreasTogether,
    LightMaker,
    Bake,
}

pub(crate) fn give_keys(ctx: &egui::Context, to: Keys) {
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("palette-arrow-keys"), to != Keys::Area);
        d.insert_temp(egui::Id::new("arrow-keys-of"), to);
        d.remove_temp::<bool>(egui::Id::new(KEYS_HELD));
    });
}

/// The keys given to `to` by a command (a report opened, a search's
/// results, Verify's findings: M4's U28 review, the orchestrator's ruling):
/// they stay its while the pointer moves or rests over an area's view,
/// until the pointer is pressed somewhere ([`let_go_of_held_keys`]) or
/// another pane is given them.
pub(crate) fn hold_keys(ctx: &egui::Context, to: Keys) {
    give_keys(ctx, to);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(KEYS_HELD), true));
}

const KEYS_HELD: &str = "arrow-keys-held";

/// Whether the keys are held by the pane a command gave them to
/// ([`hold_keys`]).
pub(crate) fn keys_held(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new(KEYS_HELD))).unwrap_or(false)
}

/// Each frame's start: a press anywhere ends a command's hold on the keys
/// (they stay where they are, to be taken by a click as any pane's are).
pub(crate) fn let_go_of_held_keys(ctx: &egui::Context) {
    if ctx.input(|i| i.pointer.any_pressed()) {
        ctx.data_mut(|d| d.remove_temp::<bool>(egui::Id::new(KEYS_HELD)));
    }
}

/// A thin outline round the pane the keys are (`rect`), so that it shows
/// where the arrow keys go.
pub(crate) fn keys_outline(ui: &egui::Ui, rect: egui::Rect, list: Keys) {
    keys_outline_when(ui, rect, keys_are(ui.ctx(), list));
}

/// [`keys_outline`] where the caller knows whether the list has the keys
/// (a hak's or a tileset's table, whose keys are one's of several open).
pub(crate) fn keys_outline_when(ui: &egui::Ui, rect: egui::Rect, has: bool) {
    if has {
        let stroke = egui::Stroke::new(1.0, mg_uikit::tokens(ui.ctx()).accent);
        ui.painter().rect_stroke(rect.shrink(1.0), 2.0, stroke, egui::StrokeKind::Inside);
    }
}

/// Whether the arrow keys are the log's (it took them with a click on
/// it). For the window's tests.
#[doc(hidden)]
pub fn keys_are_the_logs(ctx: &egui::Context) -> bool {
    keys_are(ctx, Keys::Log)
}

/// Which pane the arrow keys are, by name (`Area`, `Tree`, `Palette`,
/// `Log`, `Browser`...: the area's view where no list took them). For
/// the window's tests (F6's order).
#[doc(hidden)]
pub fn arrow_keys_are(ctx: &egui::Context) -> String {
    let of = ctx.data(|d| d.get_temp::<Keys>(egui::Id::new("arrow-keys-of")));
    match of {
        Some(keys) if has_arrows(ctx) => format!("{keys:?}"),
        _ => format!("{:?}", Keys::Area),
    }
}

/// Whether a list drawn before the editor area has the keys: the module
/// tree's, the Resources panel's, the Outliner's, the log's (each takes
/// Escape, and the tree and the Outliner Enter, before the editor area
/// sees them).
pub(crate) fn a_list_has_the_keys(ctx: &egui::Context) -> bool {
    [Keys::Tree, Keys::Browser, Keys::Outliner, Keys::Marks, Keys::Log]
        .into_iter()
        .any(|k| keys_are(ctx, k))
}

/// Whether the blueprint palette's Gallery is shown and has the keys (its
/// hints are the Status Bar's then: `area_view::bar`).
pub(crate) fn gallery_has_the_keys(app: &Moonglow, ctx: &egui::Context) -> bool {
    app.settings.palette_gallery
        && !app.palette.tiles
        && pictured(app.palette.kind)
        && keys_are(ctx, Keys::Palette)
}

/// The Status Bar's hints for the tileset palette's rows or the Prefabs
/// list while the palette has the keys (and none for the blueprint
/// palettes' lists, which have their own): empty otherwise.
pub(crate) fn list_hints(app: &Moonglow, ctx: &egui::Context) -> Vec<mg_uikit::status::Hint> {
    use mg_uikit::status::Hint;
    let view = &app.palette;
    if !keys_are(ctx, Keys::Palette) || crate::shell::keys_blocked(ctx) {
        return Vec::new();
    }
    let mut hints = Vec::new();
    if view.tiles && view.area.is_some() {
        let list = &view.tile_list;
        if !list.takes_keys() {
            return hints;
        }
        hints.push(Hint::either(["Up", "Down"], "Move in the list"));
        hints.push(Hint::either(["Left", "Right"], "Close or open a heading"));
        hints.extend(match list.cursor_row() {
            Some((Some(open), true)) => {
                Some(Hint::keys(["Enter"], if open { "Close" } else { "Open" }))
            }
            Some(_) => Some(Hint::keys(["Enter"], "Choose")),
            None => None,
        });
    } else if view.prefabs && view.prefab_list.takes_keys() {
        hints.push(Hint::either(["Up", "Down"], "Move in the list"));
        if view.prefab_list.cursor.is_some() {
            hints.push(Hint::keys(["Enter"], "Place"));
        }
    }
    hints
}

pub(crate) fn keys_are(ctx: &egui::Context, list: Keys) -> bool {
    has_arrows(ctx)
        && ctx.data(|d| d.get_temp::<Keys>(egui::Id::new("arrow-keys-of"))) == Some(list)
}

/// Says that the letters and digits typed this frame are a list's (the
/// module tree's, while it has the keys, which goes to a row by them):
/// [`letters_taken`].
pub(crate) fn take_letters(ctx: &egui::Context) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("letters-taken-in"), pass));
}

/// Whether the letters and digits are a list's: the conversation tree's
/// while it has the keys, or the module tree's this frame
/// ([`take_letters`]). The area camera's keys held down that type them
/// (W, A, S, D, Z, C, the keypad's digits) don't move it meanwhile: a
/// letter typed to go to a line went to the camera too.
pub(crate) fn letters_taken(ctx: &egui::Context) -> bool {
    keys_are(ctx, Keys::Dialog)
        || ctx.data(|d| d.get_temp::<u64>(egui::Id::new("letters-taken-in")))
            == Some(ctx.cumulative_pass_nr())
}

impl PaletteView {
    /// Puts `palette` in place of the game's standard palette of `kind`:
    /// for the tests (a palette of thousands of blueprints).
    #[doc(hidden)]
    pub fn set_standard(&mut self, kind: BlueprintKind, palette: Palette) {
        self.standard.insert(kind, Arc::new(palette));
        self.fold_fields = None;
        self.node_meta = None;
        self.palette_total = None;
        self.found = None;
    }

    /// The name of the prefab the arrow keys' cursor is at in the Prefabs
    /// list. For the window's tests.
    #[doc(hidden)]
    pub fn prefab_cursor(&self) -> Option<&str> {
        self.prefab_list.cursor.as_deref()
    }

    /// The name of the row of the tileset palette the arrow keys' cursor is
    /// at (a heading's or a brush's). For the window's tests.
    #[doc(hidden)]
    pub fn tile_cursor(&self) -> Option<String> {
        use crate::terrain_mode::TileRow;
        Some(match self.tile_list.cursor.as_ref()? {
            TileRow::Select => "Select Tiles".to_string(),
            TileRow::Folder(_, name) | TileRow::Brush(name, _) => name.clone(),
        })
    }

    /// Whether the arrow keys' cursor is at a category's own row (not at
    /// a blueprint's).
    pub fn at_category(&self) -> bool {
        self.cursor.as_ref().is_some_and(|c| c.blueprint.is_none())
    }

    /// The row the arrow keys' cursor is at: its category's place in the
    /// palette as shown (grouped, its group's first), and the blueprint's
    /// where it is at one's. None at Favorites or Recent.
    pub fn cursor_row(&self) -> Option<(Vec<usize>, Option<ResKey>)> {
        let c = self.cursor.as_ref()?;
        match &c.branch {
            Branch::Category(p) => Some((p.clone(), c.blueprint)),
            Branch::Remembered(_) => None,
        }
    }

    /// Forgets palettes built from the game data (after Reload Resources).
    /// Forgets what was read of `sets` (tilesets kept in the module,
    /// changed): their palettes, names and grass, and a tile brush or
    /// choice of one of them, a number in its file.
    pub(crate) fn forget_tilesets(&mut self, sets: &[ResRef]) {
        self.tile_palettes.retain(|t, _| !sets.contains(t));
        self.tileset_names.retain(|t, _| !sets.contains(t));
        self.tileset_grass.borrow_mut().retain(|t, _| !sets.contains(t));
        if self.tile_brush.as_ref().is_some_and(|b| sets.contains(&b.tileset)) {
            self.tile_brush = None;
            self.tile_choice = None;
        }
    }

    pub(crate) fn forget_game_data(&mut self) {
        self.standard.clear();
        self.custom_cache.clear();
        self.own = None;
        self.grouped.clear();
        self.kept = Default::default();
        self.tile_palettes.clear();
        self.tileset_names.clear();
        self.tileset_grass.borrow_mut().clear();
        // (A tile, terrain, crosser or group is a number in its tileset's
        // file, which may be another now.)
        self.tile_brush = None;
        self.tile_choice = None;
        self.categories.clear();
        // (The Standard palette's tags read the module's blueprint first:
        // another module, or the game read again, has its own.)
        self.tags.clear();
        self.tags_read.clear();
        self.made.clear();
        self.fold_fields = None;
        self.node_meta = None;
        self.palette_total = None;
        self.found = None;
    }
}

impl Default for PaletteView {
    fn default() -> PaletteView {
        PaletteView {
            kind: BlueprintKind::Creature,
            custom: false,
            restored: false,
            to_first: false,
            filter: String::new(),
            standard: HashMap::new(),
            custom_cache: HashMap::new(),
            made: HashMap::new(),
            own: None,
            grouped: HashMap::new(),
            selected: None,
            thumb: None,
            hover_since: None,
            fold: None,
            about: None,
            found: None,
            arrived: Default::default(),
            tags: HashMap::new(),
            tags_read: HashMap::new(),
            hover_tag: None,
            tags_read_one_by_one: 0,
            chosen: Vec::new(),
            tiles: false,
            prefabs: false,
            start: false,
            placing_start: false,
            lights: false,
            placing_light: None,
            placing_appearance: None,
            tile_brush: None,
            tile_choice: None,
            area: None,
            tile_palettes: HashMap::new(),
            tileset_names: HashMap::new(),
            tileset_grass: Default::default(),
            categories: HashMap::new(),
            cursor: None,
            tile_list: FlatList::default(),
            prefab_list: FlatList::default(),
            toggle: None,
            reveal: false,
            kept: Default::default(),
            fold_fields: None,
            node_meta: None,
            palette_total: None,
            open_synced: Vec::new(),
            open_set: Default::default(),
        }
    }
}

/// The categories of `kind`'s palette, as a blueprint's Category lists
/// them: the module's own (Palette › Categories…), else the game's; in
/// the palette's order, or by name (Options › General), group by group.
/// Made again when the module changes or the game data is read again, not
/// every frame.
pub fn categories(app: &mut Moonglow, kind: BlueprintKind) -> Arc<Vec<mg_rules::Choice>> {
    let by_name = app.settings.categories_by_name;
    let made_for = (app.ws.as_ref().map(mg_edit::Workspace::revision), by_name);
    if let Some((at, made)) = app.palette.categories.get(&kind)
        && *at == made_for
    {
        return made.clone();
    }
    let Some(game) = app.game.as_deref() else { return Arc::default() };
    let own = app.ws.as_mut().and_then(|ws| ws.doc(&kind.skeleton_key()).ok().cloned());
    let skeleton = own.or_else(|| {
        let data = game.resman.get(&kind.skeleton_key()).ok()?;
        Gff::read(&data).ok()
    });
    let categories: Vec<mg_rules::Choice> = skeleton
        .map(|g| Palette::read(&g).categories(game))
        .unwrap_or_default()
        .into_iter()
        .map(|(id, text)| mg_rules::Choice { row: usize::from(id), text })
        .collect();
    let categories =
        Arc::new(if by_name { mg_rules::by_path(categories, " / ") } else { categories });
    app.palette.categories.insert(kind, (made_for, categories.clone()));
    categories
}

/// A resref for a copy of `from`: the resref without its trailing digits
/// and the first free three-digit number, within 16 characters.
pub fn copy_resref(from: &str, taken: &dyn Fn(&str) -> bool) -> Option<ResRef> {
    copy_resref_named(from, &Default::default(), taken)
}

/// [`copy_resref`] by a project's naming: the builder's code in the
/// team's pattern around the stem (a copy of `nw_chicken` is
/// `ab_nw_chicken001`; of `ab_chicken001`, `ab_chicken002`).
pub fn copy_resref_named(
    from: &str,
    naming: &mg_module::project_file::Naming,
    taken: &dyn Fn(&str) -> bool,
) -> Option<ResRef> {
    let stem = naming.strip(from).trim_end_matches(|c: char| c.is_ascii_digit());
    naming.numbered(mg_module::palette_add::first_chars(stem, 13), 1, taken)
}

/// Opens one of the game's blueprints to look at (the module's own of
/// that name, if it has one, to edit).
pub(crate) fn view_blueprint(app: &mut Moonglow, key: ResKey) {
    let (Some(game), Some(ws)) = (app.game.as_deref(), app.ws.as_mut()) else { return };
    if !ws.module.contains(&key) {
        let Some(gff) = game.resman.get(&key).ok().and_then(|d| Gff::read(&d).ok()) else {
            app.log.warn(format!("{key} could not be read"));
            return;
        };
        ws.view(key, gff);
    }
    if let Some(t) = Tab::for_resource(key) {
        app.actions.push(Action::OpenTab(t));
    }
}

/// A palette: the game's standard one, or the module's custom one (its
/// `<type>palcus.itp` with the blueprints changed since the module was
/// saved, again after each change: the entries of hak blueprints, of the
/// override folder's and of blueprints there are no more too, as Aurora
/// lists them); cached.
pub(crate) fn palette(
    app: &mut Moonglow,
    kind: BlueprintKind,
    custom: bool,
) -> Option<Arc<Palette>> {
    let game = app.game.as_deref()?;
    let view = &mut app.palette;
    if !custom {
        let p = view.standard.entry(kind).or_insert_with(|| {
            Arc::new(
                game.resman
                    .get_named(&format!("{}palstd", kind.name()), mg_core::ResType::ITP)
                    .ok()
                    .and_then(|d| Gff::read(&d).ok())
                    .map(|g| Palette::read(&g))
                    .unwrap_or_default(),
            )
        });
        return Some(p.clone());
    }
    let ws = app.ws.as_mut()?;
    let rev = ws.revision();
    if let Some((r, _)) = view.custom_cache.get_mut(&kind)
        && *r != rev
        && untouched(ws, *r, kind)
    {
        *r = rev;
    }
    if view.custom_cache.get(&kind).is_none_or(|(r, _)| *r != rev) {
        // The palette as the next save writes it (as Aurora shows it): the
        // module's, with the blueprints changed since it was saved.
        let built = ws
            .flush()
            .ok()
            .and_then(|()| custom_palette_now(&ws.module, ws.saved(), game, kind, &mut view.made))
            .map(|g| Palette::read(&g))
            .unwrap_or_default();
        view.custom_cache.insert(kind, (rev, Arc::new(built)));
    }
    view.custom_cache.get(&kind).map(|(_, p)| p.clone())
}

/// The custom palette of `kind` as the next save writes it: `module`'s,
/// with the blueprints changed since it was saved (`saved`).
///
/// A module that has no such palette (a project's, whose palettes are
/// made when it is built) has it made from every blueprint of the type:
/// once, kept in `made` with the module it was made from, and brought up
/// to date from there with the blueprints changed since, as a module's
/// own is. (Made anew at every change, a search left in the Custom
/// palette read and parsed every blueprint again at each command.)
fn custom_palette_now(
    module: &mg_module::Module,
    saved: &mg_module::Module,
    game: &mg_rules::GameData,
    kind: BlueprintKind,
    made: &mut HashMap<BlueprintKind, mg_module::Module>,
) -> Option<Gff> {
    let key = kind.custom_key();
    if module.contains(&key) {
        made.remove(&kind);
        return match update_custom_palette(module, saved, game, kind).ok()? {
            Some(g) => Some(g),
            None => module.gff(&key)?.ok(),
        };
    }
    // The module as it is, with the palette made before in it: brought
    // up to date from the module that one was made from.
    let mut now = module.clone();
    let palette = match made.get(&kind).and_then(|was| Some((was, was.shared(&key)?.clone()))) {
        Some((was, itp)) => {
            now.set(key, itp);
            match update_custom_palette(&now, was, game, kind).ok()? {
                Some(g) => g,
                // (As it was: kept as it is, for the next time too.)
                None => return was.gff(&key)?.ok(),
            }
        }
        None => mg_module::palette::rebuild_custom_palette(module, game, kind).ok()?,
    };
    now.set(key, palette.to_bytes().ok()?);
    made.insert(kind, now);
    Some(palette)
}

/// More › Refresh Palette (Aurora's, on a palette's right-click menu): the
/// module's custom palette of `kind` made anew from the module's
/// blueprints and its haks' (`rebuild_custom_palette`, as Build's
/// Palettes makes each), as one command: Undo takes it back, and the
/// next save writes it. Entries made stale by other means (another tool,
/// a merge, files changed while the module was closed) are put right so;
/// those of the override folder's blueprints and of blueprints there are
/// no more go, as in Aurora.
pub(crate) fn refresh(app: &mut Moonglow, kind: BlueprintKind) {
    let label = kind.label();
    let (Some(game), Some(ws)) = (app.game.as_deref(), app.ws.as_mut()) else { return };
    let made = ws.flush().map_err(|e| e.to_string()).and_then(|()| {
        let g = mg_module::palette::rebuild_custom_palette(&ws.module, game, kind)?;
        g.to_bytes().map_err(|e| e.to_string())
    });
    let key = kind.custom_key();
    match made {
        Err(e) => app.log.error(format!("Refresh Palette ({label}): {e}")),
        Ok(bytes) if ws.module.get(&key) == Some(&bytes[..]) => {
            app.log.info(format!("Refresh Palette: the {label} palette was up to date"));
        }
        Ok(bytes) => {
            let edit = Edit::SetResource { key, data: Some(bytes) };
            match app.apply(Command::new(format!("Refresh {label} Palette"), vec![edit])) {
                Ok(()) => app.log.info(format!("Refresh Palette: the {label} palette made anew")),
                Err(e) => app.log.error(format!("Refresh Palette ({label}): {e}")),
            }
        }
    }
}

/// Remove from Palette, on a custom palette's entry whose blueprint isn't
/// the module's (a hak's, the override folder's or one there is no more:
/// Aurora's palette lists them as its file has them, and so does
/// Moonglow's): the entry taken out of the palette as the next save
/// writes it, as one command (Undo puts it back); the blueprint is left
/// where it is. Build's Palettes lists a hak's blueprint again.
pub(crate) fn remove_entry(app: &mut Moonglow, kind: BlueprintKind, resref: ResRef) {
    let (Some(game), Some(ws)) = (app.game.as_deref(), app.ws.as_mut()) else { return };
    let key = kind.custom_key();
    let made = ws.flush().map_err(|e| e.to_string()).and_then(|()| {
        let shown = update_custom_palette(&ws.module, ws.saved(), game, kind)?;
        let mut g = match shown {
            Some(g) => g,
            None => match ws.module.gff(&key) {
                Some(g) => g.map_err(|e| e.to_string())?,
                None => return Ok(None),
            },
        };
        if !mg_module::palette::remove_entries(&mut g, resref) {
            return Ok(None);
        }
        g.to_bytes().map(Some).map_err(|e| e.to_string())
    });
    match made {
        Err(e) => app.log.error(format!("Remove from Palette: {e}")),
        Ok(None) => app.log.info(format!("Remove from Palette: {resref} has no entry")),
        Ok(Some(bytes)) => {
            let edit = Edit::SetResource { key, data: Some(bytes) };
            let cmd = Command::new(format!("Remove {resref} from the Palette"), vec![edit]);
            if let Err(e) = app.apply(cmd) {
                app.log.error(format!("Remove from Palette: {e}"));
            }
        }
    }
}

/// Builds flat pre-order subtree metadata once per `(nodes, found)` pair,
/// and the places of the blueprints the search finds in each node.
fn build_node_meta(
    nodes: &[PaletteNode],
    found: Option<&std::collections::HashSet<ResRef>>,
) -> (Arc<[NodeMeta]>, Arc<[u32]>) {
    fn push(
        node: &PaletteNode,
        found: Option<&std::collections::HashSet<ResRef>>,
        out: &mut Vec<NodeMeta>,
        picks: &mut Vec<u32>,
    ) -> (u32, u32) {
        let idx = out.len();
        out.push(NodeMeta::default());
        let mut total = node.blueprints.len() as u32;
        let picks_from = picks.len() as u32;
        if let Some(f) = found {
            let found = (node.blueprints.iter().enumerate())
                .filter(|(_, b)| f.contains(&b.resref))
                .map(|(i, _)| i as u32);
            picks.extend(found);
        }
        let own = match found {
            None => total,
            Some(_) => picks.len() as u32 - picks_from,
        };
        let mut shown = own;
        for child in &node.children {
            let (ct, cs) = push(child, found, out, picks);
            total += ct;
            shown += cs;
        }
        let subtree_len = (out.len() - idx) as u32;
        out[idx] = NodeMeta { total, shown, subtree_len, picks_from, picks: own };
        (total, shown)
    }
    let (mut out, mut picks) = (Vec::with_capacity(nodes.len() * 2), Vec::new());
    for node in nodes {
        push(node, found, &mut out, &mut picks);
    }
    (Arc::from(out), Arc::from(picks))
}

/// What a search finds in a palette, kept until the search, the palette
/// or the module changes: the blueprints, and how (by words, or by letters
/// in order when nothing has every word).
fn found(
    app: &mut Moonglow,
    palette: &Arc<Palette>,
    kind: BlueprintKind,
    custom: bool,
    tags: &Arc<HashMap<ResRef, String>>,
) -> (Arc<std::collections::HashSet<ResRef>>, Needle) {
    let revision = app.ws.as_ref().map(|w| w.revision());
    let key = (kind, custom, app.palette.filter.clone(), revision, app.game_names());
    if let Some((k, _, _)) = &mut app.palette.found
        && (k.0, k.1, &k.2, k.4) == (key.0, key.1, &key.2, key.4)
        && let (Some(was), Some(ws)) = (k.3, app.ws.as_ref())
        && Some(was) != revision
        && untouched(ws, was, kind)
    {
        k.3 = revision;
    }
    if let Some((k, f, find)) = &app.palette.found
        && *k == key
    {
        return (f.clone(), find.clone());
    }
    let Some(game) = app.game.as_deref() else { return Default::default() };
    let cached_fields = app.palette.fold_fields.as_ref().is_some_and(|c| {
        Arc::ptr_eq(&c.palette, palette)
            && Arc::ptr_eq(&c.tags, tags)
            && c.kind == kind
            && c.custom == custom
            && c.revision == revision
            && c.names == app.game_names()
    });
    if !cached_fields {
        let built: Vec<(ResRef, [String; 3])> = palette
            .blueprints()
            .into_iter()
            .map(|(_, b)| {
                let tag = tags.get(&b.resref).map(|t| mg_core::text::fold(t)).unwrap_or_default();
                (b.resref, [mg_core::text::fold(&b.name.text(game)), b.resref.to_string(), tag])
            })
            .collect();
        app.palette.fold_fields = Some(FoldCache {
            palette: palette.clone(),
            tags: tags.clone(),
            kind,
            custom,
            revision,
            names: app.game_names(),
            fields: Arc::new(built),
        });
    }
    let fields = app.palette.fold_fields.as_ref().expect("built above").fields.clone();
    // Words, each in the name, the ResRef or the tag; or, when nothing has
    // every word, letters in order (`lngswd` finds Longsword).
    let mut find = Needle::new(&app.palette.filter);
    let search = |find: &Needle| -> std::collections::HashSet<ResRef> {
        fields
            .iter()
            .filter(|(_, f)| find.finds_in(&[&f[0], &f[1], &f[2]]))
            .map(|(r, _)| *r)
            .collect()
    };
    let mut hits = search(&find);
    if hits.is_empty() {
        find = find.letters_in_order();
        hits = search(&find);
    }
    // The same search of the same palette, the module changed: what it
    // finds now that it didn't (their categories open).
    if let Some(((k, c, f, _, _), was, _)) = &app.palette.found
        && (*k, *c, f) == (key.0, key.1, &key.2)
    {
        app.palette.arrived = hits.difference(was).copied().collect();
    }
    let hits = Arc::new(hits);
    app.palette.found = Some((key, hits.clone(), find.clone()));
    (hits, find)
}

/// Whether the module's changes since `revision` leave what a palette of
/// `kind` made of the module as it was: they are known
/// ([`mg_edit::Workspace::touched_since`]) and touched no blueprint of the
/// kind and no palette. (An edit of an area made the custom palette, its
/// blueprints' tags, a search's finds and the module's own anew: with
/// 8,000 items and a search typed, an edit each frame took 14 ms a frame.)
fn untouched(ws: &mg_edit::Workspace, revision: u64, kind: BlueprintKind) -> bool {
    ws.touched_since(revision).is_some_and(|touched| {
        (touched.iter())
            .all(|t| t.key.restype != kind.restype() && t.key.restype != mg_core::ResType::ITP)
    })
}

/// The tags of a palette's blueprints (read from each once; for a custom
/// palette, again after the module changes).
fn tags(
    app: &mut Moonglow,
    palette: &Palette,
    kind: BlueprintKind,
    custom: bool,
) -> Arc<HashMap<ResRef, String>> {
    let revision = if custom { app.ws.as_ref().map(|w| w.revision()) } else { None };
    if let Some((r, t)) = app.palette.tags.get_mut(&(kind, custom)) {
        if let (Some(was), Some(ws)) = (*r, app.ws.as_ref())
            && *r != revision
            && untouched(ws, was, kind)
        {
            *r = revision;
        }
        if *r == revision {
            return t.clone();
        }
    }
    let (Some(game), ws) = (app.game.as_deref(), app.ws.as_ref()) else { return Arc::default() };
    let module = ws.map(|w| &w.module);
    let (tags, _) = tags_read(palette, kind, module, game, &mut app.palette.tags_read);
    let tags = Arc::new(tags);
    app.palette.tags.insert((kind, custom), (revision, tags.clone()));
    tags
}

/// A blueprint's tag as it was read: from which bytes of the module's
/// (held, and told by `Arc::ptr_eq`; `None`: the game's or a hak's), and
/// the tag (`None`: not read, or it has none).
type TagRead = (Option<Arc<[u8]>>, Option<String>);

/// The tags of `palette`'s blueprints, and how many of them were read for
/// it: one read before (`kept`) from the same bytes is not read again.
/// (With a search typed in the Custom palette, every command anywhere
/// copied and parsed every blueprint of the type again for its tag.)
fn tags_read(
    palette: &Palette,
    kind: BlueprintKind,
    module: Option<&mg_module::Module>,
    game: &mg_rules::GameData,
    kept: &mut HashMap<ResKey, TagRead>,
) -> (HashMap<ResRef, String>, usize) {
    let (mut tags, mut read) = (HashMap::new(), 0);
    let mut listed = std::collections::HashSet::new();
    for (_, b) in palette.blueprints() {
        let k = ResKey::new(b.resref, kind.restype());
        listed.insert(k);
        let own = module.and_then(|m| m.shared(&k));
        let same = |from: &Option<Arc<[u8]>>| match (from, own) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (a, b) => a.is_none() && b.is_none(),
        };
        let tag = match kept.get(&k) {
            Some((from, tag)) if same(from) => tag.clone(),
            _ => {
                read += 1;
                let tag = (own.map(|d| Gff::read(d)))
                    .or_else(|| game.resman.get(&k).ok().map(|d| Gff::read(&d)))
                    .and_then(Result::ok)
                    .and_then(|g| g.root.string("Tag").map(crate::text::decode));
                kept.insert(k, (own.cloned(), tag.clone()));
                tag
            }
        };
        if let Some(tag) = tag {
            tags.insert(b.resref, tag);
        }
    }
    // (Those of the type the palette lists no more: let go of.)
    kept.retain(|k, _| k.restype != kind.restype() || listed.contains(k));
    (tags, read)
}

enum Pick {
    Edit(ResKey),
    /// One of the game's, opened to look at.
    View(ResKey),
    EditCopy(ResKey),
    Delete(ResKey),
    /// A custom palette's entry whose blueprint isn't the module's, taken
    /// out of the palette ([`remove_entry`]).
    RemoveEntry(ResKey),
    Preview(ResKey),
    /// The objects selected in the area become ones of this blueprint
    /// (keeping their own tags, names, scripts and variables).
    Replace(ResKey, bool),
    References(ResKey),
    /// Its Tag to the clipboard (its ResRef is copied from the menu).
    CopyTag(ResKey),
    UpdateInstances(Vec<ResKey>),
    EditTogether(Vec<ResKey>),
    /// The Export window, for these blueprints of the module's.
    Export(Vec<ResKey>),
    /// A new blueprint of the palette's kind: its wizard, as New… opens.
    New,
    /// Into a custom category (its palette id).
    MoveTo(ResKey, u8),
    /// Added to (true) or removed from Favorites.
    Favorite(ResKey, bool),
    /// A plugin's command (its place among those for menus) run on these.
    Plugin(usize, Vec<ResKey>),
}

/// The Tag of the blueprint `key` (none: it has none), for the hovered
/// one: read from its bytes once and kept with the palette's search's
/// (`tags_read`), read again only where the module's bytes of it changed;
/// no palette is walked for it.
fn tag_of(app: &mut Moonglow, key: ResKey) -> Option<String> {
    let own = app.ws.as_ref().and_then(|w| w.module.shared(&key));
    let same = |from: &Option<Arc<[u8]>>| match (from, own) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (a, b) => a.is_none() && b.is_none(),
    };
    if let Some((from, tag)) = app.palette.tags_read.get(&key)
        && same(from)
    {
        return tag.clone();
    }
    let game = app.game.as_deref()?;
    let tag = (own.map(|d| Gff::read(d)))
        .or_else(|| game.resman.get(&key).ok().map(|d| Gff::read(&d)))
        .and_then(Result::ok)
        .and_then(|g| g.root.string("Tag").map(crate::text::decode));
    let own = own.cloned();
    app.palette.tags_read_one_by_one += 1;
    app.palette.tags_read.insert(key, (own, tag.clone()));
    tag
}

/// A blueprint's Tag: the module's as it is now (edits not yet saved in
/// it), else the game's (the load order's).
fn blueprint_tag(app: &mut Moonglow, key: ResKey) -> Option<String> {
    let tag = |g: &Gff| g.root.string("Tag").map(crate::text::decode);
    if let Some(ws) = app.ws.as_mut()
        && ws.module.contains(&key)
    {
        return ws.doc(&key).ok().and_then(tag);
    }
    let data = app.game.as_ref()?.resman.get(&key).ok()?;
    tag(&Gff::read(&data).ok()?)
}

/// Expand All and Collapse All as two small buttons, for a list's toolbar.
pub(crate) fn fold_icons(
    ui: &mut egui::Ui,
    what: &str,
    keymap: &crate::keys::Keymap,
) -> Option<bool> {
    use crate::keys::Cmd;
    let mut fold = None;
    let expand = keymap.titled(&format!("Open every {what}"), Cmd::ExpandAll, ui.ctx());
    let icon = |ui: &mut egui::Ui, icon, name, tip: &str| {
        crate::widgets::icon_button(ui, icon, name, tip).clicked()
    };
    if icon(ui, crate::icons::Icon::ExpandAll, "Expand All", &expand) {
        fold = Some(true);
    }
    let collapse = keymap.titled(&format!("Close every {what}"), Cmd::CollapseAll, ui.ctx());
    if icon(ui, crate::icons::Icon::CollapseAll, "Collapse All", &collapse) {
        fold = Some(false);
    }
    fold
}

/// Expand All's and Collapse All's keys (Options › Keyboard), pressed:
/// for the pane that has the keys.
pub(crate) fn fold_keys(ui: &egui::Ui, keymap: &crate::keys::Keymap) -> Option<bool> {
    use crate::keys::Cmd;
    ui.input_mut(|i| {
        if keymap.consume(i, Cmd::ExpandAll) {
            Some(true)
        } else if keymap.consume(i, Cmd::CollapseAll) {
            Some(false)
        } else {
            None
        }
    })
}

/// The palette's search: the pane's width, with what it looks in said
/// (`hint`), and a button to clear it. Its keys, as the module tree's
/// Filter has them (GitHub issue 12): Find's keys (Ctrl+F) while the keys
/// are the palette's go to it; Escape clears it; Escape again, or Down,
/// goes back to the list. Whether Down left it for the list.
fn search_row(ui: &mut egui::Ui, view: &mut PaletteView, hint: &str) -> bool {
    let search_id = egui::Id::new("palette-search-field");
    let left = crate::widgets::SearchField::new(search_id, &mut view.filter)
        .hint(hint)
        .find_when(keys_are(ui.ctx(), Keys::Palette))
        .down()
        .show(ui)
        .left;
    if left.is_some() {
        give_keys(ui.ctx(), Keys::Palette);
    }
    let entered = left == Some(crate::widgets::FilterLeft::Down);
    if entered {
        view.cursor = None;
    }
    entered
}

/// The start location's page: what it is, and the switch that has a click
/// in an area's view put it there.
fn start_ui(app: &mut Moonglow, ui: &mut egui::Ui, view: &mut PaletteView) {
    ui.label(
        "Where a player's character enters the module (module.ifo's Mod_Entry_Area and \
         position): a blue ring with a red arrow in the area's view.",
    );
    ui.add_space(4.0);
    let tip = "On, a click in an area's view puts the start location there";
    ui.toggle_value(&mut view.placing_start, "Place the Start Location").on_hover_text(tip);
    ui.add_space(4.0);
    let at = app.ws.as_mut().and_then(|ws| {
        let info = ws.doc(&crate::module_props::info_key()).ok()?;
        let f = |label: &str| info.root.float(label).unwrap_or(0.0);
        Some((info.root.resref("Mod_Entry_Area")?, f("Mod_Entry_X"), f("Mod_Entry_Y")))
    });
    match at {
        Some((area, x, y)) => {
            ui.weak(format!("Now in {area}, at {x:.2}, {y:.2}."));
            if ui.button("Go to Start Location").clicked() {
                crate::area_view::go_to_start(app);
            }
        }
        None => {
            ui.weak("The module has no start location yet.");
        }
    }
    ui.add_space(4.0);
    ui.weak(
        "In the view, select its marker by a click: drag its ring to move it, its arrow's tip to \
         turn it; Q and E turn it, and its menu's Adjust Location sets it exactly.",
    );
}

/// The Palette panel's width under which its types are a dropdown, not a
/// row of icons (proposal section 5.2: "the palette's type strip becomes a
/// combo under 300 px of panel width"; the Narrow board's 280 shows it).
pub(crate) const TYPE_COMBO_UNDER: f32 = 300.0;

/// The lights' page: the game's invisible light placeables ("Light,
/// White" and the six others, and any a hak adds in the same way), one
/// chosen to place with a click in an area's view, as the area's menu's
/// Add Light Here puts one.
fn lights_ui(app: &mut Moonglow, ui: &mut egui::Ui, view: &mut PaletteView) {
    ui.label(
        "The game's invisible lights: placeables that light what is around them and need no \
         custom content. Each shows in the area's view as a small sun in its color.",
    );
    ui.add_space(4.0);
    let lights = crate::area_view::stock_lights(app);
    if lights.is_empty() {
        ui.weak("The game's placeables.2da has none (no game data?).");
        return;
    }
    let tip = "Chosen, a click in an area's view puts this light there (Shift+click: more)";
    mg_uikit::with_rows(ui, |ui| {
        for light in lights.iter() {
            let on = view.placing_light == Some(light.row);
            if crate::area_view::light_toggle(ui, light, on).on_hover_text(tip).clicked() {
                view.placing_light = if on { None } else { Some(light.row) };
            }
        }
    });
    // Lights of any color and reach, as content: only with the option on
    // (Options › Areas, or the project's `[lights] generated`).
    if crate::light_maker::generated_lights(app) {
        let made = crate::area_view::made_lights(app);
        if !made.is_empty() {
            ui.separator();
            ui.weak("Made by Custom Light, in the module's haks:");
        }
        mg_uikit::with_rows(ui, |ui| {
            for light in made.iter() {
                let on = view.placing_light == Some(light.row);
                if crate::area_view::light_toggle(ui, light, on).on_hover_text(tip).clicked() {
                    view.placing_light = if on { None } else { Some(light.row) };
                }
            }
        });
        ui.add_space(4.0);
        let area = view.area.filter(|a| app.area_views.contains_key(a));
        let r = ui
            .add_enabled(area.is_some(), egui::Button::new("Custom Light…"))
            .on_hover_text("Make a light of your color and reach in a hak, then click to place it")
            .on_disabled_hover_text("Open an area first");
        if r.clicked()
            && let Some(area) = area
        {
            app.light_draft = Some(crate::light_maker::LightDraft::new(app, area, None));
            app.tools.bring = Some(crate::shell::Panel::LightMaker);
        }
    }
    ui.add_space(4.0);
    ui.weak(
        "Placed, a light is a placeable: move and raise it as any. Light Color on its \
         right-click menu gives it another color; selected, a ring shows where its light ends.",
    );
}

/// The least width the palette toolbar's size slider takes beside the
/// List and Gallery switch; with less room, a row of its own.
pub(crate) const SLIDER_LEAST: f32 = 60.0;

/// How long the pointer rests on a palette row before its blueprint's
/// picture and what there is to say of it are asked for: long enough that
/// a turning wheel, which scrolls another row under a resting pointer each
/// frame, asks for none (each would make a picture: M3 T14's review), and
/// well under the tooltip's delay (egui's half a second), so that the
/// picture is being made, or made, as the tooltip opens, and the tooltip
/// does not open without it and then grow.
pub(crate) const HOVER_REST: f64 = 0.12;

/// The tiles palette's tileset, as its row says it: "City Exterior
/// (tcn01)", its name in the game's language (read once, kept in
/// `tileset_names`), else its ResRef alone. `None` with no area or
/// tileset.
fn tileset_shown(app: &Moonglow, view: &mut PaletteView) -> Option<String> {
    let area = view.area?;
    let set = app.area_views.get(&area)?.terrain.as_ref()?;
    let tileset = set.tileset;
    let name = match view.tileset_names.get(&tileset) {
        Some(name) => name.clone(),
        None => {
            let general = &set.set.general;
            let game = app.game.as_ref();
            let name = (game.and_then(|g| g.string(general.display_name)))
                .or(general.unlocalized_name.clone())
                .unwrap_or_default();
            view.tileset_names.insert(tileset, name.clone());
            name
        }
    };
    Some(match name.is_empty() || name.eq_ignore_ascii_case(&tileset.to_string()) {
        true => tileset.to_string(),
        false => format!("{name} ({tileset})"),
    })
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui) {
    if app.game.is_none() {
        ui.label("No game data.");
        return;
    }
    // Laid out in the pane's visible width: its rows wrap there. (A pane
    // that scrolls sideways offers the width of what it last held, so a
    // row once too long would never wrap again.)
    let seen = ui.clip_rect().right() - ui.cursor().left();
    if seen > 0.0 && seen < ui.available_width() {
        ui.set_max_width(seen);
    }
    // (What the Gallery's enlargement stands beside: the Right Dock, the
    // palettes' one host (they never float: a dragged-out area's window
    // brings no Palette); the pane where it is not known yet.)
    let panel = app.right_rect.unwrap_or(ui.max_rect().intersect(ui.clip_rect()));
    let mut view = std::mem::take(&mut app.palette);
    // The palette shown when Moonglow was last closed, once.
    if !std::mem::replace(&mut view.restored, true) {
        // (And what was typed in its search, where something was.)
        if !app.settings.palette_search.is_empty() {
            view.filter = app.settings.palette_search.clone();
        }
        if let Some(last) = app.settings.palette_type.clone() {
            let kind = BlueprintKind::ALL
                .into_iter()
                .find(|k| k.restype().extension().is_some_and(|e| e == last));
            (view.tiles, view.prefabs) = (last == "tiles", last == "prefabs");
            view.lights = last == "lights";
            if let Some(kind) = kind {
                view.kind = kind;
                view.custom = app.settings.palette_custom && app.ws.is_some();
            }
        }
    }
    let custom_was = view.custom;
    if crate::trace::on() {
        let r = ui.max_rect();
        let key = (
            [r.min.x.to_bits(), r.min.y.to_bits(), r.max.x.to_bits(), r.max.y.to_bits()],
            view.tiles,
            view.prefabs,
            view.kind,
            view.custom,
            view.area,
            view.filter.as_str(),
        );
        crate::trace::changed_key("palette", &key, || {
            format!(
                "in {:?}; tiles {}, prefabs {}, kind {:?}, custom {}, area {:?}, find {:?}",
                ui.max_rect(),
                view.tiles,
                view.prefabs,
                view.kind,
                view.custom,
                view.area,
                view.filter
            )
        });
    }
    // The header (GitHub issue 16), top down by what each row governs:
    // the type, its name and the source, the search, then the list's own
    // toolbar right over the list.
    //
    // The types: one row of icons, a click each, named under the pointer
    // (and below, for the one shown); in a pane too narrow for the row, a
    // dropdown with their names.
    // (The types as a dropdown, the narrow dock's row: Expand All and
    // Collapse All then at the Group by row's end, as Narrow draws them.)
    let narrow_row: bool;
    {
        use crate::icons::{LIGHTS, PREFABS, START, TILES, object};
        #[derive(Clone, Copy, PartialEq)]
        enum Type {
            Tiles,
            Kind(BlueprintKind),
            Prefabs,
            Lights,
            Start,
        }
        let now = if view.tiles {
            Type::Tiles
        } else if view.prefabs {
            Type::Prefabs
        } else if view.lights {
            Type::Lights
        } else if view.start {
            Type::Start
        } else {
            Type::Kind(view.kind)
        };
        let mut types = vec![(Type::Tiles, TILES, "Tiles", "The area's tileset")];
        types.extend(BlueprintKind::ALL.map(|k| (Type::Kind(k), object(k), k.label(), "")));
        types.push((
            Type::Prefabs,
            PREFABS,
            "Prefabs",
            "Groups of objects saved under a name, to place again in any module",
        ));
        types.push((
            Type::Lights,
            LIGHTS,
            "Lights",
            "The game's invisible lights: click one, then click in an area",
        ));
        types.push((
            Type::Start,
            START,
            "Start Location",
            "Click in an area to put the start location there",
        ));
        let mut chosen = None;
        // (A blueprint type's source, the game's or the module's: Standard
        // and Custom, beside the type's name, or in the narrow dock beside
        // the types' dropdown, as Narrow draws them.)
        let sources = !view.tiles && !view.prefabs && !view.start && !view.lights;
        let source = |ui: &mut egui::Ui, view: &mut PaletteView| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_enabled_ui(app.ws.is_some(), |ui| {
                    ui.selectable_value(&mut view.custom, true, "Custom")
                        .on_hover_text("The module's blueprints");
                });
                ui.selectable_value(&mut view.custom, false, "Standard")
                    .on_hover_text("The game's blueprints");
            });
        };
        let width = |ui: &egui::Ui, text: &str, style: egui::TextStyle| {
            let font = style.resolve(ui.style());
            ui.fonts_mut(|f| f.layout_no_wrap(text.into(), font, egui::Color32::PLACEHOLDER))
                .size()
                .x
        };
        let toggles = |ui: &egui::Ui| {
            let pad = ui.spacing().button_padding.x * 2.0;
            width(ui, "Standard", egui::TextStyle::Button)
                + width(ui, "Custom", egui::TextStyle::Button)
                + 2.0 * pad
                + ui.spacing().item_spacing.x
        };
        // Icon buttons (22 px, 1 apart: the board's `.seg.icons`, all
        // thirteen on one row in the Right Dock's 320 px), in rows where the
        // panel is narrower than them, down to [`TYPE_COMBO_UNDER`]: in a
        // narrower panel (the Narrow board's 280) a dropdown of them,
        // "Placeables". (Named by their types within the row, a group of
        // its own: the area header names its filters so too.)
        let each = mg_uikit::metrics::ICON_BUTTON + 1.0;
        // (The panel: this room and its body's margin, 8 each side.)
        let panel = ui.available_width() + 2.0 * mg_uikit::metrics::PANEL_PADDING;
        // (The button that folds the Right Dock away is beside its edge,
        // not among these: GitHub issue 10, `shell::right_dock`.)
        let as_row = panel >= TYPE_COMBO_UNDER && ui.available_width() >= each * 4.0;
        if as_row {
            ui.horizontal_wrapped(|ui| {
                mg_uikit::name_group(ui, "Palette types");
                ui.spacing_mut().item_spacing = egui::vec2(1.0, 2.0);
                for (t, icon, name, tip) in &types {
                    let tip =
                        if tip.is_empty() { name.to_string() } else { format!("{name}\n{tip}") };
                    let button = mg_uikit::IconButton::new(*icon, *name)
                        .icon_only(true)
                        .kind_hue()
                        .selectable(now == *t);
                    if ui.add(button).on_hover_text(tip).clicked() {
                        chosen = Some(*t);
                    }
                }
            });
        } else {
            let shown = types.iter().find(|t| t.0 == now).expect("one of them");
            let all: Vec<_> = types.iter().map(|t| t.0).collect();
            let mut pick = now;
            // (Its icon before it: what is shown, at a glance; Standard and
            // Custom after it, a blueprint type's.)
            ui.horizontal(|ui| {
                mg_uikit::hues::kind_icon(ui, shown.1);
                let after = match sources {
                    true => toggles(ui) + ui.spacing().item_spacing.x,
                    false => 0.0,
                };
                crate::widgets::dropdown("palette-type")
                    .selected_text(shown.2)
                    .width((ui.available_width() - after).max(60.0))
                    .show_with(
                        ui,
                        &mut pick,
                        || all.clone(),
                        |ui, pick| {
                            // (Its list named as the row of icons is.)
                            mg_uikit::name_group(ui, "Palette types");
                            for (t, _, name, _) in &types {
                                ui.selectable_value(pick, *t, *name);
                            }
                        },
                    );
                if sources {
                    source(ui, &mut view);
                }
            });
            if pick != now {
                chosen = Some(pick);
            }
        }
        // (Another type chosen: the start location is not about to be
        // placed any more.)
        if chosen.is_some_and(|t| t != Type::Start) {
            (view.start, view.placing_start) = (false, false);
        }
        if chosen.is_some_and(|t| t != Type::Lights) {
            (view.lights, view.placing_light) = (false, None);
        }
        match chosen {
            Some(Type::Tiles) => {
                view.tiles = true;
                view.prefabs = false;
            }
            Some(Type::Kind(kind)) => {
                if kind != view.kind {
                    view.chosen.clear();
                }
                view.kind = kind;
                view.tiles = false;
                view.prefabs = false;
            }
            Some(Type::Prefabs) => {
                view.prefabs = true;
                view.tiles = false;
                // (No blueprint is about to be placed any more.)
                view.selected = None;
                view.chosen.clear();
            }
            Some(Type::Lights) => {
                view.lights = true;
                (view.tiles, view.prefabs) = (false, false);
                view.selected = None;
                view.chosen.clear();
                view.tile_brush = None;
            }
            // Chosen, the start location is about to be placed, as
            // Aurora's Paint Start Location has it.
            Some(Type::Start) => {
                (view.start, view.placing_start) = (true, true);
                (view.tiles, view.prefabs) = (false, false);
                view.selected = None;
                view.chosen.clear();
                view.tile_brush = None;
            }
            None => {}
        }
        // Remembered for the next start.
        if chosen.is_some() && !view.start {
            app.settings.palette_type = Some(match (view.tiles, view.prefabs, view.lights) {
                (true, ..) => "tiles".to_string(),
                (_, true, _) => "prefabs".to_string(),
                (.., true) => "lights".to_string(),
                _ => view.kind.restype().extension().unwrap_or_default().to_string(),
            });
        }
        // The type shown, by name, with the source beside it (a blueprint
        // type's: the game's, or the module's); in the narrow dock the
        // dropdown names it and holds the source beside it (Narrow), the
        // tiles' row kept for the area's tileset.
        let name = types.iter().find(|t| t.0 == now).map_or("", |t| t.2);
        // (The source on a row of its own under the name where the two do
        // not fit side by side.)
        let gap = ui.spacing().item_spacing.x;
        let beside = width(ui, name, egui::TextStyle::Body) + gap * 2.0 + toggles(ui);
        let apart = as_row && sources && beside > ui.available_width();
        // (The tiles palette's: the area's tileset, by its name and
        // ResRef, at the row's end, as the AreaTiles board has it.)
        let tileset = view.tiles.then(|| tileset_shown(app, &mut view)).flatten();
        if as_row || tileset.is_some() {
            ui.horizontal(|ui| {
                ui.strong(name);
                if as_row && sources && !apart {
                    source(ui, &mut view);
                }
                if let Some(tileset) = &tileset {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(egui::RichText::new(tileset).weak()).truncate())
                            .on_hover_text("The area's tileset");
                    });
                }
            });
        }
        if apart {
            ui.horizontal(|ui| source(ui, &mut view));
        }
        narrow_row = !as_row;
    }
    if view.custom != custom_was {
        app.settings.palette_custom = view.custom;
        // (Bulk edits are on one palette's blueprints.)
        view.chosen.clear();
    }
    view.fold = None;
    // (What is typed is kept for the next start too.)
    if view.restored && view.filter != app.settings.palette_search {
        app.settings.palette_search = view.filter.clone();
    }
    if view.start {
        ui.separator();
        start_ui(app, ui, &mut view);
        app.palette = view;
        return;
    }
    if view.lights {
        ui.separator();
        lights_ui(app, ui, &mut view);
        app.palette = view;
        return;
    }
    if view.prefabs {
        ui.separator();
        // (Next Pane: the list's first row.)
        let from_pane = std::mem::take(&mut view.to_first) && view.prefab_list.cursor.is_none();
        if from_pane {
            view.prefab_list.ask_first(ui.ctx());
        }
        app.palette = view;
        prefabs_ui(app, ui);
        return;
    }
    if view.tiles {
        // As the blueprint palettes: the search, then a row of what there
        // is and how it is shown (the tile palette's own: its headings,
        // brushes and tiles), then the list.
        let entered = search_row(ui, &mut view, "Search name, model");
        // (Down out of the search, or Next Pane: the list's first row.)
        let from_pane = std::mem::take(&mut view.to_first) && view.tile_list.cursor.is_none();
        if entered || from_pane {
            view.tile_list.ask_first(ui.ctx());
        }
        app.palette = view;
        crate::terrain_mode::palette_ui(app, ui);
        return;
    }
    let entered = search_row(ui, &mut view, "Search name, tag, resref");
    // (Given the keys by Next Pane: at its first row, as Down from the
    // search goes.)
    let entered = entered || (std::mem::take(&mut view.to_first) && view.cursor.is_none());

    // The palette to show.
    let kind = view.kind;
    app.palette = view;
    let shown = palette(app, kind, app.palette.custom);
    let view = std::mem::take(&mut app.palette);
    let Some(palette) = shown else {
        ui.label("No module open.");
        app.palette = view;
        return;
    };
    let custom = view.custom;
    // Tags are read once a palette is searched (and shown on hover then).
    let searching = !view.filter.trim().is_empty();
    // (The caches are the palette state's: put back while they're used.)
    app.palette = view;
    let tags = if searching { tags(app, &palette, kind, custom) } else { Arc::default() };
    let (found, find) = if searching {
        let (f, find) = found(app, &palette, kind, custom, &tags);
        (Some(f), find)
    } else {
        (None, Needle::default())
    };
    let mut view = std::mem::take(&mut app.palette);

    // Group by's choices (drawn under the toolbar, below): the palette's
    // categories, or one of the project's groupings offered in it.
    let groupings =
        (app.ws.as_ref().map(|ws| ws.module.project_file.palette.clone())).unwrap_or_default();
    let ext = kind.restype().extension().unwrap_or_default();
    let offered: Vec<&Grouping> = groupings.offered(kind).collect();
    // (In the narrow dock Expand All and Collapse All end the Group by
    // row, which is drawn there whether the project offers groupings or
    // not, as Narrow draws it: Category alone, grayed, where it offers
    // none.)
    let fold_by_group = narrow_row;
    // The list's toolbar, right over the list: every category opened or
    // closed, how many blueprints the list has (of the search, if any),
    // names or pictures, the pictures' size (with the Gallery on), a new
    // blueprint, and the rest under "more" (the GalleryPalette board's
    // order: the slider takes the room between the switch and New…, on a
    // row of its own under them where less than [`SLIDER_LEAST`] is left).
    let mut categories = None;
    let gallery_on = app.settings.palette_gallery && pictured(kind);
    ui.horizontal_wrapped(|ui| {
        if !fold_by_group {
            view.fold = fold_icons(ui, "category", &app.keymap);
        }
        // (Their keys too, while the keys are the palette's.)
        if view.fold.is_none()
            && keys_are(ui.ctx(), Keys::Palette)
            && !ui.ctx().egui_wants_keyboard_input()
        {
            view.fold = fold_keys(ui, &app.keymap);
        }
        let items = found.as_ref().map_or_else(
            || match &view.palette_total {
                Some((p, n)) if Arc::ptr_eq(p, &palette) => *n,
                _ => {
                    fn count(nodes: &[mg_module::palette::PaletteNode]) -> usize {
                        nodes.iter().map(|n| n.blueprints.len() + count(&n.children)).sum()
                    }
                    let n = count(&palette.nodes);
                    view.palette_total = Some((palette.clone(), n));
                    n
                }
            },
            |f| f.len(),
        );
        ui.weak(format!("{items} item{}", if items == 1 { "" } else { "s" })).on_hover_text(
            if searching { "Blueprints found" } else { "Blueprints in this palette" },
        );
        ui.separator();
        // List or Gallery: names, or pictures in their place for the types
        // that have them (the others are lists, whichever is chosen).
        let pictures = pictured(view.kind);
        let shown = app.settings.palette_gallery && pictures;
        let list = mg_uikit::IconButton::new(crate::icons::LIST, "List").icon_only(true);
        if ui.add(list.selected(!shown)).on_hover_text("Show the blueprints by name").clicked() {
            app.settings.palette_gallery = false;
        }
        ui.add_enabled_ui(pictures, |ui| {
            let gallery =
                mg_uikit::IconButton::new(crate::icons::GALLERY, "Gallery").icon_only(true);
            if ui
                .add(gallery.selected(shown))
                .on_hover_text("Show the blueprints as pictures, to choose by eye")
                .on_disabled_hover_text(
                    "Only creatures, doors, items, placeables and waypoints have pictures",
                )
                .clicked()
            {
                app.settings.palette_gallery = true;
            }
        });
        // (New… and More's width as drawn last time: the slider's room is
        // what is left before them.)
        let tail_id = egui::Id::new("palette-toolbar-tail");
        let tail = ui.data(|d| d.get_temp::<f32>(tail_id)).unwrap_or(120.0);
        let room = ui.available_size_before_wrap().x - tail - ui.spacing().item_spacing.x;
        let slider_here = shown && room >= SLIDER_LEAST;
        if slider_here {
            ui.scope(|ui| {
                ui.spacing_mut().slider_width = room;
                crate::appearance_gallery::size_slider(app, ui);
            });
        }
        let new = ui
            .add_enabled(app.ws.is_some(), egui::Button::new("New…"))
            .on_hover_text("A new blueprint (the type's wizard)");
        if new.clicked() {
            crate::commands::Id::new_blueprint(view.kind).run(app, ui.ctx());
        }
        let more = mg_uikit::menu_button(ui, "More", |ui| {
            mg_uikit::menu_rows(ui);
            if ui
                .add_enabled(app.ws.is_some(), egui::Button::new("Categories…"))
                .on_hover_text("Add, rename and remove this module's categories for this kind")
                .clicked()
            {
                categories = Some(view.kind);
                ui.close();
            }
            if ui
                .add_enabled(app.ws.is_some(), egui::Button::new("Refresh Palette"))
                .on_hover_text(
                    "Make the module's custom palette anew from its blueprints and its haks'",
                )
                .clicked()
            {
                app.actions.push(Action::RefreshPalette(view.kind));
                ui.close();
            }
            // Every appearance there is, not only the blueprints'.
            if view.kind == BlueprintKind::Placeable
                && ui
                    .button("All Appearances…")
                    .on_hover_text(
                        "Every placeable appearance as a picture, to give to the selected \
                         placeables",
                    )
                    .clicked()
            {
                app.actions.push(Action::PlaceableGallery);
                ui.close();
            }
        })
        .response
        .on_hover_text("The module's categories, Refresh Palette and every placeable appearance");
        // (Kept only where the two stand on one row.)
        if (new.rect.center().y - more.rect.center().y).abs() < 1.0 {
            let width = more.rect.right() - new.rect.left();
            if (width - tail).abs() > 0.5 {
                ui.data_mut(|d| d.insert_temp(tail_id, width));
                ui.ctx().request_discard("the palette's toolbar measured");
            }
        }
        if shown && !slider_here {
            // (A row of its own, the toolbar's width.)
            ui.end_row();
            ui.scope(|ui| {
                ui.spacing_mut().slider_width = ui.available_size_before_wrap().x.max(40.0);
                crate::appearance_gallery::size_slider(app, ui);
            });
        }
    });
    // A search typed opens every category with a match, as it is typed.
    // (So does another palette shown with the search still there.)
    let said = format!("{:?}/{}/{}/{}", view.kind, view.custom, view.tiles, view.filter);
    let typed = crate::widgets::text_changed(ui, egui::Id::new("palette-filter"), &said);
    if typed && !view.filter.trim().is_empty() {
        view.fold = view.fold.or(Some(true));
    }
    if let Some(kind) = categories {
        app.palette = std::mem::take(&mut view);
        crate::palette_categories::open(app, kind);
        view = std::mem::take(&mut app.palette);
    }
    // Group by: the palette's categories, or one of the project's
    // groupings offered in it (`[palette.*]` in moonglow.toml), each group
    // with the categories in it (Then by Category) or its blueprints in a
    // list. Not shown where the project offers none.
    let mut by = (app.settings.palette_group_by.get(ext))
        .and_then(|id| offered.iter().position(|g| g.id == *id));
    let mut nested = !app.settings.palette_flat.iter().any(|e| e == ext);
    // (And with the Gallery on, as the GalleryPalette and GalleryWide
    // boards draw it: Category alone, grayed, where the project offers no
    // grouping.)
    if !offered.is_empty() || fold_by_group || gallery_on {
        ui.horizontal_wrapped(|ui| {
            ui.label("Group by").on_hover_text(
                "The palette's categories, or a grouping from the project's moonglow.toml \
                 ([palette.*])",
            );
            let options: Vec<Option<usize>> =
                std::iter::once(None).chain((0..offered.len()).map(Some)).collect();
            let text = |o: &Option<usize>| o.map_or("Category", |i| offered[i].title.as_str());
            let mut pick = by;
            // (Room kept for Expand All and Collapse All after it.)
            let after = match fold_by_group {
                true => 2.0 * (mg_uikit::metrics::ICON_BUTTON + ui.spacing().item_spacing.x),
                false => 0.0,
            };
            ui.add_enabled_ui(!offered.is_empty(), |ui| {
                let width = (ui.available_width() - after - ui.spacing().item_spacing.x).max(80.0);
                crate::widgets::dropdown(("palette-group-by", kind))
                    .selected_text(text(&by))
                    .width(width)
                    .show(ui, &mut pick, &options, |o| text(o).to_string());
            })
            .response
            .on_disabled_hover_text(
                "The project offers no other grouping (moonglow.toml's [palette.*])",
            );
            if pick != by {
                match pick {
                    Some(i) => {
                        app.settings.palette_group_by.insert(ext.into(), offered[i].id.clone());
                    }
                    None => {
                        app.settings.palette_group_by.remove(ext);
                    }
                }
                by = pick;
            }
            if by.is_some()
                && ui
                    .check_box(&mut nested, "Then by Category")
                    .on_hover_text("Each group holds the palette's categories, as they are nested")
                    .changed()
            {
                app.settings.palette_flat.retain(|e| e != ext);
                if !nested {
                    app.settings.palette_flat.push(ext.into());
                }
            }
            if fold_by_group {
                // (At the row's end, Expand All then Collapse All: a block
                // laid out left to right, put at the right end.)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let both = 2.0 * mg_uikit::metrics::ICON_BUTTON + ui.spacing().item_spacing.x;
                    let size = egui::vec2(both + 4.0, ui.spacing().interact_size.y);
                    let ltr = egui::Layout::left_to_right(egui::Align::Center);
                    ui.allocate_ui_with_layout(size, ltr, |ui| {
                        view.fold = view.fold.or(fold_icons(ui, "category", &app.keymap));
                    });
                });
            }
        });
    }
    let game = app.game.as_deref().expect("checked");
    let grouping = by.map(|i| offered[i]);
    let grouped = grouping.map(|g| {
        let made_for = (palette.clone(), groupings.fingerprint, g.id.clone(), nested);
        let same = |at: &GroupedFor| {
            Arc::ptr_eq(&at.0, &made_for.0)
                && (at.1, &at.2, at.3) == (made_for.1, &made_for.2, nested)
        };
        match view.grouped.get(&(kind, custom)) {
            Some((at, made)) if same(at) => made.clone(),
            _ => {
                let started = std::time::Instant::now();
                let made = Arc::new(mg_module::palette_groups::group_palette(
                    &palette,
                    kind,
                    g,
                    &|n| n.text(game),
                    nested,
                ));
                crate::trace::changed("palette-grouped", || {
                    format!("{:?} by {} in {:?}", kind, g.id, started.elapsed())
                });
                view.grouped.insert((kind, custom), (made_for, made.clone()));
                made
            }
        }
    });
    ui.separator();
    // While Paint is on (in an area that is not read-only, where Paint's
    // header shows), the Brush section right under the list (the
    // AreaPaint board): the list as tall as its rows, but for the room
    // the section took last frame.
    let brush = app.paint
        && kind == BlueprintKind::Placeable
        && view.area.is_none_or(|a| crate::claims::read_only(app, a).is_none());
    let brush_id = egui::Id::new("palette-brush-height");
    let brush_room = brush.then(|| ui.data(|d| d.get_temp::<f32>(brush_id)).unwrap_or(120.0));
    let (resrefs, cr) = (app.settings.name_resrefs, !app.settings.palette_no_cr);
    let ext = kind.restype().extension().unwrap_or_default();
    // An arrow key pressed while the keys are the palette's, the pointer
    // is over it (not over a list elsewhere that has keys of its own) and
    // no text is being typed: the rows are listed as they are drawn, to
    // move through.
    let pressed = (keys_are(ui.ctx(), Keys::Palette) && !ui.ctx().egui_wants_keyboard_input())
        .then(|| {
            use egui::Key::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp, End, Enter, Home};
            use egui::Key::{PageDown, PageUp};
            ui.input(|i| {
                let plain = i.modifiers.is_none();
                [ArrowUp, ArrowDown, ArrowLeft, ArrowRight, Enter, Home, End, PageUp, PageDown]
                    .into_iter()
                    .find(|k| plain && i.key_pressed(*k))
            })
        })
        .flatten()
        // (Down out of the search: to the list's first row.)
        .or(entered.then_some(egui::Key::ArrowDown));
    let gallery = app.settings.palette_gallery && pictured(kind);
    let menu_commands = app.plugin_menu_commands_cached();
    // (The custom palette's blueprints the module has: the others, a
    // hak's, the override folder's or none, are looked at, not edited.)
    // (Listed once a revision of the module, not every frame.)
    let own = match (&app.ws, custom) {
        (Some(ws), true) => {
            let at = (ws.revision(), kind);
            if let Some(((was, k), _)) = &mut view.own
                && (*was, *k) != at
                && *k == kind
                && untouched(ws, *was, kind)
            {
                *was = at.0;
            }
            match &view.own {
                Some((was, own)) if *was == at => own.clone(),
                _ => {
                    let own: std::collections::HashSet<ResRef> =
                        ws.module.keys_of(kind.restype()).map(|k| k.resref).collect();
                    let own = Arc::new(own);
                    view.own = Some((at, own.clone()));
                    own
                }
            }
        }
        _ => Arc::default(),
    };
    let new_label = new_label(kind);
    let (meta, picks) = match &view.node_meta {
        Some(c)
            if Arc::ptr_eq(&c.palette, &palette)
                && match (&c.grouped, &grouped) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                    _ => false,
                }
                && match (&c.found, &found) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                    _ => false,
                } =>
        {
            (c.meta.clone(), c.picks.clone())
        }
        _ => {
            let nodes = grouped.as_ref().map_or(&palette.nodes, |g| &g.palette.nodes);
            let (meta, picks) = build_node_meta(nodes, found.as_deref());
            view.node_meta = Some(MetaCache {
                palette: palette.clone(),
                grouped: grouped.clone(),
                found: found.clone(),
                meta: meta.clone(),
                picks: picks.clone(),
            });
            (meta, picks)
        }
    };
    if view.open_synced != app.settings.palette_open {
        view.open_synced.clone_from(&app.settings.palette_open);
        view.open_set.clear();
        view.open_set.extend(view.open_synced.iter().cloned());
    }
    let plugin_kind =
        mg_area::ObjectKind::from_restype(kind.restype()).map(mg_plugin::Target::Blueprint);
    let has_plugin_menu = plugin_kind.as_ref().is_some_and(|k| {
        let kinds = std::slice::from_ref(k);
        menu_commands.iter().any(|c| c.fits(kinds))
    });
    let mut tree = Tree {
        game,
        kind,
        custom,
        grouping: grouping.map(|g| (g.id.clone(), nested)),
        groups: (grouped.as_ref())
            .map(|g| g.palette.nodes.iter().map(|n| n.name.text(game)).collect())
            .unwrap_or_default(),
        origin: grouped.as_ref().map(|g| &g.origin),
        own,
        fold: view.fold,
        open: &mut view.open_set,
        open_changed: false,
        new: app.ws.is_some().then_some(new_label.as_str()),
        found: found.as_deref(),
        arrived: std::mem::take(&mut view.arrived),
        tags: &tags,
        favorites: app
            .settings
            .palette_favorites
            .iter()
            .filter_map(|f| {
                let (pfx, rest) = f.split_once(':')?;
                (pfx == ext).then(|| ResRef::from_str(rest).ok()).flatten()
            })
            .collect(),
        thumb: view.thumb,
        gallery,
        side: crate::appearance_gallery::tile_side(app),
        // (The pictures made so far: of use in the Gallery alone.)
        ready: &app.thumbnails,
        shown: Vec::new(),
        about: view.about.as_ref().map(|(k, lines)| (*k, lines.as_slice())),
        hover_tag: view.hover_tag.as_ref().map(|(k, t)| (*k, t.as_deref())),
        sel: Selection { selected: view.selected, chosen: std::mem::take(&mut view.chosen) },
        picks: Vec::new(),
        hovered: None,
        enlarged: None,
        resrefs,
        cr,
        hide_empty: app.settings.palette_hides_empty,
        cursor: (view.cursor.take()).filter(|c| {
            c.kind == kind
                && c.custom == custom
                && c.grouping == grouping.map(|g| (g.id.clone(), nested))
        }),
        reveal: std::mem::take(&mut view.reveal),
        toggle: view.toggle.take(),
        listed: pressed.map(|_| Listed::default()),
        clicked: false,
        range: None,
        commands: &menu_commands,
        plugin_kind,
        has_plugin_menu,
        meta,
        found_at: picks,
    };
    if find.is_in_order() {
        ui.weak("No exact matches: close ones");
    }
    // (A scroll place of its own: not the tiles palette's, nor another
    // panel's.)
    let mut scroll =
        egui::ScrollArea::vertical().id_salt("palette-list").auto_shrink([false, brush]);
    if let Some(room) = brush_room {
        scroll = scroll.max_height((ui.available_height() - room).max(66.0));
    }
    let mut list = scroll.show(ui, |ui| {
        // A row is one line: a name too long for the pane is cut short
        // (the pointer over it shows it whole, and its ResRef, tag and
        // challenge rating), as in the module tree. A wider pane shows
        // more.
        if !tree.gallery {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            // (Rows 22 apart; a gallery's pictures keep their gaps.)
            mg_uikit::list_rows(ui);
        }
        if find.is_empty() {
            tree.remembered(ui, "Favorites", &app.settings.palette_favorites, &palette);
            tree.remembered(ui, "Recent", &app.settings.palette_recent, &palette);
        }
        // (Grouped: a node for each group, Other last.)
        let nodes = grouped.as_ref().map_or(&palette.nodes, |g| &g.palette.nodes);
        let mut meta_idx = 0;
        for (i, node) in nodes.iter().enumerate() {
            tree.node(ui, node, &[i], None, &mut meta_idx);
        }
        // The room under the list has the menu too.
        let below = if brush { 24.0 } else { ui.available_height().max(24.0) };
        let rest = egui::vec2(ui.available_width(), below);
        let (_, below) = ui.allocate_exact_size(rest, egui::Sense::click());
        if let Some(label) = tree.new {
            mg_uikit::context_menu(&below, |ui| {
                if ui.button(label).clicked() {
                    tree.picks.push(Pick::New);
                    ui.close();
                }
            });
        }
    });
    // (Home and End scroll the list where the keys are not the palette's:
    // where they are, they move its cursor, below.)
    if pressed.is_none() && !keys_are(ui.ctx(), Keys::Palette) {
        crate::widgets::home_and_end(ui, &mut list);
    }
    keys_outline(ui, list.inner_rect, Keys::Palette);
    if brush {
        let keys = crate::paint_mode::chosen_keys(tree.sel.selected, &tree.sel.chosen);
        let names: HashMap<ResRef, String> = if keys.is_empty() {
            HashMap::new()
        } else {
            (palette.blueprints().into_iter())
                .filter(|(_, b)| keys.iter().any(|k| k.resref == b.resref))
                .map(|(_, b)| (b.resref, shown_name(&b.name, game)))
                .collect()
        };
        ui.add_space(6.0);
        let shown = ui.scope(|ui| {
            crate::paint_mode::brush_section(ui, &keys, &|k| names.get(&k.resref).cloned())
        });
        let room = shown.response.rect.height() + 6.0 + ui.spacing().item_spacing.y;
        ui.data_mut(|d| d.insert_temp(brush_id, room));
        if let Some(key) = shown.inner {
            crate::paint_mode::take_out(&mut tree.sel.selected, &mut tree.sel.chosen, key);
        }
    }
    if tree.open_changed {
        let mut list: Vec<String> = tree.open.iter().cloned().collect();
        list.sort();
        app.settings.palette_open = list;
    }
    let shown = std::mem::take(&mut tree.shown);
    // The arrow keys: the cursor moves through the rows as they were
    // drawn, or its category opens or closes (when the palette is drawn
    // next). A blueprint it comes to is selected, as by a click; on a
    // category, the blueprint selected stays in hand.
    if let (Some(key), Some(mut listed)) = (pressed, tree.listed.take()) {
        listed.page = list.inner_rect.height();
        let from = tree
            .cursor
            .as_ref()
            .and_then(|c| listed.find(c))
            .filter(|r| listed.shows(*r))
            // (From the search: the first row, not the one after the
            // blueprint in hand.)
            .or_else(|| tree.sel.selected.filter(|_| !entered).and_then(|k| listed.blueprint(k)));
        let step = match from {
            // Enter: the blueprint's editor, as a double click's (a
            // standard one is viewed); a category opens or closes.
            Some(row) if key == egui::Key::Enter => {
                let (branch, blueprint) = listed.rows[row];
                match blueprint {
                    Some(k) if tree.cursor.is_some() => {
                        let mine = custom && tree.own.contains(&k.resref);
                        tree.picks.push(if mine { Pick::Edit(k) } else { Pick::View(k) });
                        Step::Stay
                    }
                    Some(_) => Step::Stay,
                    None => Step::Fold(branch, !listed.branches[branch].open),
                }
            }
            Some(row) => listed.step(row, key),
            // Nothing to start from: the first row, or the last.
            None => {
                let mut showing = (0..listed.rows.len()).filter(|r| listed.shows(*r));
                let to = match key {
                    egui::Key::ArrowDown | egui::Key::Home | egui::Key::PageDown => showing.next(),
                    egui::Key::ArrowUp | egui::Key::End | egui::Key::PageUp => showing.next_back(),
                    _ => None,
                };
                to.map_or(Step::Stay, Step::To)
            }
        };
        match step {
            Step::To(row) => {
                let (branch, blueprint) = listed.rows[row];
                let branch = listed.branches[branch].branch.clone();
                let grouping = tree.grouping.clone();
                tree.cursor = Some(At { kind, custom, grouping, branch, blueprint });
                if blueprint.is_some() {
                    tree.sel.selected = blueprint;
                    tree.sel.chosen.clear();
                }
                view.reveal = true;
            }
            Step::Fold(branch, open) => {
                view.toggle = Some((listed.branches[branch].branch.clone(), open));
            }
            Step::Stay => {}
        }
        ui.ctx().request_repaint();
    }
    if tree.clicked {
        give_arrows(ui.ctx(), true);
    }
    view.cursor = tree.cursor.take();
    let (sel, picks, hovered) = (tree.sel, tree.picks, tree.hovered);
    let enlarged = tree.enlarged.take();
    if view.open_synced != app.settings.palette_open {
        view.open_synced.clone_from(&app.settings.palette_open);
    }
    if sel.selected.is_some() && sel.selected != view.selected {
        view.tile_brush = None;
    }
    view.selected = sel.selected;
    view.chosen = sel.chosen;
    app.palette = view;
    // The Gallery's pictures in sight: kept while they are, and those
    // still to make are made a few a frame, the first in sight first, so
    // a long category doesn't hold the toolset up
    // (`model_view::make_thumbnails`).
    use crate::model_view::Pictured;
    app.thumbnails.show(shown.into_iter().map(Pictured::Resource));
    // The hovered blueprint's picture, for its tooltip once it is made,
    // and what there is to say of it: asked for once the pointer has
    // rested on its row for `HOVER_REST` (the same row a while: not the
    // rows a turning wheel scrolls under it), well before the tooltip
    // opens.
    let now = ui.input(|i| i.time);
    let delay = HOVER_REST;
    let since = match (hovered, app.palette.hover_since) {
        (Some(key), Some((was, since))) if was == key => Some(since),
        (Some(key), _) => {
            app.palette.hover_since = Some((key, now));
            Some(now)
        }
        (None, _) => {
            app.palette.hover_since = None;
            None
        }
    };
    let rested = since.is_some_and(|since| now - since >= delay);
    if let Some(since) = since.filter(|_| !rested) {
        ui.ctx().request_repaint_after_secs((since + delay - now).max(0.0) as f32);
    }
    let hovered = hovered.filter(|_| rested);
    if let Some(key) = hovered
        && let Some(id) = app.thumbnails.get(Pictured::Resource(key))
        && app.palette.thumb != Some((key, id))
    {
        app.palette.thumb = Some((key, id));
        ui.ctx().request_repaint();
    }
    if let Some(key) = hovered
        && app.palette.about.as_ref().is_none_or(|(k, _)| *k != key)
    {
        let lines = about(app, key);
        app.palette.about = Some((key, lines));
        ui.ctx().request_repaint();
    }
    // Its Tag, read for it alone (the search reads every blueprint's):
    // once, and kept.
    if let Some(key) = hovered
        && app.palette.hover_tag.as_ref().is_none_or(|(k, _)| *k != key)
    {
        let tag = match tags.get(&key.resref) {
            Some(tag) => Some(tag.clone()),
            None => tag_of(app, key),
        };
        app.palette.hover_tag = Some((key, tag));
        ui.ctx().request_repaint();
    }
    // The Gallery's enlargement: a tooltip beside the dock, never over it
    // or the cell it tells of, never outside the window, gone while
    // something is dragged (the List's tip is at the pointer, as before).
    if let Some(e) = enlarged.filter(|e| hovered == Some(e.key))
        && !egui::DragAndDrop::has_any_payload(ui.ctx())
    {
        enlargement(app, ui, &e, panel);
    }

    for pick in picks {
        match pick {
            Pick::Edit(key) => {
                if let Some(t) = Tab::for_resource(key) {
                    app.actions.push(Action::OpenTab(t));
                }
            }
            Pick::EditCopy(key) => app.copy_dialog(key, true),
            Pick::View(key) => view_blueprint(app, key),
            Pick::Delete(key) => {
                let cmd = Command::new(
                    format!("Delete {key}"),
                    vec![Edit::SetResource { key, data: None }],
                );
                if let Err(e) = app.apply(cmd) {
                    app.log.error(e.to_string());
                }
            }
            Pick::RemoveEntry(key) => remove_entry(app, kind, key.resref),
            Pick::New => crate::commands::Id::new_blueprint(kind).run(app, ui.ctx()),
            Pick::Preview(key) => app.actions.push(Action::OpenTab(Tab::Model(key))),
            Pick::Replace(key, own) => match app.palette.area {
                Some(area) => {
                    let n = crate::area_view::replace_selected(app, area, key, own);
                    if n > 0 {
                        app.log.info(format!("Replaced {n} with {}", key.resref));
                    }
                }
                None => app.log.warn("Open an area and select what to replace first"),
            },
            Pick::References(key) => app.actions.push(Action::FindReferences(key)),
            Pick::CopyTag(key) => match blueprint_tag(app, key) {
                Some(tag) => ui.ctx().copy_text(tag),
                None => app.log.warn(format!("{key}: its Tag couldn't be read")),
            },
            Pick::Plugin(i, keys) => {
                let on = keys.into_iter().map(mg_plugin::Selected::Blueprint).collect();
                app.actions.push(crate::plugins::run_on(&menu_commands[i], on));
            }
            Pick::UpdateInstances(keys) => app.update_instances_of(keys),
            Pick::EditTogether(keys) => app.actions.push(Action::OpenTab(Tab::Blueprints(keys))),
            Pick::Export(keys) => app.actions.push(Action::ExportDialog(keys)),
            Pick::MoveTo(key, id) => {
                let field = kind.palette_field();
                let current = app
                    .ws
                    .as_mut()
                    .and_then(|ws| ws.doc(&key).ok())
                    .and_then(|g| g.root.integer(field));
                if current.is_some_and(|c| c != i64::from(id)) {
                    app.actions.push(Action::Apply(Command::new(
                        format!("Move {} to another category", key.resref),
                        vec![Edit::SetField {
                            key,
                            path: mg_edit::GffPath::root(),
                            label: field.into(),
                            value: Some(Value::Byte(id)),
                        }],
                    )));
                }
            }
            Pick::Favorite(key, on) => {
                let list = &mut app.settings.palette_favorites;
                list.retain(|r| *r != remembered(key));
                if on {
                    list.push(remembered(key));
                }
            }
        }
    }
}

/// The blueprints picked in a palette: the one clicked, and more chosen
/// with Ctrl+click or Shift+click (custom palettes).
struct Selection {
    selected: Option<ResKey>,
    chosen: Vec<ResKey>,
}

impl Selection {
    fn has(&self, key: ResKey) -> bool {
        self.selected == Some(key) || self.chosen.contains(&key)
    }

    /// What a bulk edit on `key` applies to: the selection, if `key` is in
    /// it, else `key` alone.
    fn bulk(&self, key: ResKey) -> Vec<ResKey> {
        if !self.has(key) || self.chosen.is_empty() {
            return vec![key];
        }
        let mut keys: Vec<ResKey> = self.selected.into_iter().chain(self.chosen.clone()).collect();
        keys.sort();
        keys.dedup();
        keys
    }
}

/// How many blueprints a palette node and those under it hold.
/// Every blueprint under a palette node.
fn all_under(node: &PaletteNode, kind: BlueprintKind, out: &mut Vec<ResKey>) {
    out.extend(node.blueprints.iter().map(|b| ResKey::new(b.resref, kind.restype())));
    for c in &node.children {
        all_under(c, kind, out);
    }
}

/// A palette name as its row shows it: its text, or, where it is a
/// talk-table string that isn't there (a custom talk table that wasn't
/// found, a line the table lacks), which string it is, so that a row
/// without a name says why.
fn shown_name(name: &mg_module::palette::PaletteName, game: &mg_rules::GameData) -> String {
    let text = name.text(game);
    match name {
        mg_module::palette::PaletteName::StrRef(s) if text.trim().is_empty() => {
            let custom = mg_core::StrRef(*s).is_custom();
            format!("(no text: {}string {s})", if custom { "custom talk table " } else { "" })
        }
        _ => text,
    }
}

/// "New Creature…" and the like, for the palettes' menus: the label the
/// module tree gives the command.
fn new_label(kind: BlueprintKind) -> String {
    crate::tree::new_label(&crate::commands::Id::new_blueprint(kind).name())
}

/// What the palette tree is drawn with.
struct Tree<'a> {
    game: &'a mg_rules::GameData,
    kind: BlueprintKind,
    custom: bool,
    /// The grouping shown (its id, and whether by category within),
    /// none by Category; its groups' names, by their places; and each
    /// category's place in the palette it came from.
    grouping: Option<(String, bool)>,
    groups: Vec<String>,
    origin: Option<&'a HashMap<Vec<usize>, Vec<usize>>>,
    /// The custom palette's blueprints the module has (the palette also
    /// lists its haks', the override folder's and blueprints there are no
    /// more, as Aurora's does: those are viewed, as the game's are).
    own: Arc<std::collections::HashSet<ResRef>>,
    /// Every category opened or closed this frame.
    fold: Option<bool>,
    /// The categories left open (`Settings::palette_open`), and whether a
    /// category opened or closed changed them.
    open: &'a mut std::collections::HashSet<String>,
    open_changed: bool,
    /// What a right click offers to make a new blueprint of the kind
    /// ("New Creature…"), where a module is open.
    new: Option<&'a str>,
    /// The blueprints the search finds (`None`: no search).
    found: Option<&'a std::collections::HashSet<ResRef>>,
    /// Those the search finds since the module changed: their categories
    /// open this frame (`PaletteView::arrived`).
    arrived: std::collections::HashSet<ResRef>,
    tags: &'a HashMap<ResRef, String>,
    /// This palette's favorites.
    favorites: std::collections::HashSet<ResRef>,
    /// The thumbnail ready for the blueprint hovered last frame.
    thumb: Option<(ResKey, Option<egui::TextureId>)>,
    /// The Gallery: blueprints as pictures. Those made so far, and those
    /// in sight (made or not).
    gallery: bool,
    side: f32,
    ready: &'a crate::model_view::Thumbnails,
    shown: Vec<ResKey>,
    /// And what there is to say of it.
    about: Option<(ResKey, &'a [String])>,
    /// The hovered blueprint's Tag, read for its hover ([`tag_of`]).
    hover_tag: Option<(ResKey, Option<&'a str>)>,
    sel: Selection,
    picks: Vec<Pick>,
    /// The blueprint hovered now.
    hovered: Option<ResKey>,
    /// In the Gallery, the cell hovered now: its blueprint, where it is,
    /// its name and challenge rating (for its enlargement beside the
    /// dock).
    enlarged: Option<Enlarged>,
    /// Options › General: ResRefs beside names, and creatures' challenge
    /// ratings.
    resrefs: bool,
    cr: bool,
    /// Options › General: categories with no blueprint in them (or under
    /// them) left out.
    hide_empty: bool,
    /// The row the arrow keys are at, and whether to bring it into view.
    cursor: Option<At>,
    reveal: bool,
    /// A branch the keys open or close in this frame.
    toggle: Option<(Branch, bool)>,
    /// The rows as drawn, where a key was pressed (`None`: not listed).
    listed: Option<Listed>,
    /// Something in the tree was clicked: the arrow keys are its own.
    clicked: bool,
    /// A blueprint Shift+clicked (with Ctrl: added to those chosen): the
    /// rows from the one selected to it are chosen, once its branch's
    /// rows are drawn.
    range: Option<(ResKey, bool)>,
    /// The plugins' commands for right-click menus.
    commands: &'a [crate::plugins::PluginCommand],
    plugin_kind: Option<mg_plugin::Target>,
    has_plugin_menu: bool,
    meta: Arc<[NodeMeta]>,
    /// The places of the blueprints the search finds ([`MetaCache::picks`]).
    found_at: Arc<[u32]>,
}

/// Where a blueprint's row is: its branch, the branch's place in the
/// keys' listing (if the rows are listed) and the blueprint the cursor is
/// at in the branch.
type Within<'a> = (&'a Branch, Option<usize>, Option<ResKey>);

/// A blueprint's row: its name, its ResRef in parentheses and a creature's
/// challenge rating, where each is asked for.
fn blueprint_label(name: &str, resref: Option<ResRef>, cr: Option<f32>) -> String {
    let mut label = name.to_string();
    if let Some(r) = resref {
        label.push_str(&format!(" ({r})"));
    }
    if let Some(cr) = cr {
        label.push_str(&format!("  (CR {cr})"));
    }
    label
}

/// A blueprint as Favorites and Recent remember it: `utp:plc_chest1`.
pub(crate) fn remembered(key: ResKey) -> String {
    format!("{}:{}", key.restype.extension().unwrap_or_default(), key.resref)
}

impl Tree<'_> {
    /// How a category at `path` is kept among those left open
    /// (`Settings::palette_open`).
    fn remembered_key(&self, path: &[usize]) -> String {
        let ext = self.kind.restype().extension().unwrap_or_default();
        let source = if self.custom { "custom" } else { "standard" };
        let dotted = |p: &[usize]| p.iter().map(usize::to_string).collect::<Vec<_>>().join(".");
        // (Grouped: by the grouping, the group's name and the category's
        // place in the palette, which stay as the groups change.)
        match (&self.grouping, path.split_first()) {
            (Some((id, _)), Some((group, rest))) => {
                let name = self.groups.get(*group).map_or("", String::as_str);
                match self.origin.and_then(|o| o.get(path)).filter(|_| !rest.is_empty()) {
                    Some(from) => format!("{ext}/{source}/@{id}/{name}/{}", dotted(from)),
                    None => format!("{ext}/{source}/@{id}/{name}"),
                }
            }
            _ => format!("{ext}/{source}/{}", dotted(path)),
        }
    }

    /// A category drawn open or closed: kept so for the next start, but
    /// for what a search opened (it closes again).
    fn remember(&mut self, key: String, open: bool) {
        if self.found.is_some() || self.open.contains(&key) == open {
            return;
        }
        if open {
            self.open.insert(key);
        } else {
            self.open.remove(&key);
        }
        self.open_changed = true;
    }

    /// Whether a blueprint the search finds since the module changed is in
    /// a node or under it.
    fn arrives_in(&self, node: &PaletteNode) -> bool {
        !self.arrived.is_empty()
            && (node.blueprints.iter().any(|b| self.arrived.contains(&b.resref))
                || node.children.iter().any(|c| self.arrives_in(c)))
    }

    /// Whether the arrow keys' cursor is in `branch`: at its own row, or
    /// at a blueprint's under it.
    fn cursor_in(&self, branch: &Branch) -> Option<Option<ResKey>> {
        self.cursor.as_ref().filter(|c| c.branch == *branch).map(|c| c.blueprint)
    }

    /// What the keys ask of a branch's header, this frame: open or closed.
    fn asked(&self, branch: &Branch) -> Option<bool> {
        self.fold.or(self.toggle.as_ref().filter(|(b, _)| b == branch).map(|(_, open)| *open))
    }

    /// A branch's header as drawn: marked where the cursor is at it,
    /// brought into view if the keys moved the cursor there, the cursor's
    /// on a click, and listed open or closed for the keys.
    fn header(
        &mut self,
        ui: &egui::Ui,
        branch: &Branch,
        listing: Option<usize>,
        header: &egui::Response,
        behind: egui::layers::ShapeIdx,
    ) {
        if self.cursor_in(branch) == Some(None) {
            // (The keys' cursor on a header: the theme's `cell-select`.)
            let fill = mg_uikit::tokens(ui.ctx()).cell_select;
            let radius = ui.visuals().widgets.inactive.corner_radius;
            ui.painter().set(behind, egui::epaint::RectShape::filled(header.rect, radius, fill));
            if self.reveal {
                header.scroll_to_me(None);
            }
        }
        if header.clicked() {
            let (kind, custom) = (self.kind, self.custom);
            let grouping = self.grouping.clone();
            let branch = branch.clone();
            self.cursor = Some(At { kind, custom, grouping, branch, blueprint: None });
            self.clicked = true;
        }
        if let (Some(at), Some(listed)) = (listing, &mut self.listed) {
            use egui::collapsing_header::CollapsingState;
            listed.branches[at].open =
                CollapsingState::load(ui.ctx(), header.id).is_some_and(|s| s.is_open());
        }
    }

    fn node(
        &mut self,
        ui: &mut egui::Ui,
        node: &PaletteNode,
        path: &[usize],
        parent: Option<usize>,
        meta_idx: &mut usize,
    ) {
        let searching = self.found.is_some();
        let node_meta = self.meta[*meta_idx];
        let end = *meta_idx + node_meta.subtree_len as usize;
        // (Empty, under Hide empty categories: left out, searched or not.)
        if (self.hide_empty && node_meta.total == 0) || (searching && node_meta.shown == 0) {
            *meta_idx = end;
            return;
        }
        *meta_idx += 1;
        let (game, kind, custom) = (self.game, self.kind, self.custom);
        let branch = Branch::Category(path.to_vec());
        let top = ui.cursor().top();
        let listing = self.listed.as_mut().map(|l| l.branch(branch.clone(), parent, top));
        let within = self.cursor_in(&branch).flatten();
        let behind = ui.painter().add(egui::Shape::Noop);
        // (A group counts every blueprint in it, in its categories too.)
        let count = if self.grouping.is_some() && path.len() == 1 {
            node_meta.total as usize
        } else {
            node.blueprints.len()
        };
        let header_rect = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(ui.available_width().max(1.0), ui.spacing().interact_size.y),
        );
        let header_visible = ui.is_rect_visible(header_rect);
        let font = egui::TextStyle::Body.resolve(ui.style());
        // (Its count at the row's end, as the Main board draws it; named
        // with it for a screen reader, "Battlefield (31)", as before.)
        let count_text = (header_visible && count > 0).then(|| count.to_string());
        let title = if header_visible {
            let name = shown_name(&node.name, game);
            let count_width = count_text.as_ref().map_or(0.0, |c| {
                let galley = ui.fonts_mut(|f| {
                    f.layout_no_wrap(c.clone(), font.clone(), egui::Color32::PLACEHOLDER)
                });
                galley.size().x + 12.0
            });
            // (Cut short with an ellipsis where the panel is too narrow for it:
            // a header's words never wrap, and would widen the dock.)
            fitted_beside(ui, name, count_width)
        } else {
            egui::WidgetText::from("")
        };
        // (Finding opens every category with a match as it is typed,
        // through `fold`: they can be closed again after.)
        // (Open as it was left at the last start, until it is opened or
        // closed: `Tree::remember`.)
        // (While a search is typed, the categories are open or closed on
        // their own, apart: the search cleared, each is again as it was
        // before it.)
        let remembered = (!searching).then(|| self.remembered_key(path));
        let default_open = searching || remembered.as_ref().is_some_and(|k| self.open.contains(k));
        let shown = egui::CollapsingHeader::new(title)
            .icon(mg_uikit::collapsing_icon)
            .id_salt(("palette", kind, custom, path, searching, &self.grouping))
            .default_open(default_open)
            .open(self.asked(&branch).or_else(|| self.arrives_in(node).then_some(true)))
            .show(ui, |ui| {
                for (i, child) in node.children.iter().enumerate() {
                    let mut p = path.to_vec();
                    p.push(i);
                    self.node(ui, child, &p, listing, meta_idx);
                }
                if searching {
                    let from = node_meta.picks_from as usize;
                    let picks = self.found_at.get(from..from + node_meta.picks as usize);
                    let rows: Vec<&PaletteBlueprint> = (picks.unwrap_or_default().iter())
                        .filter_map(|&i| node.blueprints.get(i as usize))
                        .collect();
                    self.rows(ui, NodeRows::Refs(&rows), (&branch, listing, within));
                } else {
                    self.rows(ui, NodeRows::Slice(&node.blueprints), (&branch, listing, within));
                }
            });
        // Advance unconditionally past this subtree even when collapsed so sibling indices stay aligned.
        *meta_idx = end;
        let row = shown.header_response.rect;
        if let Some(c) = count_text {
            let t = mg_uikit::tokens(ui.ctx());
            let at = egui::pos2(ui.max_rect().right() - 4.0, row.center().y);
            ui.painter().text(at, egui::Align2::RIGHT_CENTER, c, font, t.text_3);
        }
        shown.header_response.widget_info(|| {
            let name = shown_name(&node.name, game);
            let said = if count > 0 { format!("{name} ({count})") } else { name };
            egui::WidgetInfo::labeled(egui::WidgetType::CollapsingHeader, true, &said)
        });
        self.header(ui, &branch, listing, &shown.header_response, behind);
        if let Some(remembered) = remembered {
            self.remember(remembered, shown.openness > 0.5);
        }
        // The plugins' commands for its kind, run on every blueprint of
        // the category (and those under it).
        let plugins = self.has_plugin_menu;
        let kinds = self.plugin_kind.as_slice();
        if !custom {
            if plugins {
                mg_uikit::context_menu(&shown.header_response, |ui| {
                    if let Some(i) = crate::plugins::menu_commands(ui, self.commands, kinds) {
                        let mut keys = Vec::new();
                        all_under(node, kind, &mut keys);
                        self.picks.push(Pick::Plugin(i, keys));
                    }
                });
            }
            return;
        }
        // A custom blueprint dropped on a category moves into it.
        if let Some(id) = node.id
            && let Some(d) = shown.header_response.dnd_release_payload::<Dragged>()
            && d.0.restype == kind.restype()
        {
            self.picks.push(Pick::MoveTo(d.0, id));
        }
        // A custom category: Update Instances of everything in it.
        let new = self.new;
        let mut made = false;
        mg_uikit::context_menu(&shown.header_response, |ui| {
            if let Some(label) = new {
                if ui.button(label).clicked() {
                    made = true;
                    ui.close();
                }
                mg_uikit::menu_separator(ui);
            }
            let mut keys = Vec::new();
            all_under(node, kind, &mut keys);
            if ui
                .add_enabled(!keys.is_empty(), egui::Button::new("Update Instances"))
                .on_hover_text("Every blueprint in this category")
                .clicked()
            {
                self.picks.push(Pick::UpdateInstances(keys.clone()));
                ui.close();
            }
            if plugins && let Some(i) = crate::plugins::menu_commands(ui, self.commands, kinds) {
                self.picks.push(Pick::Plugin(i, keys));
            }
        });
        if made {
            self.picks.push(Pick::New);
        }
    }

    /// A blueprint's row in a branch: with the branch, its place in the
    /// keys' listing and the blueprint the cursor is at in it.
    fn row(
        &mut self,
        ui: &mut egui::Ui,
        b: &PaletteBlueprint,
        within: Within<'_>,
        cell: Option<Cell>,
    ) {
        let (branch, listing, cursor) = within;
        let (kind, custom) = (self.kind, self.custom);
        let key = ResKey::new(b.resref, kind.restype());
        // (One the module has: a hak's, the override folder's or one there
        // is no more is in the custom palette too, as in Aurora's.)
        let mine = custom && self.own.contains(&b.resref);
        if let (Some(at), Some(listed)) = (listing, &mut self.listed) {
            let top = cell.as_ref().map_or(ui.cursor().top(), |c| c.at.top());
            listed.tops.insert(listed.rows.len(), top);
            listed.rows.push((at, Some(key)));
        }
        // (The row the keys moved the cursor to is brought into view.)
        let reveal = self.reveal && cursor == Some(key);
        // In the Gallery, a cell where the grid's arithmetic puts it: out of
        // sight, its place alone (for the keys), and its picture asked for
        // in the row ahead.
        if let Some(cell) = &cell {
            if let (Some(listed), Some(_)) = (&mut self.listed, listing) {
                listed.places.insert(listed.rows.len() - 1, cell.at);
            }
            if reveal {
                ui.scroll_to_rect(cell.at, None);
            }
            if cell.ahead {
                self.shown.push(key);
            }
            if !cell.drawn {
                return;
            }
        }
        // Rows out of sight take their room only (a palette may list
        // thousands).
        let height = ui.spacing().interact_size.y;
        let room = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(1.0, height));
        if cell.is_none() && !reveal && !ui.is_rect_visible(room) {
            ui.allocate_space(egui::vec2(1.0, height));
            return;
        }
        let name = shown_name(&b.name, self.game);
        let favorite = self.favorites.contains(&b.resref);
        let label =
            blueprint_label(&name, self.resrefs.then_some(b.resref), b.cr.filter(|_| self.cr));
        // Dragged into an area view, it is placed where it is dropped; onto
        // a custom category, it moves there. (A favorite marked with a
        // bookmark: before its name in the list, at a picture's top right.)
        let r = if let Some(cell) = &cell {
            let r = self.cell(ui, b, cell, &name, &label);
            if r.hovered() && !egui::DragAndDrop::has_any_payload(ui.ctx()) {
                let (at, name, cr) = (cell.at, name.clone(), b.cr);
                self.enlarged = Some(Enlarged { key, at, name, cr });
            }
            r
        } else {
            let on = self.sel.has(key);
            let sense = egui::Sense::click_and_drag();
            match favorite {
                true => {
                    // (Its own sense, a click and a drag, as the other rows'.)
                    let row = mg_uikit::IconButton::new(crate::icons::FAVORITE, label)
                        .selectable(on)
                        .sense(sense);
                    ui.add(row)
                }
                false => ui.add(egui::Button::selectable(on, label).sense(sense)),
            }
        };
        if r.hovered() {
            self.hovered = Some(key);
        }
        if reveal && cell.is_none() {
            r.scroll_to_me(None);
        }
        if r.clicked() {
            // The arrow keys go on from here.
            let (grouping, branch) = (self.grouping.clone(), branch.clone());
            self.cursor = Some(At { kind, custom, grouping, branch, blueprint: Some(key) });
            self.clicked = true;
        }
        let thumb = self.thumb.filter(|(k, _)| *k == key).and_then(|(_, t)| t);
        let about = self.about.filter(|(k, _)| *k == key).map_or(&[][..], |(_, lines)| lines);
        let tags = self.tags;
        let read = self.hover_tag.filter(|(k, _)| *k == key).and_then(|(_, t)| t);
        let r = if cell.is_some() {
            r
        } else {
            r.on_hover_ui(|ui| {
                ui.strong(&name);
                ui.label(format!("ResRef {}", b.resref));
                let tag = tags.get(&b.resref).map(String::as_str).or(read);
                if let Some(t) = tag.filter(|t| !t.is_empty()) {
                    ui.label(format!("Tag {t}"));
                }
                if let Some(cr) = b.cr {
                    ui.label(format!("Challenge rating {cr}"));
                }
                if let Some(id) = thumb {
                    let size = 180.0;
                    ui.image(egui::load::SizedTexture::new(id, egui::vec2(size, size)));
                }
                for line in about {
                    ui.label(line);
                }
            })
        };
        if r.clicked() {
            // Ctrl+click chooses several: custom blueprints for bulk
            // edits, placeables (custom or the game's) to paint; Shift+click
            // the rows from the one selected to this one.
            let several = self.several(b);
            let (command, shift) = ui.input(|i| (i.modifiers.command, i.modifiers.shift));
            let sel = &mut self.sel;
            if several && shift && sel.selected.is_some_and(|s| s != key) {
                self.range = Some((key, command));
            } else if several && command {
                if let Some(at) = sel.chosen.iter().position(|k| *k == key) {
                    sel.chosen.remove(at);
                } else if sel.selected != Some(key) {
                    sel.chosen.push(key);
                }
            } else {
                sel.selected = Some(key);
                sel.chosen.clear();
            }
        }
        if r.drag_started() {
            r.dnd_set_drag_payload(Dragged(key));
        }
        if r.double_clicked() {
            self.picks.push(if mine { Pick::Edit(key) } else { Pick::View(key) });
        }
        let (sel, picks, new) = (&self.sel, &mut self.picks, self.new);
        let commands = self.commands;
        mg_uikit::context_menu(&r, |ui| {
            if let Some(label) = new {
                if ui.button(label).on_hover_text("A new blueprint (the type's wizard)").clicked() {
                    picks.push(Pick::New);
                    ui.close();
                }
                mg_uikit::menu_separator(ui);
            }
            if mine && ui.button("Edit").clicked() {
                picks.push(Pick::Edit(key));
                ui.close();
            }
            if !mine
                && ui
                    .button("View")
                    .on_hover_text("Its properties, to look at: the game's blueprint isn't changed")
                    .clicked()
            {
                picks.push(Pick::View(key));
                ui.close();
            }
            let keys = sel.bulk(key);
            if mine
                && keys.len() > 1
                && ui
                    .button(format!("Edit {} Together", keys.len()))
                    .on_hover_text("One editor: what you change is set on each")
                    .clicked()
            {
                picks.push(Pick::EditTogether(keys.clone()));
                ui.close();
            }
            if mine {
                let label = if keys.len() > 1 {
                    format!("Export {}…", keys.len())
                } else {
                    "Export…".into()
                };
                if ui
                    .button(label)
                    .on_hover_text("Write it out as a file or an ERF, as in Aurora (File › Export)")
                    .clicked()
                {
                    let all = if keys.len() > 1 { keys } else { vec![key] };
                    picks.push(Pick::Export(all));
                    ui.close();
                }
            }
            if ui
                .button("Edit Copy…")
                .on_hover_text(
                    "Copy into the module's custom palette, under a ResRef and Tag you give",
                )
                .clicked()
            {
                picks.push(Pick::EditCopy(key));
                ui.close();
            }
            if crate::model_view::previewable(key.restype) && ui.button("Preview").clicked() {
                picks.push(Pick::Preview(key));
                ui.close();
            }
            // The two ways of replacing the area's selection, together.
            mg_uikit::menu_button(ui, "Replace Selected", |ui| {
                if ui
                    .button("Replace Selected with This")
                    .on_hover_text(
                        "Make the selected objects of this type ones of this blueprint, where they \
                         stand",
                    )
                    .clicked()
                {
                    picks.push(Pick::Replace(key, false));
                    ui.close();
                }
                if ui
                    .button("Replace Selected, Keeping Tags and Scripts")
                    .on_hover_text(
                        "As Replace Selected with This, keeping each one's tag, name, scripts and \
                         variables",
                    )
                    .clicked()
                {
                    picks.push(Pick::Replace(key, true));
                    ui.close();
                }
            });
            let text = if favorite { "Remove from Favorites" } else { "Add to Favorites" };
            if ui.button(text).clicked() {
                picks.push(Pick::Favorite(key, !favorite));
                ui.close();
            }
            if ui
                .button("Find References")
                .on_hover_text("Where the module places or names this blueprint")
                .clicked()
            {
                picks.push(Pick::References(key));
                ui.close();
            }
            // (For a script, without opening the blueprint: Troubletcat's
            // ask after 3.1.0, the game's blueprints' fields copied from
            // View.) Its names to paste, together.
            mg_uikit::menu_button(ui, "Copy", |ui| {
                if ui.button("Copy ResRef").on_hover_text("Its ResRef, to paste").clicked() {
                    ui.ctx().copy_text(key.resref.to_string());
                    ui.close();
                }
                if ui.button("Copy Tag").on_hover_text("Its Tag, to paste").clicked() {
                    picks.push(Pick::CopyTag(key));
                    ui.close();
                }
            });
            if mine {
                let keys = sel.bulk(key);
                let text = match keys.len() {
                    1 => "Update Instances".to_string(),
                    n => format!("Update Instances of {n}"),
                };
                let tip = if keys.len() == 1 {
                    "Remake the objects placed from it. Ctrl+click chooses several blueprints."
                } else {
                    "Remake the objects placed from them"
                };
                if ui.button(text).on_hover_text(tip).clicked() {
                    picks.push(Pick::UpdateInstances(keys));
                    ui.close();
                }
            }
            if mine && ui.button("Delete").clicked() {
                picks.push(Pick::Delete(key));
                ui.close();
            }
            // An entry of the custom palette whose blueprint isn't the
            // module's (a hak's, the override folder's, one there is no
            // more): its entry alone taken out, the blueprint left.
            if custom
                && !mine
                && ui
                    .button("Remove from Palette")
                    .on_hover_text(
                        "Take its entry out of the module's palette. The blueprint stays where it \
                         is.",
                    )
                    .clicked()
            {
                picks.push(Pick::RemoveEntry(key));
                ui.close();
            }
            // The plugins' commands for the kind, run on the blueprints
            // chosen (a menu with no Properties to end it: last).
            if let Some(object) = mg_area::ObjectKind::from_restype(key.restype)
                && let Some(i) = crate::plugins::menu_commands(
                    ui,
                    commands,
                    &[mg_plugin::Target::Blueprint(object)],
                )
            {
                picks.push(Pick::Plugin(i, sel.bulk(key)));
            }
        });
    }

    /// Favorites or Recent: the remembered blueprints this palette has, in
    /// order.
    fn remembered(
        &mut self,
        ui: &mut egui::Ui,
        title: &'static str,
        list: &[String],
        palette: &Arc<Palette>,
    ) {
        // (Found once for the palette and the list, and kept: every
        // blueprint of the palette was listed twice a frame and each of
        // the list's looked for among them by its text.)
        let id = egui::Id::new(("palette-remembered-kept", title, self.kind, self.custom));
        let kind = self.kind;
        let kept = crate::blueprint::kept_for(ui.ctx(), id, palette, &list.to_vec(), || {
            Arc::new(remembered_in(palette, kind, list))
        });
        if kept.is_empty() {
            return;
        }
        let branch = Branch::Remembered(title);
        let top = ui.cursor().top();
        let listing = self.listed.as_mut().map(|l| l.branch(branch.clone(), None, top));
        let within = self.cursor_in(&branch).flatten();
        let behind = ui.painter().add(egui::Shape::Noop);
        let shown = egui::CollapsingHeader::new(format!("{title} ({})", kept.len()))
            .icon(mg_uikit::collapsing_icon)
            .id_salt(("palette-remembered", title, self.kind, self.custom))
            .default_open(true)
            .open(self.asked(&branch))
            .show(ui, |ui| self.rows(ui, NodeRows::Slice(&kept), (&branch, listing, within)));
        self.header(ui, &branch, listing, &shown.header_response, behind);
    }

    /// A category's blueprints: a list, or in the Gallery a grid.
    /// Whether `b` can be chosen with others: a custom blueprint the
    /// module has (for bulk edits), or a placeable (to paint).
    fn several(&self, b: &PaletteBlueprint) -> bool {
        (self.custom && self.own.contains(&b.resref)) || self.kind == BlueprintKind::Placeable
    }

    /// A Shift+click on one of `rows`: the rows from the one selected to
    /// it are chosen (those that can be), in place of those chosen before
    /// or, with Ctrl, as well. The one selected stays where a range
    /// starts; where it is in another branch, the row clicked is selected
    /// alone.
    fn choose_range(&mut self, rows: NodeRows<'_>) {
        let Some((to, adding)) = self.range else { return };
        let restype = self.kind.restype();
        let at = |key: ResKey| rows.iter().position(|b| ResKey::new(b.resref, restype) == key);
        let Some(end) = at(to) else { return };
        self.range = None;
        let Some(start) = self.sel.selected.and_then(at) else {
            self.sel.selected = Some(to);
            self.sel.chosen.clear();
            return;
        };
        let lo = start.min(end);
        let hi = start.max(end);
        let keys: Vec<ResKey> = rows
            .iter()
            .skip(lo)
            .take(hi - lo + 1)
            .filter(|b| self.several(b))
            .map(|b| ResKey::new(b.resref, restype))
            .filter(|k| Some(*k) != self.sel.selected)
            .collect();
        if !adding {
            self.sel.chosen.clear();
        }
        for k in keys {
            if !self.sel.chosen.contains(&k) {
                self.sel.chosen.push(k);
            }
        }
    }

    fn rows(&mut self, ui: &mut egui::Ui, rows: NodeRows<'_>, within: Within<'_>) {
        if self.gallery {
            self.grid(ui, rows, within);
        } else if self.listed.is_some() || self.reveal || self.range.is_some() {
            // (Every row gone through: the keys need every place.)
            for b in rows.iter() {
                self.row(ui, b, within, None);
            }
        } else {
            // The rows out of sight take their room in one go, as each
            // would take it alone (a search may find thousands: one by
            // one, 8,000 found took a millisecond a frame).
            let height = ui.spacing().interact_size.y;
            let gap = ui.spacing().item_spacing.y;
            let (step, n) = (height + gap, rows.len());
            let (clip, top) = (ui.clip_rect(), ui.cursor().top());
            let first = (((clip.top() - top) / step).floor().max(0.0) as usize).min(n);
            let last = ((((clip.bottom() - top) / step).ceil().max(0.0) as usize) + 1).min(n);
            if first > 0 {
                ui.allocate_space(egui::vec2(1.0, first as f32 * step - gap));
            }
            for b in rows.iter().skip(first).take(last.saturating_sub(first)) {
                self.row(ui, b, within, None);
            }
            if last < n {
                ui.allocate_space(egui::vec2(1.0, (n - last) as f32 * step - gap));
            }
        }
        self.choose_range(rows);
    }

    /// A category's blueprints in the Gallery: the kit's grid, its cells
    /// grown from the slider's side to share the palette's width, laid out
    /// by arithmetic. Only the cells in sight are drawn, and their pictures
    /// and a row's ahead asked for; the rest are gone through only where
    /// the keys need every place (a key pressed, the cursor brought into
    /// sight, a Shift+click's range).
    fn grid(&mut self, ui: &mut egui::Ui, rows: NodeRows<'_>, within: Within<'_>) {
        use mg_uikit::gallery::{GridLayout, label_height};
        let seen = ui.clip_rect().right() - ui.cursor().left();
        let room = ui.available_width().min(seen).max(1.0);
        let layout = GridLayout::grow(room, self.side, label_height(false, 2, self.detail()));
        let size = egui::vec2(room, layout.height(rows.len()));
        let (rect, grid) = ui.allocate_exact_size(size, egui::Sense::hover());
        // (A list of all of the category's pictures to a screen reader:
        // those out of sight are not built, the keys reach them.)
        grid.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Pictures"));
        mg_uikit::gallery::said_as_list(ui.ctx(), grid.id, rows.len());
        let clip = ui.clip_rect();
        let in_sight = layout.rows_in(rect.top(), clip.y_range(), rows.len());
        let drawn = layout.cells_of(in_sight.clone(), rows.len());
        let ahead = in_sight.start..(in_sight.end + 1).min(layout.rows(rows.len()));
        let ahead = layout.cells_of(ahead, rows.len());
        let every = self.listed.is_some() || self.reveal || self.range.is_some();
        let (cursor, tile) = (within.2, layout.tile);
        for (i, b) in rows.iter().enumerate() {
            let key = ResKey::new(b.resref, self.kind.restype());
            if !every && !ahead.contains(&i) && cursor != Some(key) {
                continue;
            }
            let at = layout.rect(rect.min, i);
            let (drawn, ahead) = (drawn.contains(&i), ahead.contains(&i));
            let cell = Cell { at, tile, drawn, ahead, index: i, of: rows.len() };
            self.row(ui, b, within, Some(cell));
        }
        // (The keys' cell, built, told to a screen reader as the list's
        // active one: `cell`'s id.)
        let kind = self.kind.restype();
        let on = rows.iter().position(|b| cursor == Some(ResKey::new(b.resref, kind)));
        if let Some(i) = on.filter(|i| drawn.contains(i))
            && let Some(b) = rows.get(i)
        {
            let key = ResKey::new(b.resref, kind);
            let cell = ui.id().with(("palette-cell", key)).accesskit_id();
            ui.ctx().accesskit_node_builder(grid.id, |n| n.set_active_descendant(cell));
        }
    }

    /// Whether the Gallery's cells have a detail line: a creature's
    /// challenge rating, or ResRefs (Options › General).
    fn detail(&self) -> bool {
        self.resrefs || (self.cr && self.kind == BlueprintKind::Creature)
    }

    /// A blueprint in the Gallery: the kit's cell, its picture over its
    /// name (two lines, then an ellipsis), its ResRef or challenge rating
    /// on the detail line, the bookmark of a favorite, the "No model" badge
    /// of one whose appearance has no model. Its response, said by its
    /// whole label (`label`).
    fn cell(
        &mut self,
        ui: &mut egui::Ui,
        b: &PaletteBlueprint,
        cell: &Cell,
        name: &str,
        label: &str,
    ) -> egui::Response {
        let key = ResKey::new(b.resref, self.kind.restype());
        let pictured = crate::model_view::Pictured::Resource(key);
        let made = self.ready.made(pictured);
        let id = ui.id().with(("palette-cell", key));
        let r = ui.interact(cell.at, id, egui::Sense::CLICK | egui::Sense::DRAG);
        let state = mg_uikit::gallery::CellState {
            hovered: r.hovered(),
            cursor: self.cursor.as_ref().is_some_and(|c| c.blueprint == Some(key)),
            keys: keys_are(ui.ctx(), Keys::Palette),
        };
        let cr = b.cr.filter(|_| self.cr).map(|cr| format!("CR {cr}"));
        let detail = match (self.resrefs, cr) {
            (true, Some(cr)) => Some(format!("{}  {cr}", b.resref)),
            (true, None) => Some(b.resref.to_string()),
            (false, cr) => cr,
        };
        let icon = mg_area::ObjectKind::from_restype(key.restype).map(crate::icons::object);
        let mut cell_ =
            mg_uikit::gallery::GalleryCell::new(name, crate::widgets::cell_picture(made))
                .said(label)
                .detail(detail.as_deref())
                .selected(self.sel.has(key))
                .favorite(self.favorites.contains(&b.resref))
                .no_model(self.ready.no_model(pictured));
        if let Some(icon) = icon {
            cell_ = cell_.kind(icon);
        }
        cell_.paint(ui, cell.at, cell.tile, state, &r);
        mg_uikit::gallery::said_in_list(ui.ctx(), r.id, cell.index, cell.of);
        r
    }
}

/// The width of the Gallery's enlargement: its picture's and the frame's
/// margins.
const ENLARGED: f32 = 180.0 + 16.0;

/// The Gallery's enlargement of the cell `e`: its name, ResRef, Tag and
/// challenge rating, its picture 180 points large (once made) and what
/// there is to say of it, as a tooltip beside `dock` (the Right Dock, or
/// the palette's own window where it floats) at the cell's top
/// ([`mg_uikit::gallery::show_beside`]).
fn enlargement(app: &Moonglow, ui: &egui::Ui, e: &Enlarged, dock: egui::Rect) {
    let view = &app.palette;
    let thumb = view.thumb.filter(|(k, _)| *k == e.key).and_then(|(_, t)| t);
    let about = view.about.as_ref().filter(|(k, _)| *k == e.key).map_or(&[][..], |(_, l)| l);
    let tag = (view.hover_tag.as_ref())
        .filter(|(k, _)| *k == e.key)
        .and_then(|(_, t)| t.as_deref())
        .filter(|t| !t.is_empty());
    let no_model = app.thumbnails.blueprint_has_no_model(e.key);
    let id = egui::Id::new("palette-enlargement");
    // (Measured anew for another cell, or as its picture, Tag or lines
    // come.)
    let what = (e.key, thumb.is_some(), tag, about.len(), no_model);
    mg_uikit::gallery::show_beside(ui.ctx(), id, e.at, dock, ENLARGED, what, |ui| {
        ui.strong(&e.name);
        ui.label(format!("ResRef {}", e.key.resref));
        if let Some(t) = tag {
            ui.label(format!("Tag {t}"));
        }
        if let Some(cr) = e.cr {
            ui.label(format!("Challenge rating {cr}"));
        }
        if let Some(id) = thumb {
            ui.image(egui::load::SizedTexture::new(id, egui::vec2(180.0, 180.0)));
        }
        if no_model {
            ui.label("Its appearance has no model to draw: its Properties say why (Verify too)");
        }
        for line in about {
            ui.label(line);
        }
    });
}

/// The Gallery's cell under the pointer: its blueprint, where it is, its
/// name and challenge rating.
struct Enlarged {
    key: ResKey,
    at: egui::Rect,
    name: String,
    cr: Option<f32>,
}

/// Where a blueprint's cell is in the Gallery's grid of its category: its
/// rectangle and its tile's size, and whether it is drawn (in sight) and
/// its picture asked for (in sight, or in the row ahead).
struct Cell {
    at: egui::Rect,
    tile: egui::Vec2,
    drawn: bool,
    ahead: bool,
    /// Its place among its category's pictures, and how many there are.
    index: usize,
    of: usize,
}

/// The blueprints of `list` (Favorites or Recent: `ext:resref` each) that
/// `palette` has, of `kind`, in the list's order.
fn remembered_in(palette: &Palette, kind: BlueprintKind, list: &[String]) -> Vec<PaletteBlueprint> {
    let ext = kind.restype().extension().unwrap_or_default();
    let wanted: Vec<ResRef> = list
        .iter()
        .filter_map(|r| r.strip_prefix(ext)?.strip_prefix(':'))
        .filter_map(|r| ResRef::from_str(r).ok())
        .collect();
    if wanted.is_empty() {
        return Vec::new();
    }
    let all: HashMap<ResRef, &PaletteBlueprint> =
        palette.blueprints().into_iter().rev().map(|(_, b)| (b.resref, b)).collect();
    wanted.iter().filter_map(|r| all.get(r).map(|b| (*b).clone())).collect()
}

/// What a palette's hover says of a blueprint that has no model to show:
/// a sound's sounds, a trigger's kind, an encounter's creatures, a store's
/// prices, a waypoint's map note. Nothing for the kinds that have a
/// picture.
fn about(app: &Moonglow, key: ResKey) -> Vec<String> {
    use mg_core::ResType as T;
    if !matches!(key.restype, T::UTS | T::UTT | T::UTE | T::UTM | T::UTW) {
        return Vec::new();
    }
    let Some(gff) = crate::area_tools::blueprint(app, key) else { return Vec::new() };
    let s = &gff.root;
    let int = |label: &str| s.integer(label).unwrap_or(0);
    let on = |label: &str| int(label) != 0;
    // Up to four names, and how many more.
    let listed = |names: Vec<String>| crate::text::some_of(&names, 5);
    let resrefs = |list: &str, label: &str| -> Vec<String> {
        let items = s.list(list).unwrap_or(&[]);
        items.iter().filter_map(|i| i.resref(label)).map(|r| r.to_string()).collect()
    };
    let mut out = Vec::new();
    match key.restype {
        T::UTS => {
            let sounds = resrefs("Sounds", "Sound");
            if !sounds.is_empty() {
                out.push(format!("Plays {}", listed(sounds)));
            }
            let place =
                if on("Positional") { "from where it stands" } else { "everywhere in the area" };
            let how = if on("Continuous") { "without a pause" } else { "at intervals" };
            out.push(format!("Heard {place}, {how}, volume {}", int("Volume")));
        }
        T::UTT => out.push(match int("Type") {
            1 => "An area transition".into(),
            2 => format!("A trap (type {})", int("TrapType")),
            _ => "A generic trigger".into(),
        }),
        T::UTE => {
            let creatures = resrefs("CreatureList", "ResRef");
            if !creatures.is_empty() {
                out.push(format!("Spawns {}", listed(creatures)));
            }
            out.push(format!(
                "{} to {} creatures; {}",
                int("RecCreatures"),
                int("MaxCreatures"),
                if on("SpawnOption") { "spawns once" } else { "spawns again" }
            ));
        }
        T::UTM => {
            out.push(format!("Sells at {}%, buys at {}%", int("MarkUp"), int("MarkDown")));
            let items: usize = s
                .list("StoreList")
                .unwrap_or(&[])
                .iter()
                .map(|page| page.list("ItemList").map_or(0, <[_]>::len))
                .sum();
            out.push(format!("{items} items for sale"));
        }
        T::UTW => {
            let note = s.locstring("MapNote").and_then(|l| app.game.as_deref()?.locstring(l));
            match note.filter(|n| on("HasMapNote") && !n.is_empty()) {
                Some(n) => out.push(format!("Map note: {n}")),
                None => out.push("No map note".into()),
            }
        }
        _ => {}
    }
    out
}

/// The palette's Prefabs: the groups of objects saved, each placed with a
/// click, and how to save one.
fn prefabs_ui(app: &mut Moonglow, ui: &mut egui::Ui) {
    let area = app.palette.area.filter(|a| app.area_views.contains_key(a));
    let chosen = area.and_then(|a| app.area_views.get(&a)).map_or(0, |v| v.selection.len());
    ui.label(
        "A prefab is a group of placed objects kept under a name: a camp, a market stall, a \
         furnished room. It is placed as one, in any area of any module.",
    );
    let save = ui
        .add_enabled(chosen > 0, egui::Button::new("Save Selection as Prefab…"))
        .on_hover_text("The objects selected in the area shown, kept as a prefab")
        .on_disabled_hover_text("Select objects in an area first");
    if save.clicked()
        && let Some(area) = area
    {
        crate::area_view::save_selection_as_prefab(app, area);
    }
    ui.separator();
    let dir = app.prefab_dir.clone();
    // The prefab deleted last can be put back (its file is Moonglow's, not
    // the module's: Edit › Undo doesn't know it).
    if let Some((name, text)) = app.prefab_deleted.clone() {
        let mut keep = true;
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Prefab '{name}' deleted."));
            if ui.button("Undo Delete").on_hover_text("Put the prefab back").clicked() {
                match crate::prefabs::restore(dir.as_deref(), &name, &text) {
                    Ok(()) => app.log.info(format!("Prefab '{name}' put back")),
                    Err(e) => app.log.error(format!("Undo Delete: {e}")),
                }
                keep = false;
            }
            let dismiss = crate::widgets::clear_button().name("Dismiss");
            let tip = "Dismiss this note: the prefab stays deleted";
            if ui.add(dismiss).on_hover_text(tip).clicked() {
                keep = false;
            }
        });
        if !keep {
            app.prefab_deleted = None;
        }
        ui.separator();
    }
    let names = crate::prefabs::list(dir.as_deref());
    if names.is_empty() {
        ui.weak("No prefabs yet. Select objects in an area, then Save Selection as Prefab…");
        return;
    }
    // The prefab Rename… was chosen for, and the name typed so far.
    let renamed_id = egui::Id::new("palette-prefab-rename");
    let mut renamed: Option<(String, String)> = ui.data(|d| d.get_temp(renamed_id));
    // (Renamed this frame: the list read again, so the old name isn't
    // listed, nor its missing file read for its hover.)
    let mut renamed_now = false;
    if let Some((name, mut to)) = renamed.clone().filter(|(n, _)| names.contains(n)) {
        let mut done = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Rename '{name}' to"));
            let r = ui.add(mg_uikit::text_field(&mut to).desired_width(160.0));
            crate::widgets::autofocus(ui, &r);
            let taken = names.iter().any(|n| n != &name && n.eq_ignore_ascii_case(to.trim()));
            let ok = crate::prefabs::valid_name(&to) && !taken;
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.add_enabled(ok, egui::Button::new("Rename")).clicked() || (ok && enter))
                && to.trim() != name
            {
                match crate::prefabs::rename(dir.as_deref(), &name, &to) {
                    Ok(()) => app.log.info(format!("Prefab '{name}' renamed to '{}'", to.trim())),
                    Err(e) => app.log.error(format!("Rename prefab: {e}")),
                }
                renamed_now = true;
                done = true;
            } else if ui.button("Cancel").clicked()
                || ui.input(|i| i.key_pressed(egui::Key::Escape))
                || (ok && enter)
            {
                done = true;
            }
            if taken {
                ui.colored_label(ui.visuals().warn_fg_color, "Another prefab has that name");
            } else if !ok && !to.is_empty() {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "Letters, digits, spaces, - and _ only",
                );
            }
        });
        renamed = (!done).then_some((name, to));
        ui.separator();
    } else {
        renamed = None;
    }
    let names = if renamed_now { crate::prefabs::list(dir.as_deref()) } else { names };
    // (The arrow keys' cursor, moved on the rows as they were drawn last.)
    let mut list = std::mem::take(&mut app.palette.prefab_list);
    list.keys(ui);
    let scrolled = egui::ScrollArea::vertical()
        .id_salt("palette-prefabs")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            mg_uikit::list_rows(ui);
            for name in names {
                let slot = list.slot(name.clone(), 0);
                let being = renamed.as_ref().is_some_and(|(n, _)| n == &name);
                let r = ui.selectable_label(being, &name).on_hover_ui(|ui| {
                    // What it holds, read while the pointer rests on it.
                    match crate::prefabs::summary(dir.as_deref(), &name) {
                        Ok(holds) => ui.label(holds),
                        Err(e) => ui.colored_label(ui.visuals().error_fg_color, e),
                    };
                    ui.weak("Click, then click in the area to place it. Right-click for more.");
                });
                list.seen(ui, slot, r.rect, None, false);
                if r.clicked() {
                    list.click(ui.ctx(), slot);
                }
                // (Enter on the row the keys' cursor is at is a click.)
                if r.clicked() || list.asked(slot).0 {
                    app.actions.push(Action::PlacePrefab(name.clone()));
                }
                mg_uikit::context_menu(&r, |ui| {
                    if ui.button("Place").clicked() {
                        app.actions.push(Action::PlacePrefab(name.clone()));
                        ui.close();
                    }
                    if ui.button("Rename…").clicked() {
                        renamed = Some((name.clone(), name.clone()));
                        ui.close();
                    }
                    if ui
                        .button("Delete")
                        .on_hover_text("Undo Delete, above the list, puts it back")
                        .clicked()
                    {
                        match crate::prefabs::delete(dir.as_deref(), &name) {
                            Ok(text) => {
                                app.log.info(format!("Prefab '{name}' deleted"));
                                app.prefab_deleted = Some((name.clone(), text));
                            }
                            Err(e) => app.log.error(format!("Delete prefab: {e}")),
                        }
                        ui.close();
                    }
                });
            }
        });
    list.end(scrolled.inner_rect.height());
    app.palette.prefab_list = list;
    keys_outline(ui, scrolled.inner_rect, Keys::Palette);
    ui.data_mut(|d| match renamed {
        Some(v) => {
            d.insert_temp(renamed_id, v);
        }
        None => d.remove::<(String, String)>(renamed_id),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A module of waypoint blueprints `names` (each tagged for its name,
    /// in category 1), its skeleton of two categories, and no custom
    /// palette; and the game data of no resources.
    fn waypoints(names: &[&str]) -> (mg_module::Module, mg_rules::GameData) {
        use mg_gff::{Struct, Value};
        let mut m = mg_module::Module::new();
        let mut skeleton = Gff::new(*b"ITP ");
        let node = |id: u8, name: &str| {
            let mut s = Struct::new(0);
            s.set("ID", Value::Byte(id));
            s.set("NAME", Value::String(name.as_bytes().to_vec()));
            s
        };
        skeleton.root.set("MAIN", Value::List(vec![node(0, "Plain"), node(1, "Posts")]));
        m.set_gff(BlueprintKind::Waypoint.skeleton_key(), &skeleton).unwrap();
        for name in names {
            set_waypoint(&mut m, name, name);
        }
        let game = mg_rules::GameData::new(
            mg_resman::ResMan::new(),
            mg_tlk::Tlk::new(mg_core::Language::ENGLISH),
        );
        (m, game)
    }

    fn set_waypoint(m: &mut mg_module::Module, resref: &str, name: &str) {
        let r = ResRef::from_str(resref).unwrap();
        let g = mg_module::blueprints::waypoint(r, &name.to_uppercase(), 1, 1);
        m.set_gff(ResKey::new(r, mg_core::ResType::UTW), &g).unwrap();
    }

    /// A blueprint's tag is read again only when its bytes are others
    /// (the review's B24: every blueprint was read and parsed for its tag
    /// at every change of the module).
    #[test]
    fn a_palette_s_tags_are_read_again_for_the_blueprints_that_changed() {
        let kind = BlueprintKind::Waypoint;
        let (mut m, game) = waypoints(&["gate", "inn", "well"]);
        let palette = |m: &mg_module::Module| {
            Palette::read(&mg_module::palette::rebuild_custom_palette(m, &game, kind).unwrap())
        };
        let mut kept = HashMap::new();
        let (tags, read) = tags_read(&palette(&m), kind, Some(&m), &game, &mut kept);
        assert_eq!(read, 3);
        let tag = |tags: &HashMap<ResRef, String>, r: &str| {
            tags.get(&ResRef::from_str(r).unwrap()).cloned()
        };
        assert_eq!(tag(&tags, "inn").as_deref(), Some("INN"));
        // Nothing changed: none read; one changed, one added: those two.
        let (again, read) = tags_read(&palette(&m), kind, Some(&m), &game, &mut kept);
        assert_eq!((again, read), (tags, 0));
        set_waypoint(&mut m, "inn", "tavern");
        set_waypoint(&mut m, "mill", "mill");
        let (tags, read) = tags_read(&palette(&m), kind, Some(&m), &game, &mut kept);
        assert_eq!(read, 2);
        assert_eq!(tag(&tags, "inn").as_deref(), Some("TAVERN"));
        assert_eq!(tag(&tags, "mill").as_deref(), Some("MILL"));
        assert_eq!(tag(&tags, "gate").as_deref(), Some("GATE"));
        // One deleted: its tag is let go of.
        m.remove(&ResKey::parse("well", mg_core::ResType::UTW).unwrap());
        let (tags, read) = tags_read(&palette(&m), kind, Some(&m), &game, &mut kept);
        assert_eq!((read, tags.len(), kept.len()), (0, 3, 3));
    }

    /// What a palette made of the module is kept through a change that
    /// touched none of its kind's blueprints and no palette (an area's
    /// edit), and made again after one that did, or where what changed
    /// isn't known.
    #[test]
    fn a_palette_is_kept_through_a_change_to_none_of_its_blueprints() {
        use mg_edit::{Command, Edit, GffPath, Workspace};
        let kind = BlueprintKind::Waypoint;
        let (mut m, _) = waypoints(&["gate", "inn"]);
        let area = ResKey::parse("field", mg_core::ResType::ARE).unwrap();
        m.set_gff(area, &Gff::new(*b"ARE ")).unwrap();
        let mut ws = Workspace::new(m);
        let set = |key: ResKey, label: &str| {
            let value = Some(mg_gff::Value::String(b"x".to_vec()));
            let edit = Edit::SetField { key, path: GffPath::root(), label: label.into(), value };
            Command::new("Set", vec![edit])
        };
        let was = ws.revision();
        ws.apply(set(area, "Comments")).unwrap();
        assert!(untouched(&ws, was, kind), "an area's edit");
        let inn = ResKey::parse("inn", mg_core::ResType::UTW).unwrap();
        let after_area = ws.revision();
        ws.apply(set(inn, "Comment")).unwrap();
        assert!(!untouched(&ws, after_area, kind), "one of its blueprints");
        assert!(untouched(&ws, after_area, BlueprintKind::Item), "another kind's blueprint");
        assert!(!untouched(&ws, was, kind), "the area's edit and the blueprint's");
        ws.undo().unwrap();
        assert!(!untouched(&ws, after_area + 1, kind), "its undo");
        // (Not known: from before what the workspace keeps.)
        assert!(!untouched(&ws, 0, kind));
    }

    /// The places of the blueprints a search finds, node by node, are the
    /// rows each node shows (looked up one by one before, each frame).
    #[test]
    fn the_rows_a_search_finds_are_kept_by_node() {
        let named = |r: &str| PaletteBlueprint {
            resref: ResRef::from_str(r).unwrap(),
            name: mg_module::palette::PaletteName::StrRef(0),
            cr: None,
            faction: None,
        };
        let node = |blueprints: Vec<PaletteBlueprint>, children: Vec<PaletteNode>| PaletteNode {
            name: mg_module::palette::PaletteName::StrRef(0),
            id: None,
            blueprints,
            children,
        };
        let nodes = vec![
            node(
                vec![named("a1"), named("b1"), named("a2")],
                vec![node(vec![named("b2"), named("a3")], vec![])],
            ),
            node(vec![named("b3")], vec![]),
        ];
        let found: std::collections::HashSet<ResRef> =
            ["a1", "a2", "a3"].iter().map(|r| ResRef::from_str(r).unwrap()).collect();
        let (meta, picks) = build_node_meta(&nodes, Some(&found));
        let rows = |i: usize, n: &PaletteNode| -> Vec<String> {
            let m = meta[i];
            picks[m.picks_from as usize..][..m.picks as usize]
                .iter()
                .map(|&p| n.blueprints[p as usize].resref.to_string())
                .collect()
        };
        assert_eq!(rows(0, &nodes[0]), ["a1", "a2"]);
        assert_eq!(rows(1, &nodes[0].children[0]), ["a3"]);
        assert!(rows(2, &nodes[1]).is_empty());
        assert_eq!((meta[0].shown, meta[2].shown), (3, 0));
        let (meta, picks) = build_node_meta(&nodes, None);
        assert!(picks.is_empty());
        assert_eq!((meta[0].shown, meta[0].total), (5, 5));
    }

    /// A module with no custom palette has it made once, and brought up
    /// to date with what changed since: as it would be made anew.
    #[test]
    fn a_palette_made_for_a_module_without_one_is_kept_up_to_date() {
        let kind = BlueprintKind::Waypoint;
        let (mut m, game) = waypoints(&["gate", "inn", "well"]);
        let saved = m.clone();
        let anew = |m: &mg_module::Module| {
            Palette::read(&mg_module::palette::rebuild_custom_palette(m, &game, kind).unwrap())
        };
        let mut made = HashMap::new();
        let mut now = |m: &mg_module::Module| {
            Palette::read(&custom_palette_now(m, &saved, &game, kind, &mut made).unwrap())
        };
        let first = now(&m);
        assert_eq!(first, anew(&m));
        assert_eq!(first.blueprints().len(), 3);
        assert_eq!(now(&m), first, "nothing changed");
        // One named anew, one added, one deleted; then another kind's
        // resource, which changes nothing of it.
        set_waypoint(&mut m, "inn", "tavern");
        assert_eq!(now(&m), anew(&m));
        set_waypoint(&mut m, "mill", "mill");
        assert_eq!(now(&m), anew(&m));
        m.remove(&ResKey::parse("well", mg_core::ResType::UTW).unwrap());
        let after = now(&m);
        assert_eq!(after, anew(&m));
        assert_eq!(after.blueprints().len(), 3);
        m.set(ResKey::parse("hello", mg_core::ResType::NSS).unwrap(), b"void main() {}".to_vec());
        assert_eq!(now(&m), after);
        assert!(!m.contains(&kind.custom_key()), "nothing written into the module");
    }

    /// Favorites and Recent are found in a palette as they were when every
    /// blueprint was compared by its text for each (the review's B23): in
    /// the list's order, those of the palette's type that it has, the
    /// first of a ResRef it has twice.
    #[test]
    fn remembered_blueprints_are_found_as_by_looking_through_them_all() {
        let named = |resref: &str, n: u32| PaletteBlueprint {
            resref: ResRef::from_str(resref).unwrap(),
            name: mg_module::palette::PaletteName::StrRef(n),
            cr: None,
            faction: None,
        };
        let node = |children, blueprints| PaletteNode {
            name: mg_module::palette::PaletteName::StrRef(0),
            id: Some(1),
            children,
            blueprints,
        };
        let inner = node(Vec::new(), vec![named("wolf", 3), named("bat", 4)]);
        let palette = Palette {
            nodes: vec![
                node(vec![inner], vec![named("orc", 1), named("bat", 2)]),
                node(vec![], vec![]),
            ],
        };
        let list: Vec<String> = [
            "utc:bat",
            "uti:orc",
            "utc:gone",
            "utc:wolf",
            "utcx:orc",
            "utc:orc",
            "utc:Not A Name!",
        ]
        .map(String::from)
        .into();
        let kind = BlueprintKind::Creature;
        let all = palette.blueprints();
        let by_text: Vec<PaletteBlueprint> = list
            .iter()
            .filter_map(|r| r.strip_prefix("utc:"))
            .filter_map(|r| all.iter().find(|(_, b)| b.resref.to_string() == r).map(|(_, b)| *b))
            .cloned()
            .collect();
        let found = remembered_in(&palette, kind, &list);
        assert_eq!(found, by_text);
        assert_eq!(found, [named("bat", 2), named("wolf", 3), named("orc", 1)]);
        assert!(remembered_in(&palette, BlueprintKind::Item, &list[..1]).is_empty());
    }

    /// The arrow keys in the Gallery: a category's five pictures three to
    /// a row; Up and Down a row at a time (to the nearest picture across),
    /// out of the pictures past the first and the last row; Left and Right
    /// a picture at a time, Left from the first to the category.
    #[test]
    fn the_arrow_keys_go_a_row_of_pictures_at_a_time_in_the_gallery() {
        use egui::Key::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp};
        let key = |name: &str| ResKey::parse(name, mg_core::ResType::UTP).unwrap();
        let mut l = Listed::default();
        let category = l.branch(Branch::Category(vec![0]), None, 0.0); // row 0
        for (i, name) in ["a", "b", "c", "d", "e"].into_iter().enumerate() {
            l.rows.push((category, Some(key(name)))); // rows 1 to 5
            let at = egui::pos2(10.0 * (i % 3) as f32, 10.0 * (i / 3) as f32);
            l.places.insert(i + 1, egui::Rect::from_min_size(at, egui::vec2(8.0, 8.0)));
        }
        l.branch(Branch::Category(vec![1]), None, 0.0); // row 6
        l.branches[category].open = true;
        assert_eq!(l.step(1, ArrowDown), Step::To(4), "a under d");
        assert_eq!(l.step(3, ArrowDown), Step::To(5), "the nearest across: e under c");
        assert_eq!(l.step(5, ArrowUp), Step::To(2));
        assert_eq!(l.step(4, ArrowDown), Step::To(6), "past the last row");
        assert_eq!(l.step(2, ArrowUp), Step::To(0), "past the first row: the category");
        assert_eq!(l.step(3, ArrowRight), Step::To(4), "on to the next row's first");
        assert_eq!(l.step(5, ArrowRight), Step::Stay);
        assert_eq!(l.step(4, ArrowLeft), Step::To(3));
        assert_eq!(l.step(1, ArrowLeft), Step::To(0), "from the first, to its category");
        assert_eq!(l.step(6, ArrowUp), Step::To(5), "a category's row: as in the list");
    }

    /// Home and End, Page Up and Page Down: a heading (row 0, at 0) over a
    /// category's seven pictures three to a row (rows 1 to 7, 100 apart),
    /// a closed heading (row 8, at 330) and another (row 9, at 352); a
    /// list's height in sight of 250. Home and End to the first and last
    /// row showing; a page down from a picture to the farthest row of
    /// pictures within the page, the one nearest across, or past the
    /// pictures to the headings; a page up likewise; from a row with no
    /// row within the page, the next.
    #[test]
    fn home_end_and_the_page_keys_go_through_headings_and_pictures() {
        use egui::Key::{End, Home, PageDown, PageUp};
        let key = |name: &str| ResKey::parse(name, mg_core::ResType::UTP).unwrap();
        let mut l = Listed { page: 250.0, ..Default::default() };
        let category = l.branch(Branch::Category(vec![0]), None, 0.0); // row 0
        for i in 0..7 {
            l.tops.insert(l.rows.len(), 22.0 + 100.0 * (i / 3) as f32);
            l.rows.push((category, Some(key(&format!("p{i}"))))); // rows 1 to 7
            let at = egui::pos2(100.0 * (i % 3) as f32, 22.0 + 100.0 * (i / 3) as f32);
            l.places.insert(i + 1, egui::Rect::from_min_size(at, egui::vec2(90.0, 90.0)));
        }
        let closed = l.branch(Branch::Category(vec![1]), None, 330.0); // row 8
        l.rows.push((closed, Some(key("hidden")))); // 9: in a closed heading
        l.tops.insert(9, 340.0);
        l.branch(Branch::Category(vec![2]), None, 352.0); // row 10
        l.branches[category].open = true;
        assert_eq!(l.step(4, Home), Step::To(0), "the first row");
        assert_eq!(l.step(4, End), Step::To(10), "the last row showing");
        assert_eq!(l.step(0, Home), Step::Stay);
        assert_eq!(l.step(10, End), Step::Stay);
        // From p1 (row 2, at 22): rows within 272 are those at 122 and
        // 222; the farthest at 222, p7 alone there.
        assert_eq!(l.step(2, PageDown), Step::To(7));
        // From p4 (row 5, at 122): within 372, the headings at 330 and
        // 352 (the closed one's picture not showing): the last.
        assert_eq!(l.step(5, PageDown), Step::To(10));
        // From p6 (row 7, at 222) up: within -28, the heading at 0.
        assert_eq!(l.step(7, PageUp), Step::To(0));
        // From the last heading (at 352) up: within 102, p3 to p5 at 122,
        // the nearest across of none (a heading): the first of them.
        assert_eq!(l.step(10, PageUp), Step::To(4));
        // A page shorter than a row: at least the next row.
        l.page = 50.0;
        assert_eq!(l.step(2, PageDown), Step::To(3), "the next row");
        assert_eq!(l.step(5, PageUp), Step::To(4));
    }

    /// The arrow keys in a tree: Favorites (open) with one blueprint, a
    /// category (open) holding a closed category and two blueprints, and
    /// a closed category after it.
    #[test]
    fn the_arrow_keys_move_through_the_rows_showing() {
        use egui::Key::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp};
        let key = |name: &str| ResKey::parse(name, mg_core::ResType::UTP).unwrap();
        let mut l = Listed::default();
        let favorites = l.branch(Branch::Remembered("Favorites"), None, 0.0); // row 0
        l.rows.push((favorites, Some(key("chest")))); // 1
        let outer = l.branch(Branch::Category(vec![0]), None, 0.0); // 2
        let inner = l.branch(Branch::Category(vec![0, 0]), Some(outer), 0.0); // 3
        l.rows.push((inner, Some(key("hidden")))); // 4: drawn, its branch closing
        l.rows.push((outer, Some(key("chest")))); // 5
        l.rows.push((outer, Some(key("barrel")))); // 6
        let last = l.branch(Branch::Category(vec![1]), None, 0.0); // 7
        l.branches[favorites].open = true;
        l.branches[outer].open = true;
        // Down and Up: the rows showing, not those of a closed branch.
        assert_eq!(l.step(3, ArrowDown), Step::To(5));
        assert_eq!(l.step(5, ArrowUp), Step::To(3));
        assert_eq!(l.step(6, ArrowDown), Step::To(7));
        assert_eq!(l.step(7, ArrowDown), Step::Stay);
        assert_eq!(l.step(0, ArrowUp), Step::Stay);
        // Left: a blueprint to its branch, an open branch closes, a closed
        // one goes to the branch it is in (none: it stays).
        assert_eq!(l.step(6, ArrowLeft), Step::To(2));
        assert_eq!(l.step(2, ArrowLeft), Step::Fold(outer, false));
        assert_eq!(l.step(3, ArrowLeft), Step::To(2));
        assert_eq!(l.step(7, ArrowLeft), Step::Stay);
        assert_eq!(l.step(1, ArrowLeft), Step::To(0));
        // Right: a closed branch opens, an open one goes into it, a
        // blueprint stays.
        assert_eq!(l.step(3, ArrowRight), Step::Fold(inner, true));
        assert_eq!(l.step(2, ArrowRight), Step::To(3));
        assert_eq!(l.step(0, ArrowRight), Step::To(1));
        assert_eq!(l.step(5, ArrowRight), Step::Stay);
        assert_eq!(l.step(7, ArrowRight), Step::Fold(last, true));
        // A blueprint is found in its category before Favorites; a row by
        // its branch.
        assert_eq!(l.blueprint(key("chest")), Some(5));
        let at = |branch, blueprint| At {
            kind: BlueprintKind::Placeable,
            custom: false,
            grouping: None,
            branch,
            blueprint,
        };
        assert_eq!(l.find(&at(Branch::Remembered("Favorites"), Some(key("chest")))), Some(1));
        assert_eq!(l.find(&at(Branch::Category(vec![0, 0]), None)), Some(3));
        assert!(!l.shows(4) && l.shows(3));
    }

    #[test]
    fn a_blueprints_row_names_what_is_asked_for() {
        let r = ResRef::from_str("nw_goblina").unwrap();
        assert_eq!(blueprint_label("Goblin", None, None), "Goblin");
        assert_eq!(blueprint_label("Goblin", Some(r), None), "Goblin (nw_goblina)");
        assert_eq!(blueprint_label("Goblin", None, Some(0.5)), "Goblin  (CR 0.5)");
        assert_eq!(blueprint_label("Goblin", Some(r), Some(1.0)), "Goblin (nw_goblina)  (CR 1)");
    }

    #[test]
    fn copies_take_the_next_free_number() {
        let taken = |n: &str| ["nw_chicken001", "nw_chicken002"].contains(&n);
        assert_eq!(copy_resref("nw_chicken", &taken).unwrap().to_string(), "nw_chicken003");
        assert_eq!(copy_resref("nw_chicken001", &taken).unwrap().to_string(), "nw_chicken003");
        // At most 16 characters.
        let long = copy_resref("abcdefghijklmnop", &|_| false).unwrap().to_string();
        assert_eq!(long, "abcdefghijklm001");
        // By a builder's code: theirs begins it, once.
        let ab = mg_module::project_file::Naming::new("ab", "{code}_{name}");
        let copy = |from: &str, taken: &dyn Fn(&str) -> bool| {
            copy_resref_named(from, &ab, taken).unwrap().to_string()
        };
        assert_eq!(copy("nw_chicken", &taken), "ab_nw_chicken001");
        assert_eq!(copy("ab_chicken001", &|n| n == "ab_chicken001"), "ab_chicken002");
        assert_eq!(copy("abcdefghijklmnop", &|_| false), "ab_abcdefghij001");
    }

    /// The custom talk table read again (its Save) is found by its new
    /// lines, and is no load order change: the browser keeps its views
    /// (review U's 1, where every Save emptied them).
    #[test]
    fn a_talk_table_read_again_is_found_by_its_new_lines_and_keeps_the_browser() {
        use mg_resman::{LayerClass, MemContainer, ResMan, priority};
        let tlk_key = ResKey::parse("mg_t", mg_core::ResType::TLK).unwrap();
        let layer = |text: &str| {
            let mut tlk = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
            tlk.entries.push(mg_tlk::TlkEntry::text(text));
            let mut c = MemContainer::new();
            c.insert(tlk_key, tlk.to_bytes().unwrap());
            c
        };
        let mut rm = ResMan::new();
        rm.add(priority::HAK, "hak:old", LayerClass::Erf, layer("Old sword"));
        let game = mg_rules::GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut m = mg_module::Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        ifo.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mg_t".to_vec()));
        m.set_gff(ResKey::parse("module", mg_core::ResType::IFO).unwrap(), &ifo).unwrap();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.game = Some(Arc::new(game));
        app.ws = Some(mg_edit::Workspace::new(m));
        app.reload_custom_tlk();
        assert!(app.game.as_deref().unwrap().custom_tlk().is_some(), "{:?}", app.log.entries);

        let blueprint = PaletteBlueprint {
            resref: ResRef::from_str("sword").unwrap(),
            name: mg_module::palette::PaletteName::StrRef(0x0100_0000),
            cr: None,
            faction: None,
        };
        let palette = Arc::new(Palette {
            nodes: vec![PaletteNode {
                name: mg_module::palette::PaletteName::StrRef(0),
                id: Some(1),
                children: Vec::new(),
                blueprints: vec![blueprint],
            }],
        });
        let tags: Tags = Default::default();
        let kind = BlueprintKind::Item;
        let finds = |app: &mut Moonglow, filter: &str| {
            app.palette.filter = filter.into();
            found(app, &palette, kind, false, &tags).0.len()
        };
        assert_eq!(finds(&mut app, "old"), 1);

        // The table saved with the line renamed, read again.
        let game = Arc::get_mut(app.game.as_mut().unwrap()).unwrap();
        game.resman.add(priority::HAK_USER, "hak:new", LayerClass::Erf, layer("New sword"));
        let pictured = (tlk_key, None);
        app.browser.pictures.insert(pictured.clone(), 1.0);
        let load_order = app.load_order;
        app.reload_custom_tlk();
        assert_eq!(finds(&mut app, "old"), 0, "the same search, found again");
        assert_eq!(finds(&mut app, "new"), 1, "by the new line");
        assert_eq!(app.load_order, load_order, "no load order change");
        assert!(app.browser.pictures.contains_key(&pictured), "the browser keeps its own");
    }

    /// A flat list's keys: Up and Down stay in the rows, Left and Right
    /// close and open a heading, go into it and out to it, and a page goes
    /// a list's height (at least a row).
    #[test]
    fn a_flat_list_s_keys_follow_its_rows() {
        use egui::Key::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp, End, Home, PageDown, PageUp};
        let row = |depth, open, top| FlatRow { depth, open, enter_folds: true, top };
        // A closed heading, an open one with two rows in it, a row.
        let rows = [
            row(0, Some(false), 0.0),
            row(0, Some(true), 22.0),
            row(1, None, 44.0),
            row(1, None, 66.0),
            row(0, None, 88.0),
        ];
        let step = |at, key| flat_step(&rows, at, key, 44.0);
        assert_eq!(step(0, ArrowUp), Step::Stay);
        assert_eq!(step(0, ArrowDown), Step::To(1));
        assert_eq!(step(3, ArrowUp), Step::To(2));
        assert_eq!(step(4, ArrowDown), Step::Stay);
        assert_eq!(step(0, End), Step::To(4));
        assert_eq!(step(3, Home), Step::To(0));
        assert_eq!(step(0, ArrowRight), Step::Fold(0, true));
        assert_eq!(step(1, ArrowRight), Step::To(2), "into the open heading");
        assert_eq!(step(1, ArrowLeft), Step::Fold(1, false));
        assert_eq!(step(3, ArrowLeft), Step::To(1), "out to its heading");
        assert_eq!(step(0, ArrowLeft), Step::Stay, "from a closed heading at the top");
        assert_eq!(step(4, ArrowRight), Step::Stay);
        assert_eq!(step(0, PageDown), Step::To(2), "a list's height away");
        assert_eq!(step(4, PageUp), Step::To(2));
        let near = [row(0, None, 0.0), row(0, None, 100.0)];
        assert_eq!(flat_step(&near, 0, PageDown, 10.0), Step::To(1), "at least the next row");
    }

    /// A flat list's cursor whose row is no longer drawn goes to the
    /// heading it was in, else to the row that took its place.
    #[test]
    fn a_flat_list_s_cursor_moves_off_a_row_no_longer_drawn() {
        let draw = |list: &mut FlatList<&'static str>, rows: &[(&'static str, usize)]| {
            for (key, depth) in rows {
                list.slot(*key, *depth);
            }
            list.end(100.0);
        };
        let all = [("a", 0), ("b", 0), ("b1", 1), ("b2", 1), ("c", 0)];
        let mut list = FlatList::default();
        draw(&mut list, &all);
        list.cursor = Some("b2");
        // Its heading closed: the cursor goes to the heading.
        draw(&mut list, &[("a", 0), ("b", 0), ("c", 0)]);
        assert_eq!(list.cursor, Some("b"));
        // A row searched out, with none above it to be in: the row that took
        // its place.
        draw(&mut list, &all);
        list.cursor = Some("c");
        draw(&mut list, &[("a", 0), ("b", 0)]);
        assert_eq!(list.cursor, Some("b"));
        // A row still drawn stays.
        draw(&mut list, &all);
        assert_eq!(list.cursor, Some("b"));
        // Nothing drawn: the cursor is left as it is.
        draw(&mut list, &[]);
        assert_eq!(list.cursor, Some("b"));
        assert!(!list.takes_keys());
    }

    /// Down out of an empty search (the first row asked for, nothing
    /// drawn to go to) is not kept: rows that appear later, with the keys
    /// the palette's again, are not moved to or scrolled to.
    #[test]
    fn a_first_row_asked_for_is_not_kept_for_later_rows() {
        let ctx = egui::Context::default();
        let mut list: FlatList<&'static str> = FlatList::default();
        let frame =
            |list: &mut FlatList<&'static str>, asks: bool, keys: bool, rows: &[&'static str]| {
                let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                    let ctx = ui.ctx().clone();
                    give_keys(&ctx, if keys { Keys::Palette } else { Keys::Area });
                    if asks {
                        list.ask_first(&ctx);
                    }
                    list.keys(ui);
                    for row in rows {
                        list.slot(*row, 0);
                    }
                    list.end(100.0);
                });
                out.textures_delta.clear();
            };
        // Asked on an empty list: used up there.
        frame(&mut list, true, true, &[]);
        frame(&mut list, false, true, &["a", "b"]);
        frame(&mut list, false, true, &["a", "b"]);
        assert_eq!(list.cursor, None, "rows appeared later");
        assert!(!list.reveal, "and nothing is scrolled to");
        // Asked, the keys the palette's no more: let go of too.
        frame(&mut list, true, false, &["a", "b"]);
        frame(&mut list, false, true, &["a", "b"]);
        assert_eq!(list.cursor, None, "the keys went elsewhere");
        // Asked with rows and the keys: the first row.
        frame(&mut list, true, true, &["a", "b"]);
        assert_eq!(list.cursor, Some("a"));
    }
}
