//! The Appearance Gallery: every appearance of a kind as a picture, in a
//! grid, in a tab of its own: placeables (placeables.2da's models),
//! creatures (appearance.2da: the creature given appearances as it is in
//! each, its head, colors and what it wears, else a plain body) and doors
//! (genericdoors.2da). Opened from Tools, the palette's All Appearances…,
//! and Gallery… beside the appearance in a placeable's, creature's or
//! door's Properties. A click gives the appearance to the object whose
//! Properties opened it, else to the objects of that kind selected in the
//! area.
//!
//! As the AppearanceGallery board has it (M4's U26): the kit's header
//! (the kind, Find, the pictures' size, the count), the hint strip, the
//! grid; the Preview and the Appearance panel of the row chosen in the
//! Right Dock (`shell::Brings`), and the Status Bar's hints and count
//! ([`shown`]).

use std::collections::HashMap;

use mg_core::ResType;
use mg_edit::{Command, Edit, GffPath};
use mg_gff::Value;
use mg_resman::ResKey;
use mg_rules::ChoiceColumns;

use crate::model_view::Pictured;
use crate::{Action, Moonglow};

/// An appearance dragged from the gallery (dropped in an area view, a
/// placeable with it is placed there): its placeables.2da row and name.
#[derive(Debug, Clone, PartialEq)]
pub struct DraggedAppearance {
    pub row: usize,
    pub name: String,
}

/// What a gallery shows the appearances of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Kind {
    #[default]
    Placeable,
    Creature,
    Door,
}

impl Kind {
    pub(crate) const ALL: [Kind; 3] = [Kind::Placeable, Kind::Creature, Kind::Door];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Kind::Placeable => "Placeables",
            Kind::Creature => "Creatures",
            Kind::Door => "Doors",
        }
    }

    /// One, and several, of what it gives appearances to.
    fn noun(self, n: usize) -> String {
        let one = match self {
            Kind::Placeable => "placeable",
            Kind::Creature => "creature",
            Kind::Door => "door",
        };
        if n == 1 { format!("1 {one}") } else { format!("{n} {one}s") }
    }

    /// Its table, and the columns its rows are named by.
    fn table(self) -> (&'static str, ChoiceColumns<'static>) {
        match self {
            Kind::Placeable => {
                ("placeables", ChoiceColumns { name: Some("StrRef"), label: Some("Label") })
            }
            Kind::Creature => {
                ("appearance", ChoiceColumns { name: Some("STRING_REF"), label: Some("LABEL") })
            }
            Kind::Door => {
                ("genericdoors", ChoiceColumns { name: Some("Name"), label: Some("Label") })
            }
        }
    }

    /// The field an object keeps its appearance in.
    fn field(self) -> &'static str {
        match self {
            Kind::Placeable => "Appearance",
            Kind::Creature => "Appearance_Type",
            Kind::Door => "GenericType_New",
        }
    }

    /// The fields set to give an object row `row`.
    fn fields(self, row: i64) -> Vec<(&'static str, Value)> {
        match self {
            Kind::Placeable => vec![("Appearance", Value::Dword(row as u32))],
            Kind::Creature => vec![("Appearance_Type", Value::Word(row as u16))],
            // (A generic door: the tileset's own type 0, and the old byte.)
            Kind::Door => {
                let mut fields = vec![
                    ("Appearance", Value::Dword(0)),
                    ("GenericType_New", Value::Dword(row as u32)),
                ];
                if (0..=255).contains(&row) {
                    fields.push(("GenericType", Value::Byte(row as u8)));
                }
                fields
            }
        }
    }

    /// Why row `row` has no model to draw, if it has none
    /// ([`mg_rules::appearance`], as the Properties and Verify say it).
    fn problem(self, game: &mg_rules::GameData, row: usize) -> Option<String> {
        let mut object = mg_gff::Struct::default();
        for (label, value) in self.fields(row as i64) {
            object.set(label, value);
        }
        game.appearance_problem(self.object(), &object).map(|p| p.to_string())
    }

    /// The wizard a double click starts with the appearance.
    pub(crate) fn wizard(self) -> &'static str {
        match self {
            Kind::Placeable => "Placeable Wizard",
            Kind::Creature => "Creature Wizard",
            Kind::Door => "Door Wizard",
        }
    }

    /// The blueprint type an object of it is.
    fn restype(self) -> ResType {
        match self {
            Kind::Placeable => ResType::UTP,
            Kind::Creature => ResType::UTC,
            Kind::Door => ResType::UTD,
        }
    }

    /// The kind whose objects are blueprints of `restype`.
    fn of_restype(restype: ResType) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.restype() == restype)
    }

    /// The column naming row's model.
    fn model_column(self) -> &'static str {
        match self {
            Kind::Creature => "RACE",
            Kind::Placeable | Kind::Door => "ModelName",
        }
    }

    /// The kind's icon, a picture's while it is made.
    fn icon(self) -> mg_uikit::Icon {
        crate::icons::object(self.object())
    }

    fn object(self) -> mg_area::ObjectKind {
        match self {
            Kind::Placeable => mg_area::ObjectKind::Placeable,
            Kind::Creature => mg_area::ObjectKind::Creature,
            Kind::Door => mg_area::ObjectKind::Door,
        }
    }

    /// What pictures row `row`: its model, or a creature's look: the
    /// creature the gallery gives appearances to as it is (`dressed`,
    /// [`Thumbnails::creature`](crate::model_view::Thumbnails::creature)),
    /// else a plain body.
    fn pictured(self, table: &mg_2da::TwoDa, row: usize, dressed: Option<u64>) -> Option<Pictured> {
        match self {
            Kind::Creature => Some(match dressed {
                Some(id) => Pictured::Creature(id, row as u16),
                None => Pictured::Look(mg_preview::CreatureLook::new(row as u16)),
            }),
            Kind::Placeable | Kind::Door => {
                let model = table.get(row, "ModelName")?.to_ascii_lowercase();
                ResKey::parse(&model, ResType::MDL).map(Pictured::Resource)
            }
        }
    }
}

/// A gallery's state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gallery {
    pub kind: Kind,
    pub filter: String,
    /// Scrolled to the current appearance once, when it opens.
    placed: bool,
    /// The object it was opened for (its document, and where in it): its
    /// Properties' Gallery…. Else the area's selection is given the
    /// appearances.
    pub target: Option<(ResKey, GffPath)>,
    /// The row whose picture was clicked or the keys moved to last (its
    /// kind's), what the Preview and the Appearance panel show; none
    /// until one is: they show the row the gallery opened at.
    pub(crate) look: Option<i64>,
    /// As the gallery was last drawn, for the panels and the Status Bar
    /// drawn before or after it: the row the panels show, what a click
    /// does, the table's count, and its grid's id (whether it has the
    /// keys).
    pub(crate) drawn: Drawn,
    /// The pass its tab was last drawn in, and whether its grid is still
    /// to take the keys: it takes them as the tab comes to the front of
    /// its pane (opened, or its tab chosen), not after.
    pub(crate) shown: (u64, bool),
    /// Down in Find was pressed: the grid takes the keys, its cursor on
    /// the first picture shown.
    down: bool,
    /// The picture the grid's cursor starts at, once Down left Find for it
    /// (in place of the one chosen), and how many pictures there were:
    /// let go of when their number changes (Find typed again).
    entry: Option<(usize, usize)>,
    /// The grid is to be scrolled to its top, where its first picture is.
    top: bool,
    /// Per-row `appearance_problem` results cached by `(kind, app.load_order)`.
    pub(crate) problems: ((Kind, u64), HashMap<usize, Option<String>>),
}

/// What the gallery was last drawn as ([`Gallery::drawn`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Drawn {
    pub(crate) row: Option<i64>,
    pub(crate) mode: Mode,
    /// The count as the header says it, without the table's name ("838
    /// appearances", "12 of 838 appearances").
    pub(crate) count: String,
    pub(crate) grid: Option<egui::Id>,
}

/// What a click on a picture does, as the gallery is: the hint strip's
/// words and the Status Bar's hints both by it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Opened for one object (its Properties' Gallery…): a click gives it
    /// the appearance.
    Target,
    /// Objects of the kind selected in the area: a click gives them it.
    Selection,
    /// Placeables, opened on their own, some selected: a click takes one
    /// to place, a right click gives the selected the appearance.
    PlaceGive,
    /// Placeables, opened on their own, none selected: a click takes one
    /// to place.
    Place,
    /// Creatures or doors, none selected: a click does nothing but show
    /// it in the panels.
    #[default]
    Look,
}

impl Gallery {
    /// A gallery for one object, opening at its appearance.
    pub(crate) fn of(kind: Kind, key: ResKey, path: GffPath) -> Gallery {
        Gallery { kind, target: Some((key, path)), ..Default::default() }
    }
}

/// The side of a gallery's pictures, points: Options keep what the slider
/// leaves.
pub(crate) fn tile_side(app: &Moonglow) -> f32 {
    f32::from(app.settings.gallery_tile.unwrap_or(DEFAULT_TILE)).clamp(MIN_TILE, MAX_TILE)
}

const DEFAULT_TILE: u16 = 128;
const MIN_TILE: f32 = 64.0;
const MAX_TILE: f32 = 220.0;

/// The slider for the pictures' size (the palette's Gallery has it too).
pub(crate) fn size_slider(app: &mut Moonglow, ui: &mut egui::Ui) {
    let side = tile_side(app);
    if let Some(side) = crate::widgets::picture_size_slider(ui, side, MIN_TILE..=MAX_TILE, None) {
        app.settings.gallery_tile = Some(side);
    }
}

/// What was done to the gallery's pictures this frame: the row clicked,
/// double-clicked, given to the selection from its menu, or the keys
/// moved to.
#[derive(Debug, Default)]
struct Clicked {
    click: Option<i64>,
    double: Option<i64>,
    give: Option<i64>,
    moved: Option<i64>,
}

/// The rows of the kind's table the grid shows: all of them, creatures by
/// name (sorted once), and those Find finds (found once for what is
/// typed), kept: none of it is made again in a frame that changes
/// neither.
struct Rows {
    name: &'static str,
    table: std::sync::Arc<mg_2da::TwoDa>,
    all: std::sync::Arc<Vec<mg_rules::Choice>>,
    found: std::sync::Arc<Vec<usize>>,
}

impl Rows {
    fn of(app: &Moonglow, ctx: &egui::Context, kind: Kind, filter: &str) -> Option<Rows> {
        let game = app.game.as_deref()?;
        let (name, cols) = kind.table();
        let table = game.table(name).ok()?;
        // (The table's rows as the game data keeps them, not a copy a frame;
        // creatures by name, sorted once: the table's order is no one's.)
        let mut all = game.choice_list(name, cols).ok()?;
        if kind == Kind::Creature {
            all = crate::blueprint::by_name_kept(ctx, "gallery", &all);
        }
        let found = crate::blueprint::found_kept(ctx, "gallery", &all, filter);
        Some(Rows { name, table, all, found })
    }

    fn row(&self, i: usize) -> &mg_rules::Choice {
        &self.all[self.found[i]]
    }

    /// Where row `row` is among those found, kept for the list found and
    /// the row (a walk of 17,100 creatures a frame, before).
    fn place(&self, ctx: &egui::Context, row: Option<i64>) -> Option<usize> {
        let id = egui::Id::new("appearance-gallery-place");
        crate::blueprint::kept_for(ctx, id, &self.found, &row, || {
            let row = row?;
            (0..self.found.len()).find(|&i| self.row(i).row as i64 == row)
        })
    }
}

/// The count the header and the Status Bar show: "838 appearances
/// (appearance.2da)", "12 of 838 appearances (appearance.2da)" while Find
/// leaves some out.
fn count_words(rows: &Rows, table: bool) -> String {
    let (found, all) = (rows.found.len(), rows.all.len());
    let words = match found == all {
        true => format!("{all} appearances"),
        false => format!("{found} of {all} appearances"),
    };
    if table { format!("{words} ({}.2da)", rows.name) } else { words }
}

/// The grid: the kind's rows found, as pictures, opening at `current` (the
/// row chosen, ringed; or `taking`, the one taken to place), in a scroll
/// with the board's room round the cells. What was clicked; `give`:
/// whether a picture's menu offers to give it to the selection.
fn grid(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    state: &mut Gallery,
    rows: &Rows,
    (current, taking): (Option<i64>, Option<i64>),
    dressed: Option<u64>,
    give: bool,
) -> Clicked {
    let kind = state.kind;
    let Some(game) = app.game.clone() else { return Clicked::default() };
    if state.problems.0 != (kind, app.load_order) {
        state.problems.0 = (kind, app.load_order);
        state.problems.1.clear();
    }
    let name = rows.name;
    let side = tile_side(app);
    let ready = &app.thumbnails;
    let (mut shown, mut pick) = (Vec::new(), Clicked::default());
    let icon = kind.icon();
    let id = grid_id(kind);
    let keys = !crate::shell::keys_blocked(ui.ctx());
    let at = rows.place(ui.ctx(), current.or(taking));
    let count = rows.found.len();
    if state.entry.is_some_and(|(_, was)| was != count) {
        state.entry = None;
    }
    let entry = state.entry.map(|(at, _)| at).filter(|e| *e < count);
    let grid = mg_uikit::gallery::GalleryGrid::new(id, count, side)
        .cursor(entry.or(at))
        .keys_allowed(keys)
        .name(kind.name());
    if count == 0 {
        let what = match rows.all.is_empty() {
            true => format!("No {} in {name}.2da", kind.name().to_lowercase()),
            false => format!("No {} found", kind.name().to_lowercase()),
        };
        let mut empty = mg_uikit::dialog::EmptyState::new(icon, what);
        if !rows.all.is_empty() {
            empty = empty
                .why("No appearance matches.")
                .button(mg_uikit::KitButton::new(None, "Clear Find"));
        }
        if empty.show(ui).first().is_some_and(egui::Response::clicked) {
            state.filter.clear();
        }
        return pick;
    }
    let area = egui::ScrollArea::vertical().id_salt(kind).auto_shrink([false, false]);
    let area = match std::mem::take(&mut state.top) {
        true => area.vertical_scroll_offset(0.0),
        false => area,
    };
    let shown_area = area.show(ui, |ui| {
        // (The board's room round the cells: 6 above, 4 right, 8 below and
        // left.)
        let room = egui::Margin { left: 8, right: 4, top: 6, bottom: 8 };
        egui::Frame::new()
            .inner_margin(room)
            .show(ui, |ui| {
                let out = grid.show(ui, |ui, i, rect, tile, r, cell| {
                    let c = rows.row(i);
                    let key = kind.pictured(&rows.table, c.row, dressed);
                    let made = match key {
                        Some(k) => ready.made(k),
                        None => Some(None),
                    };
                    let chosen = current == Some(c.row as i64) || taking == Some(c.row as i64);
                    // (Marked where it has no model: a builder chose such rows
                    // and saw nothing say why.)
                    let problem = state
                        .problems
                        .1
                        .entry(c.row)
                        .or_insert_with(|| kind.problem(&game, c.row))
                        .as_deref();
                    mg_uikit::gallery::GalleryCell::new(
                        &c.text,
                        crate::widgets::cell_picture(made),
                    )
                    .kind(icon)
                    .selected(chosen)
                    .no_model(problem.is_some())
                    .paint(ui, rect, tile, cell, r);
                    let r = r.clone().on_hover_ui(|ui| {
                        if let Some(p) = problem {
                            ui.label(format!("{} ({name}.2da row {})\n{p}", c.text, c.row));
                        } else {
                            ui.label(format!("{} ({name}.2da row {})", c.text, c.row));
                        }
                    });
                    if r.double_clicked() {
                        pick.double = Some(c.row as i64);
                    } else if r.clicked() {
                        pick.click = Some(c.row as i64);
                    }
                    if give {
                        mg_uikit::context_menu(&r, |ui| {
                            if ui.button("Give to the Selection").clicked() {
                                pick.give = Some(c.row as i64);
                                ui.close();
                            }
                        });
                    }
                    // Dragged into an area's view: a placeable of it there.
                    if kind == Kind::Placeable && r.drag_started() {
                        r.dnd_set_drag_payload(DraggedAppearance {
                            row: c.row,
                            name: c.text.clone(),
                        });
                    }
                });
                // (Enter or Space on the keys' picture: its click.)
                if let Some(i) = out.activated {
                    pick.click = Some(rows.row(i).row as i64);
                }
                if let Some(i) = out.moved {
                    pick.moved = Some(rows.row(i).row as i64);
                }
                // (Those in sight and a row ahead: kept while they are, and
                // made if they aren't yet, a few a frame.)
                for i in out.ahead.clone() {
                    shown.extend(kind.pictured(&rows.table, rows.row(i).row, dressed));
                }
                out
            })
            .inner
    });
    // (Opened at the row chosen, once, where the grid put it.)
    if !state.placed {
        state.placed = match at {
            Some(at) => {
                crate::widgets::open_gallery_at(ui.ctx(), &shown_area, &shown_area.inner, at)
            }
            None => true,
        };
    }
    app.thumbnails.show(shown);
    pick
}

/// The grid's id for `kind` (its keys' cursor kept under it).
fn grid_id(kind: Kind) -> egui::Id {
    egui::Id::new(("appearance-gallery", kind))
}

/// The objects a gallery gives appearances to: their document, their
/// places in its list, the first one's appearance and (a creature) its
/// fields to picture it by
/// ([`Thumbnails::creature`](crate::model_view::Thumbnails::creature)),
/// and whether they all have the first one's appearance.
type Chosen = (ResKey, Vec<usize>, Option<i64>, Option<u64>, bool);

/// The fields a creature is pictured by
/// ([`Thumbnails::creature`](crate::model_view::Thumbnails::creature)),
/// kept for its place (`at`) and the workspace's revision: its fields
/// were written out and hashed each frame.
fn dressed_kept(
    thumbnails: &mut crate::model_view::Thumbnails,
    revision: Option<u64>,
    ctx: &egui::Context,
    at: (ResKey, GffPath),
    of: &mg_gff::Struct,
) -> u64 {
    let id = egui::Id::new("appearance-gallery-dressed");
    let key = (at, revision);
    type Kept = (((ResKey, GffPath), Option<u64>), u64);
    let kept: Option<Kept> = ctx.data(|d| d.get_temp(id));
    // (And while the pictures still have its fields: let go of with
    // them, when none was asked for a while, they are given again.)
    if let Some((_, kept)) = kept.filter(|(k, _)| *k == key)
        && thumbnails.has_creature(kept)
    {
        return kept;
    }
    let made = thumbnails.creature(of);
    ctx.data_mut(|d| d.insert_temp(id, (key, made)));
    made
}

/// The objects of the gallery's kind selected in the area shown (in its
/// GIT).
fn selected(app: &mut Moonglow, ctx: &egui::Context, kind: Kind) -> Option<Chosen> {
    let area = app.palette.area?;
    let view = app.area_views.get(&area)?;
    let object = kind.object();
    let chosen: Vec<usize> =
        view.selection.iter().filter(|(k, _)| *k == object).map(|&(_, i)| i).collect();
    let git = ResKey::new(area, ResType::GIT);
    let first = *chosen.first()?;
    let Moonglow { ws: Some(ws), thumbnails, .. } = app else { return None };
    let revision = Some(ws.revision());
    let doc = ws.doc(&git).ok()?;
    let list = doc.root.list(object.list())?;
    let of = list.get(first)?;
    let now = of.integer(kind.field());
    let alike = chosen.iter().all(|&i| list.get(i).and_then(|s| s.integer(kind.field())) == now);
    let path = GffPath::root().item(object.list(), first);
    let dressed =
        (kind == Kind::Creature).then(|| dressed_kept(thumbnails, revision, ctx, (git, path), of));
    Some((git, chosen, now, dressed, alike))
}

/// The header (the board's `.editor-header`): the kind (Placeables,
/// Creatures, Doors), Find, and at its right the pictures' size and the
/// count; what has no room goes into More.
fn header(app: &mut Moonglow, ui: &mut egui::Ui, state: &mut Gallery, count: &str) {
    use mg_uikit::header::{EditorHeader, Item};
    use std::cell::{Cell, RefCell};
    let choices = [
        (Kind::Placeable, "Placeables", "Placeables: placeables.2da"),
        (Kind::Creature, "Creatures", "Creatures: appearance.2da"),
        (Kind::Door, "Doors", "Doors: genericdoors.2da"),
    ];
    let kind = Cell::new(state.kind);
    let font = egui::TextStyle::Body.resolve(ui.style());
    let ppp = ui.ctx().pixels_per_point().to_bits();
    let pad = 2.0 * mg_uikit::metrics::CONTROL_PADDING_X;
    let sw_id = egui::Id::new("appearance-gallery-switch-width");
    let sw_key = (font.size.to_bits(), pad.to_bits(), ppp);
    let switch_width = ui
        .data(|d| d.get_temp::<((u32, u32, u32), f32)>(sw_id))
        .filter(|(k, _)| *k == sw_key)
        .map(|(_, w)| w)
        .unwrap_or_else(|| {
            let words = |w: &str| {
                ui.fonts_mut(|f| {
                    f.layout_no_wrap(w.to_string(), font.clone(), egui::Color32::PLACEHOLDER)
                })
                .size()
                .x
            };
            let widths: f32 = choices.iter().map(|c| words(c.1) + pad).sum();
            let w = widths.ceil() + (choices.len() - 1) as f32;
            ui.data_mut(|d| d.insert_temp(sw_id, (sw_key, w)));
            w
        });
    let switch = |ui: &mut egui::Ui| {
        let mut chosen = kind.get();
        let room = egui::vec2(switch_width, mg_uikit::metrics::CONTROL_HEIGHT);
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        ui.allocate_ui_with_layout(room, layout, |ui| {
            mg_uikit::segmented(ui, KIND, &mut chosen, &choices);
        });
        kind.set(chosen);
    };
    let filter = RefCell::new(std::mem::take(&mut state.filter));
    let down = Cell::new(false);
    let find = |ui: &mut egui::Ui| {
        let mut text = filter.borrow_mut();
        let left =
            crate::widgets::SearchField::new(egui::Id::new("gallery-filter-field"), &mut text)
                .name("Find")
                .hint("Find")
                .width(FIND_WIDTH)
                .find_over()
                .down()
                .show(ui)
                .left;
        if left == Some(crate::widgets::FilterLeft::Down) {
            down.set(true);
        }
    };
    let side = Cell::new(tile_side(app));
    let moved = Cell::new(None);
    let slider = |ui: &mut egui::Ui| {
        ui.spacing_mut().slider_width = SLIDER_WIDTH;
        let range = MIN_TILE..=MAX_TILE;
        if let Some(s) = crate::widgets::picture_size_slider(ui, side.get(), range, None) {
            moved.set(Some(s));
        }
    };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let color = ui.visuals().weak_text_color();
    let count_width =
        ui.fonts_mut(|f| f.layout_no_wrap(count.to_string(), font, color)).size().x.ceil() + 2.0;
    let words = |ui: &mut egui::Ui| {
        ui.add(egui::Label::new(egui::RichText::new(count).color(color)).truncate());
    };
    EditorHeader::new(HEADER)
        .left(Item::custom(KIND, switch_width, switch).more(switch_width, switch).primary())
        .left(Item::custom("Find", FIND_WIDTH, find).more(FIND_WIDTH, find).primary())
        .right(Item::custom(SIZE, SLIDER_WIDTH, slider).more(SLIDER_WIDTH, slider))
        .right(Item::separator())
        .right(Item::custom(COUNT, count_width, words).more(count_width, words))
        .show(ui);
    state.filter = filter.into_inner();
    state.down |= down.get();
    if let Some(s) = moved.get() {
        app.settings.gallery_tile = Some(s);
    }
    if kind.get() != state.kind {
        // (Still in front: the switch keeps the keys it was given.)
        *state = Gallery { kind: kind.get(), shown: state.shown, ..Default::default() };
    }
}

/// The header's name, as a screen reader and the tests find it.
pub const HEADER: &str = "Appearance Gallery header";
/// The kind's segmented control's name.
const KIND: &str = "Kind";
/// The size slider's and the count's names in the header's record.
const SIZE: &str = "Picture size";
const COUNT: &str = "Count";
/// Find's width and the slider's (the board's 180 and 120).
const FIND_WIDTH: f32 = 180.0;
const SLIDER_WIDTH: f32 = 120.0;
/// The hint strip's height (the board's `.modebar`).
const STRIP_HEIGHT: f32 = 26.0;

/// The hint strip (the board's `.modebar`, 26 high): what a click does
/// as the gallery is, and Use the Selection where it was opened for one
/// object. Whether Use the Selection was clicked.
fn hint_strip(ui: &mut egui::Ui, kind: Kind, mode: Mode, words: (String, usize)) -> bool {
    let t = mg_uikit::tokens(ui.ctx());
    let (rect, _) = ui
        .allocate_exact_size(egui::vec2(ui.available_width(), STRIP_HEIGHT), egui::Sense::hover());
    ui.painter().hline(rect.x_range(), rect.bottom() - 0.5, egui::Stroke::new(1.0, t.line_soft));
    let inner = rect.shrink2(egui::vec2(8.0, 0.0));
    let mut strip = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    strip.set_clip_rect(inner.intersect(ui.clip_rect()));
    let (what, n) = words;
    let them = if n == 1 { "it" } else { "them" };
    let mut used = false;
    // (Its words cut with an ellipsis where the tab is narrow, all of them
    // in the tooltip: never past the tab's edge. Use the Selection keeps
    // its room after them.)
    let say = |ui: &mut egui::Ui, words: String, weak: bool, keep: f32| {
        let text = egui::RichText::new(words);
        let text = if weak { text.color(ui.visuals().weak_text_color()) } else { text };
        let room = (ui.available_width() - keep).max(0.0);
        ui.scope(|ui| {
            ui.set_max_width(room);
            ui.add(egui::Label::new(text).truncate());
        });
    };
    match mode {
        Mode::Target => {
            let button = "Use the Selection";
            let font = egui::TextStyle::Button.resolve(strip.style());
            let wide =
                strip.fonts_mut(|f| f.layout_no_wrap(button.into(), font, egui::Color32::WHITE));
            let keep = wide.size().x
                + 2.0 * strip.spacing().button_padding.x
                + strip.spacing().item_spacing.x;
            say(&mut strip, format!("A click gives {what} the appearance"), false, keep);
            used = strip
                .small_button(button)
                .on_hover_text("Give appearances to the objects selected in the area instead")
                .clicked();
        }
        Mode::PlaceGive => {
            let words = format!(
                "A click takes a placeable to place. Right click gives it to the {} selected.",
                kind.noun(n)
            );
            say(&mut strip, words, false, 0.0);
        }
        Mode::Selection => {
            let words = format!(
                "{} selected in the area. A click gives {them} the appearance.",
                kind.noun(n)
            );
            say(&mut strip, words, false, 0.0);
        }
        Mode::Place => {
            let words = "A click takes a placeable to place in an area. A double click makes a blueprint of it.";
            say(&mut strip, words.into(), true, 0.0);
        }
        Mode::Look => {
            let words = format!(
                "Select {} in an area, then click a picture to give them its appearance.",
                kind.name().to_lowercase()
            );
            say(&mut strip, words, true, 0.0);
        }
    }
    used
}

/// The gallery's tab: to look through, and to give an appearance with a
/// click: to the object whose Properties opened it (while it is there),
/// else to the objects of its kind selected in the area.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui) {
    let mut state = app.placeable_gallery.take().unwrap_or_default();
    let ctx = ui.ctx().clone();
    let rows = Rows::of(app, &ctx, state.kind, &state.filter);
    let count = rows.as_ref().map_or(String::new(), |r| count_words(r, true));
    header(app, ui, &mut state, &count);
    // (The kind or Find changed in the header: the rows as they are now.)
    let rows = Rows::of(app, &ctx, state.kind, &state.filter);
    let kind = state.kind;
    // (Down out of Find: the grid has the keys, its cursor on the first
    // picture shown.)
    let found = rows.as_ref().map_or(0, |r| r.found.len());
    if std::mem::take(&mut state.down) && found > 0 {
        (state.entry, state.top) = (Some((0, found)), true);
        ctx.memory_mut(|m| m.request_focus(grid_id(kind)));
    }
    // The one object it was opened for, as it is now.
    let one = state.target.clone().and_then(|(key, path)| {
        let Moonglow { ws: Some(ws), thumbnails, .. } = app else { return None };
        let revision = Some(ws.revision());
        let doc = ws.doc(&key).ok()?;
        let of = path.get(&doc.root)?;
        let now = of.integer(kind.field());
        let dressed = (kind == Kind::Creature)
            .then(|| dressed_kept(thumbnails, revision, &ctx, (key, path.clone()), of));
        Some((key, path, now, dressed))
    });
    if one.is_none() {
        state.target = None;
    }
    // (Opened on its own, for placeables: a click takes one to place.)
    let places = one.is_none() && kind == Kind::Placeable;
    let chosen = if one.is_none() { selected(app, &ctx, kind) } else { None };
    let (current, dressed) = match (&one, &chosen) {
        (Some((_, _, now, dressed)), _) => (*now, *dressed),
        (None, Some((_, _, now, dressed, _))) => (*now, *dressed),
        _ => (None, None),
    };
    // (Nothing to give where every object it is given to has it: the
    // first's alone was looked at, and a click on its appearance did
    // nothing for the others selected with it.)
    let alike = chosen.as_ref().is_none_or(|c| c.4);
    let mode = match (&one, &chosen) {
        (Some(_), _) => Mode::Target,
        (None, Some(_)) if places => Mode::PlaceGive,
        (None, Some(_)) => Mode::Selection,
        _ if places => Mode::Place,
        _ => Mode::Look,
    };
    let words = match (&one, &chosen) {
        (Some((key, path, ..)), _) => {
            let what = if path.0.is_empty() { key.to_string() } else { "the placed object".into() };
            (what, 1)
        }
        (None, Some((_, list, ..))) => (String::new(), list.len()),
        _ => (String::new(), 0),
    };
    if hint_strip(ui, kind, mode, words) {
        state.target = None;
    }
    let taking = app.palette.placing_appearance.as_ref().map(|a| a.row as i64);
    let clicked = match &rows {
        Some(rows) => {
            grid(app, ui, &mut state, rows, (current, taking), dressed, places && chosen.is_some())
        }
        None => Clicked::default(),
    };
    // What the panels show: the picture clicked or the keys moved to last,
    // else the one it opened at.
    if let Some(row) = clicked.click.or(clicked.double).or(clicked.give).or(clicked.moved) {
        state.look = Some(row);
    }
    // (Come to the front of its pane, opened or its tab chosen: its grid
    // takes the keys, so the ring is the keys' accent as the board draws
    // it. Only as it comes, once its pictures are listed: Find typed in,
    // another tab or a floating window keeps them after. A pass missed is
    // the tab not drawn, behind another or folded.)
    let pass = ctx.cumulative_pass_nr();
    let (last, mut wants) = state.shown;
    wants |= last + 1 < pass;
    if wants && rows.as_ref().is_some_and(|r| !r.found.is_empty()) {
        ctx.memory_mut(|m| m.request_focus(grid_id(kind)));
        state.entry = None;
        wants = false;
    }
    state.shown = (pass, wants);
    state.drawn = Drawn {
        row: state.look.or(current).or(taking),
        mode,
        count: rows.as_ref().map_or(String::new(), |r| count_words(r, false)),
        grid: Some(grid_id(kind)),
    };
    // A double click: a blueprint of it, by its kind's wizard, its
    // Properties opened after (as a palette's blueprint opens them).
    if let Some(row) = clicked.double {
        start_wizard(app, kind, row);
    }
    // Opened on its own, a placeable's click takes it to place (again:
    // put down).
    if places && let Some(row) = clicked.click {
        let name = rows
            .as_ref()
            .and_then(|r| r.all.iter().find(|c| c.row as i64 == row).map(|c| c.text.clone()))
            .unwrap_or_else(|| row.to_string());
        app.palette.placing_appearance = match taking == Some(row) {
            true => None,
            false => Some(DraggedAppearance { row: row as usize, name }),
        };
    }
    let pick = match places {
        true => clicked.give,
        false => clicked.click.or(clicked.give),
    }
    .filter(|row| Some(*row) != current || !alike);
    let set = |key: ResKey, path: GffPath, row: i64| -> Vec<Edit> {
        kind.fields(row)
            .into_iter()
            .map(|(label, value)| Edit::SetField {
                key,
                path: path.clone(),
                label: label.into(),
                value: Some(value),
            })
            .collect()
    };
    let mut edits: Vec<Edit> = match (pick, one, chosen) {
        (Some(row), Some((key, path, ..)), _) => set(key, path, row),
        (Some(row), None, Some((git, list, ..))) => list
            .iter()
            .flat_map(|&i| set(git, GffPath::root().item(kind.object().list(), i), row))
            .collect(),
        _ => Vec::new(),
    };
    if !edits.is_empty() {
        crate::blueprint::with_a_body(app, &mut edits);
        crate::blueprint::with_its_portrait(app, &mut edits);
        app.actions.push(Action::Apply(Command::new("Appearance", edits)));
    }
    app.placeable_gallery = Some(state);
}

/// A blueprint of appearance `row` of `kind`, made by its kind's wizard
/// (the Placeable or Door Wizard, the Creature Wizard), named for it, its
/// Properties opened after.
fn start_wizard(app: &mut Moonglow, kind: Kind, row: i64) {
    use mg_module::palette::BlueprintKind;
    if kind == Kind::Creature {
        let appearance = row as u16;
        let w = crate::creature_wizard::CreatureWizard { appearance, ..Default::default() };
        app.creature_wizard = Some(w);
        return;
    }
    let name = (app.game.as_deref())
        .and_then(|g| g.choices(kind.table().0, kind.table().1).ok())
        .and_then(|all| all.into_iter().find(|c| c.row as i64 == row).map(|c| c.text))
        .unwrap_or_default();
    let of = if kind == Kind::Door { BlueprintKind::Door } else { BlueprintKind::Placeable };
    let w = crate::blueprint_wizard::BlueprintWizard::new(of).given(kind.fields(row), &name);
    app.blueprint_wizard = Some(w);
}

/// The gallery's state where `editor` is its tab.
fn of_editor<'a>(app: &'a Moonglow, editor: Option<&crate::Tab>) -> Option<&'a Gallery> {
    matches!(editor, Some(crate::Tab::PlaceableGallery))
        .then_some(app.placeable_gallery.as_ref())
        .flatten()
}

/// Whether `editor` is the Appearance Gallery's tab (whose Preview and
/// Appearance panels are its own).
pub(crate) fn is_gallery(editor: Option<&crate::Tab>) -> bool {
    matches!(editor, Some(crate::Tab::PlaceableGallery))
}

/// The row the gallery's panels show, of its kind, as it was last drawn.
fn shown_row(app: &Moonglow, editor: Option<&crate::Tab>) -> Option<(Kind, usize)> {
    let g = of_editor(app, editor)?;
    let row = usize::try_from(g.drawn.row?).ok()?;
    Some((g.kind, row))
}

/// What the Preview shows of an appearance: an object of `restype` (a
/// placeable's, creature's or door's blueprint type) given row `row`, as
/// the gallery's picture has it (a creature a plain body).
pub(crate) fn object_of(restype: ResType, row: u32) -> Option<(ResType, mg_gff::Struct)> {
    let kind = Kind::of_restype(restype)?;
    let object = match kind {
        Kind::Creature => mg_preview::CreatureLook::new(u16::try_from(row).ok()?).to_utc(),
        Kind::Placeable | Kind::Door => {
            let mut object = mg_gff::Struct::default();
            for (label, value) in kind.fields(i64::from(row)) {
                object.set(label, value);
            }
            object
        }
    };
    Some((restype, object))
}

/// A quiet line in a panel with nothing to show.
fn say(ui: &mut egui::Ui, words: &str) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.weak(words);
    });
}

/// What the gallery's panels say with no appearance chosen.
const NOTHING_CHOSEN: &str = "Click a picture to see its appearance.";

/// The gallery's Preview (the board's Right Dock, over Appearance): the
/// row chosen as an object of the kind, in the model view, paused as it
/// is first made (its Play turns it; at rest it asks for no frame).
pub(crate) fn preview(app: &mut Moonglow, ui: &mut egui::Ui, editor: Option<&crate::Tab>) {
    let t = mg_uikit::tokens(ui.ctx());
    ui.painter().rect_filled(ui.max_rect(), 0.0, t.base);
    let Some((kind, row)) = shown_row(app, editor) else {
        say(ui, NOTHING_CHOSEN);
        return;
    };
    let source = crate::model_view::Source::Appearance { restype: kind.restype(), row: row as u32 };
    // (Its views of the rows chosen before go: release their GPU targets
    // first so free_texture and renderer caches do not leak while browsing.)
    let drop_keys: Vec<_> = app
        .model_views
        .keys()
        .filter(|s| matches!(s, crate::model_view::Source::Appearance { .. }) && **s != source)
        .cloned()
        .collect();
    for k in drop_keys {
        if let Some(mut view) = app.model_views.remove(&k)
            && let Some(vp) = app.viewport.as_mut()
        {
            view.let_go_of_targets(vp);
        }
    }
    let shown =
        crate::model_view::Shown { info: false, pop_out: false, restore: false, paused: true };
    crate::model_view::panel(app, ui, source, shown);
}

/// The Preview's and the Appearance panel's details (their headers'
/// second words, the board's "c_aribeth" and "appearance.2da").
pub(crate) fn panel_detail(
    app: &Moonglow,
    editor: Option<&crate::Tab>,
    panel: crate::shell::Panel,
) -> Option<String> {
    let g = of_editor(app, editor)?;
    let table = g.kind.table().0;
    match panel {
        crate::shell::Panel::Appearance => Some(format!("{table}.2da")),
        _ => {
            let (kind, row) = shown_row(app, editor)?;
            let t = app.game.as_deref()?.table(table).ok()?;
            t.get(row, kind.model_column()).filter(|m| *m != "****").map(str::to_lowercase)
        }
    }
}

/// The Appearance panel (the board's, under the Preview): the facts of
/// the row chosen in its table, a row each: for a creature its row, name,
/// label, model, model type, size, movement rate, personal space, height
/// and blood color; for a placeable or a door its row, name, label, model
/// and sound, and a placeable's shadow and whether it is static, a door's
/// whether it blocks sight.
pub(crate) fn facts(app: &mut Moonglow, ui: &mut egui::Ui, editor: Option<&crate::Tab>) {
    let Some((kind, row)) = shown_row(app, editor) else {
        say(ui, NOTHING_CHOSEN);
        return;
    };
    let Some(game) = app.game.clone() else { return };
    // (Read once for the row and the table as the game data keeps it:
    // its name was looked for through every choice each frame.)
    let Some(list) = game.choice_list(kind.table().0, kind.table().1).ok() else { return };
    let id = egui::Id::new("appearance-gallery-facts");
    let rows = crate::blueprint::kept_for(ui.ctx(), id, &list, &(kind, row), || {
        ui.ctx().data_mut(|d| *d.get_temp_mut_or_default::<usize>(facts_read_id()) += 1);
        facts_of(&game, kind, row, &list)
    });
    let Some(rows) = rows else {
        say(ui, "The table has no such row.");
        return;
    };
    let t = mg_uikit::tokens(ui.ctx());
    egui::ScrollArea::vertical().id_salt("appearance-facts").auto_shrink([false, false]).show(
        ui,
        |ui| {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 6)).show(ui, |ui| {
                // (Rows FACT_PITCH apart, no spacing between: the board's 28.)
                ui.spacing_mut().item_spacing.y = 0.0;
                for &(label, ref value, mono) in rows.iter() {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), FACT_PITCH),
                        egui::Sense::hover(),
                    );
                    let label_rect = rect.with_max_x(rect.left() + FACT_LABEL);
                    let value_rect = rect.with_min_x(rect.left() + FACT_LABEL);
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(label_rect)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    crate::widgets::field_label(&mut child, label);
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(value_rect)
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    let text = egui::RichText::new(value.as_str()).color(t.text_2);
                    let text = if mono { text.monospace() } else { text };
                    child.add(egui::Label::new(text).truncate());
                }
            });
        },
    );
}

/// Where the count of the Appearance panel's reads is kept (the window's
/// own, so other tests' windows don't add to it).
fn facts_read_id() -> egui::Id {
    egui::Id::new("appearance-gallery-facts-read")
}

/// How many times the Appearance panel in `ctx`'s window has read a row's
/// facts (for the tests: once for a row shown, not each frame).
#[doc(hidden)]
pub fn facts_read(ctx: &egui::Context) -> usize {
    ctx.data(|d| d.get_temp(facts_read_id())).unwrap_or(0)
}

/// The Appearance panel's rows' pitch and its labels' column (the
/// board's 28 and 124).
const FACT_PITCH: f32 = 28.0;
const FACT_LABEL: f32 = 124.0;

/// A row's facts as the Appearance panel shows them: label, value, and
/// whether the value is a name in the table (in Hack).
fn facts_of(
    game: &mg_rules::GameData,
    kind: Kind,
    row: usize,
    list: &[mg_rules::Choice],
) -> Option<Vec<(&'static str, String, bool)>> {
    let (name, cols) = kind.table();
    let t = game.table(name).ok()?;
    if row >= t.len() {
        return None;
    }
    let cell = |c: &str| t.get(row, c).filter(|v| *v != "****").map(str::to_string);
    let named = list.iter().find(|c| c.row == row).map(|c| c.text.clone()).unwrap_or_default();
    let mut out = vec![("Row", row.to_string(), false), ("Name", named, false)];
    let label = cols.label.and_then(cell).unwrap_or_default();
    out.push(("Label", label, true));
    out.push(("Model", cell(kind.model_column()).unwrap_or_default().to_lowercase(), true));
    let yes = |v: Option<String>| match v.as_deref() {
        Some("1") => "Yes".to_string(),
        Some(_) => "No".to_string(),
        None => String::new(),
    };
    let sound = |c: &str| {
        let n = t.get_int(row, c)?;
        let s = game.table("placeableobjsnds").ok()?;
        let label = s.get(usize::try_from(n).ok()?, "LABEL")?.to_string();
        Some(format!("{label} ({n})"))
    };
    match kind {
        Kind::Creature => {
            let model_type = cell("MODELTYPE").unwrap_or_default();
            let model_words = match model_type.as_str() {
                "F" => "Full model",
                "P" => "Part-based",
                "S" => "Simple",
                "L" => "Large",
                _ => "",
            };
            let model_type = match model_words {
                "" => model_type,
                w => format!("{w} ({model_type})"),
            };
            out.push(("Model type", model_type, false));
            let size = t.get_int(row, "SIZECATEGORY");
            let size_words = match size {
                Some(1) => "Tiny".to_string(),
                Some(2) => "Small".to_string(),
                Some(3) => "Medium".to_string(),
                Some(4) => "Large".to_string(),
                Some(5) => "Huge".to_string(),
                Some(n) => n.to_string(),
                None => String::new(),
            };
            out.push(("Size", size_words, false));
            let rate = cell("MOVERATE").unwrap_or_default();
            let rate_words = match rate.as_str() {
                "NORM" => "Normal",
                "FAST" => "Fast",
                "SLOW" => "Slow",
                "VSLOW" => "Very slow",
                "VFAST" => "Very fast",
                "DFAST" => "DM fast",
                "PC" => "Player's",
                "DEFAULT" => "Default",
                "" => "",
                other => other,
            };
            let num = |c: &str| t.get_float(row, c).map(|v| format!("{v:.2}"));
            let moves = match (num("WALKDIST"), num("RUNDIST")) {
                (Some(w), Some(r)) => format!("{rate_words} (walk {w}, run {r})"),
                _ => rate_words.to_string(),
            };
            out.push(("Movement rate", moves, false));
            out.push(("Personal space", cell("PERSPACE").unwrap_or_default(), false));
            out.push(("Height", cell("HEIGHT").unwrap_or_default(), false));
            let blood = cell("BLOODCOLR").unwrap_or_default();
            let blood_words = match blood.as_str() {
                "R" => "Red",
                "W" => "White",
                "G" => "Green",
                "Y" => "Yellow",
                "N" => "None",
                _ => "",
            };
            let blood = match blood_words {
                "" => blood,
                w => format!("{w} ({blood})"),
            };
            out.push(("Blood color", blood, false));
        }
        Kind::Placeable => {
            out.push(("Sound", sound("SoundAppType").unwrap_or_default(), false));
            out.push(("Shadow size", cell("ShadowSize").unwrap_or_default(), false));
            out.push(("Static", yes(cell("Static")), false));
        }
        Kind::Door => {
            out.push(("Sound", sound("SoundAppType").unwrap_or_default(), false));
            out.push(("Blocks sight", yes(cell("BlockSight")), false));
        }
    }
    Some(out)
}

/// What the Status Bar shows beside the gallery (the AppearanceGallery
/// board's): what a click does as it is (Click Give the appearance with
/// an object to give it to; Click Take to place for placeables opened on
/// their own, and Right Click Menu with some selected, whose Give to the
/// Selection gives it; no
/// Click where it does nothing), Drag Place in an area for placeables,
/// Double Click and the wizard it starts, the grid's arrows while it has
/// the keys; none while a modal, a dialog, a menu or a popover is in
/// front. Its count at the right, the header's words: "838 appearances",
/// or "12 of 838 appearances" under Find.
pub(crate) fn shown(app: &Moonglow, ctx: &egui::Context) -> crate::shell::Shown {
    use crate::shell::{Count, Shown};
    use mg_uikit::status::Hint;
    let Some(g) = app.placeable_gallery.as_ref() else { return Shown::default() };
    // (The header's count, from the one place: "12 of 838 appearances"
    // under Find.)
    let counts = vec![Count::new(g.drawn.count.clone())];
    let mut hints = Vec::new();
    if crate::shell::keys_blocked(ctx)
        || mg_uikit::popover::Popover::any_open(ctx)
        || egui::Popup::is_any_open(ctx)
    {
        return Shown { hints, counts, note: None };
    }
    if g.drawn.grid.is_some_and(|id| ctx.memory(|m| m.has_focus(id))) {
        hints.push(Hint::either(["Up", "Down"], "Row of pictures"));
        hints.push(Hint::either(["Left", "Right"], "One picture"));
    }
    match g.drawn.mode {
        Mode::Target | Mode::Selection => hints.push(Hint::keys(["Click"], "Give the appearance")),
        Mode::Place => hints.push(Hint::keys(["Click"], "Take to place")),
        Mode::PlaceGive => {
            hints.push(Hint::keys(["Click"], "Take to place"));
            // (A right click opens the picture's menu, whose Give to the
            // Selection gives it.)
            hints.push(Hint::keys(["Right Click"], "Menu"));
        }
        Mode::Look => {}
    }
    if g.kind == Kind::Placeable {
        hints.push(Hint::keys(["Drag"], "Place in an area"));
    }
    hints.push(Hint::keys(["Double Click"], format!("{} with it", g.kind.wizard())));
    Shown { hints, counts, note: None }
}
