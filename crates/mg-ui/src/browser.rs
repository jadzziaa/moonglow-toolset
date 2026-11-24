//! The resource browser: every resource in the load order (game, haks,
//! module) with the layer it comes from, and a read-only view of any of
//! them that can be copied into the module or saved to a file. The
//! browser is the Left Dock's Resources panel (`shell::Panel::Resources`,
//! the redesign's M3 T4): its header's buttons ([`tools`]) and the panel
//! under them ([`ui`]); the read-only view opens in the editor area.

use mg_uikit::Controls as _;
use std::path::PathBuf;

use egui::Ui;
use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_gff::{Gff, Value};
use mg_resman::ResKey;

use crate::text::decode;
use crate::{Action, FileKind, Moonglow, Tab};

/// How far an overridden copy's row is set in under its resource's.
const COPY_INDENT: f32 = 18.0;
/// The Name field's name for a screen reader (and tests): told from the
/// module tree's Filter.
pub(crate) const FILTER: &str = "Filter Resources";
/// A resource's bytes from the load order: the copy the game reads, or
/// (`layer`, by its label) the copy that layer has.
fn read_copy<'a>(
    rm: &'a mg_resman::ResMan,
    key: &ResKey,
    layer: Option<&str>,
) -> Option<std::borrow::Cow<'a, [u8]>> {
    match layer {
        None => rm.get(key).ok(),
        Some(label) => {
            let i = rm.layers().iter().position(|l| l.label == label)?;
            rm.get_in(i, key).ok()
        }
    }
}

/// A row of the browser's list: a copy of a resource and its layer's
/// label; for a copy that a higher layer's overrides, that layer's.
#[derive(Debug, Clone)]
struct Row {
    key: ResKey,
    layer: std::sync::Arc<str>,
    over: Option<std::sync::Arc<str>>,
}

/// The browser's index and filters.
#[derive(Debug, Default)]
pub struct Browser {
    /// Every copy of each resource, the one the game reads first and those
    /// it overrides under it; rebuilt when stale.
    entries: Vec<Row>,
    /// How many resources there are (the copies the game reads).
    resources: usize,
    types: std::sync::Arc<Vec<ResType>>,
    pub restypes: std::collections::BTreeSet<ResType>,
    pub filter: String,
    /// The copies that others override are listed under them (a builder's
    /// report: of a table in two haks, only the higher one was found).
    pub show_copies: bool,
    /// Indices into `entries` matching the filters, for `query`.
    shown: Vec<usize>,
    /// How many of those are overridden copies.
    shown_copies: usize,
    /// Whether any matching entry in `shown` is an MDL model.
    shown_has_mdl: bool,
    query: Option<(std::collections::BTreeSet<ResType>, String, bool)>,
    /// The load order changed since the index was built.
    pub(crate) stale: bool,
    /// Export as Files writes compiled models as text.
    pub models_as_text: bool,
    /// 2DA views: StrRefs shown as numbers rather than their text.
    pub(crate) raw_strrefs: bool,
    /// 2DA views: only the rows a layer last changed (its index in the
    /// table's copies).
    pub rows_from: Option<usize>,
    /// The row the keys are at, among those listed.
    pub cursor: Option<usize>,
    /// The cursor moved: the list goes to it.
    reveal: bool,
    /// The 2DA views, each table where it is (its key and layer) on the
    /// editor's grid, read-only: its cell selected, its Find, its hidden
    /// columns, kept while it is shown.
    pub(crate) table_views: std::collections::HashMap<(ResKey, Option<String>), TableView>,
    /// How many views have been made (their grids' ids are counted down
    /// from the highest, apart from the editor's).
    made_views: u32,
    /// Monotonic counter for LRU eviction of cached 2DA views.
    table_view_use: u64,
    /// An Export as Files into a folder that has some of the files: asked
    /// about before any is written over.
    pub(crate) exporting: Option<Exporting>,
    /// The read-only tabs whose first bytes are shown in place of their
    /// viewer (a picture, a sound, a model: the header's More's Show Bytes
    /// or Show as Text, M5's W6), by key and layer, while the window runs.
    pub(crate) bytes_shown: std::collections::HashSet<(ResKey, Option<String>)>,
    /// The read-only tabs' pictures kept decoded, by key and layer, with
    /// when each was last drawn: let go of a while after
    /// ([`let_go_of_pictures`]).
    pub(crate) pictures: std::collections::HashMap<(ResKey, Option<String>), f64>,
    /// The read-only text tabs' Find bars and marked lines, by key and
    /// layer ([`TextFind`]), kept while their tabs are open.
    pub(crate) finds: std::collections::HashMap<(ResKey, Option<String>), TextFind>,
}

/// A 2DA's read-only view.
#[derive(Debug)]
pub(crate) struct TableView {
    /// The reading of the table's layers it was made for ([`made`]):
    /// another is made for another.
    of: u64,
    /// The layer whose rows alone it shows (Rows from), as last set.
    rows_from: Option<usize>,
    pub(crate) doc: crate::table_view::TableDoc,
    /// The tab that drew it last (a resource tab, a layer's copy's, a hak
    /// editor's): its grid's keys are that tab's while it is the active
    /// editor (`table_view::grid_has_keys`).
    tab: Option<Tab>,
    last_used: u64,
}

/// How many 2DA views are kept: more are made anew.
const VIEWS_KEPT: usize = 16;

impl Browser {
    pub fn new() -> Browser {
        Browser { stale: true, show_copies: true, ..Default::default() }
    }

    fn refresh(&mut self, game: Option<&mg_rules::GameData>) {
        if !self.stale {
            return;
        }
        let app_resman = game.map(|g| &g.resman);
        self.stale = false;
        self.query = None;
        self.entries = match app_resman {
            Some(rm) => {
                let labels: Vec<std::sync::Arc<str>> =
                    rm.layers().iter().map(|l| std::sync::Arc::from(l.label.as_str())).collect();
                rm.every_copy()
                    .into_iter()
                    .flat_map(|(key, layers)| {
                        let top = labels[layers[0]].clone();
                        let labels = &labels;
                        layers.into_iter().enumerate().map(move |(n, i)| Row {
                            key,
                            layer: labels[i].clone(),
                            over: (n > 0).then(|| top.clone()),
                        })
                    })
                    .collect()
            }
            None => Vec::new(),
        };
        // (The game's own dialog.tlk, which is no resource of the load
        // order: read-only in its tab, M4's U17.)
        // (Whatever the load order holds: a hak's own `dialog.tlk` is a
        // resource of its own, listed and opened as every other.)
        if let Some(g) = game {
            let key = crate::talk_view::dialog_key();
            self.entries.push(Row {
                key,
                layer: std::sync::Arc::from(game_talk_layer(g.language)),
                over: None,
            });
        }
        self.resources = self.entries.iter().filter(|r| r.over.is_none()).count();
        let mut types: Vec<ResType> = self.entries.iter().map(|r| r.key.restype).collect();
        types.sort_by_key(|t| t.extension().unwrap_or_default());
        types.dedup();
        self.types = std::sync::Arc::new(types);
    }

    /// The keys at the row `at` of those listed, the list gone to it.
    pub(crate) fn go_to(&mut self, at: usize) {
        self.cursor = Some(at);
        self.reveal = true;
    }

    /// The types of the resources listed under the filters (for the
    /// tests: that every type checked in the Type list is listed).
    pub fn listed_types(&mut self) -> std::collections::BTreeSet<ResType> {
        let shown = self.filtered().to_vec();
        shown.into_iter().map(|i| self.entries[i].key.restype).collect()
    }

    fn filtered(&mut self) -> &[usize] {
        let needle = mg_core::text::Needle::new(&self.filter);
        let cache_hit = self.query.as_ref().is_some_and(|(types, q, copies)| {
            *copies == self.show_copies && q == needle.as_str() && *types == self.restypes
        });
        if !cache_hit {
            let check_name = !needle.is_empty();
            let mut shown = Vec::new();
            let mut shown_copies = 0usize;
            let mut has_mdl = false;
            for (i, r) in self.entries.iter().enumerate() {
                if !self.show_copies && r.over.is_some() {
                    continue;
                }
                if !self.restypes.is_empty() && !self.restypes.contains(&r.key.restype) {
                    continue;
                }
                if check_name {
                    let matches = match r.key.resref.as_str() {
                        Some(s) => needle.finds(s),
                        None => needle.finds(&r.key.resref.to_string_lossy()),
                    };
                    if !matches {
                        continue;
                    }
                }
                if r.over.is_some() {
                    shown_copies += 1;
                }
                if r.key.restype == ResType::MDL {
                    has_mdl = true;
                }
                shown.push(i);
            }
            self.shown = shown;
            self.shown_copies = shown_copies;
            self.shown_has_mdl = has_mdl;
            self.query =
                Some((self.restypes.clone(), needle.as_str().to_string(), self.show_copies));
            // (Another list: the keys start at its top.)
            self.cursor = self.cursor.map(|_| 0);
            self.reveal = self.cursor.is_some();
        }
        &self.shown
    }
}

/// What the Resources panel's header (its tab bar's tools, the board's
/// `.panel-tabs .tools`) needs of the browser, read before the bar is
/// drawn: how many resources are listed (the copies the game reads),
/// whether that is fewer than all (Export as Files is offered only then:
/// not the whole game by a slip), and whether the game data is there.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Header {
    found: usize,
    narrowed: bool,
    game: bool,
}

/// What the panel's header buttons asked.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Asked {
    export: bool,
    refresh: bool,
}

/// The index made where stale and the filters applied: the header's
/// [`Header`].
pub(crate) fn header(app: &mut Moonglow) -> Header {
    let Moonglow { browser, game, .. } = app;
    browser.refresh(game.as_deref());
    let count = browser.filtered().len();
    let found = count - browser.shown_copies;
    Header { found, narrowed: found > 0 && found < browser.resources, game: game.is_some() }
}

/// Export's tooltip, which says how many it writes ([`tools`]):
/// enabled, what it does with them; disabled, what to do first.
fn export_tip(header: &Header) -> String {
    match header.found {
        0 => "Nothing is listed to export".into(),
        _ if !header.narrowed => "Narrow the list to export it".into(),
        1 => "Write the 1 listed resource into a folder as a file".into(),
        n => format!("Write the {n} listed resources into a folder as files"),
    }
}

/// The header's buttons, in the board's order: Export N as Files… (named
/// with how many it writes, its tip too; enabled with the list narrowed)
/// and Refresh.
pub(crate) fn tools(ui: &mut Ui, header: &Header) -> Asked {
    use crate::widgets::icon_only;
    let name = format!("Export {} as Files…", header.found);
    let tip = export_tip(header);
    let r = ui.add_enabled(header.narrowed, icon_only(mg_uikit::Icon::Export, &name));
    let export = r.on_hover_text(&tip).on_disabled_hover_text(&tip).clicked();
    let tip = "Read the load order again";
    let r = ui.add_enabled(header.game, icon_only(mg_uikit::Icon::Update, "Refresh"));
    let refresh = r.on_hover_text(tip).on_disabled_hover_text(tip).clicked();
    Asked { export, refresh }
}

/// Does what the header's buttons asked.
pub(crate) fn run_tools(app: &mut Moonglow, asked: Asked) {
    let browser = &mut app.browser;
    if asked.refresh {
        browser.stale = true;
    }
    if asked.export {
        // (Not the game's dialog.tlk, no resource of the load order: its
        // tab's Export writes it.)
        let keys = (browser.shown.iter().map(|&i| &browser.entries[i]))
            .filter(|r| !is_game_talk(r))
            .map(|r| (r.key, r.over.as_ref().map(|_| r.layer.to_string())))
            .collect();
        app.actions.push(Action::SaveResources(keys));
    }
}

fn type_summary(types: &std::collections::BTreeSet<ResType>) -> String {
    if types.is_empty() {
        return "all types".to_string();
    }
    let mut exts: Vec<&str> = types.iter().map(|t| t.extension().unwrap_or("?")).collect();
    exts.sort_unstable();
    match exts.len() {
        1 => exts[0].to_string(),
        2 | 3 => exts.join(", "),
        n => format!("{n} types ({}, {}…)", exts[0], exts[1]),
    }
}

/// A resource's kind as its row's icon shows it.
pub(crate) fn kind_icon(t: ResType) -> mg_uikit::Icon {
    use mg_uikit::Icon;
    if let Some(kind) = mg_area::ObjectKind::from_restype(t) {
        return crate::icons::object(kind);
    }
    match t {
        ResType::NSS | ResType::NCS | ResType::NDB => Icon::Script,
        ResType::ARE | ResType::GIT | ResType::GIC => Icon::Area,
        ResType::DLG => Icon::Conversation,
        ResType::MDL | ResType::WOK | ResType::PWK | ResType::DWK => Icon::Model,
        ResType::TGA | ResType::DDS | ResType::PLT | ResType::TXI | ResType::MTR => Icon::Texture,
        ResType::TWODA => Icon::Table,
        ResType::WAV | ResType::BMU => Icon::Sound,
        ResType::JUI => Icon::Nui,
        ResType::JRL => Icon::Journal,
        ResType::TLK => Icon::TalkTable,
        ResType::SET => Icon::Tileset,
        ResType::ITP => Icon::Palette,
        ResType::IFO => Icon::Module,
        ResType::FAC => Icon::Faction,
        _ => Icon::File,
    }
}

/// The room at a row's left before its icon and at its right after its
/// layer (`.tree-row`: `padding: 0 6px 0 4px`), and between its parts
/// (`gap: 4px`; the layer's `padding-left: 4px` more).
const ROW_LEFT: f32 = 4.0;
const ROW_RIGHT: f32 = 6.0;
const ROW_GAP: f32 = 4.0;
/// The share of a row's words the layer takes at most where both do not
/// fit: the name is what is looked for, and comes first.
const LAYER_SHARE: f32 = 0.45;
/// A row's name's size (the board's `.res .tree-row .name.mono`).
const NAME_SIZE: f32 = 12.0;
/// The least of a layer's words worth showing, cut with an ellipsis
/// ([`share`]): less than this, and the row shows its name alone (the
/// row's tip has both).
const LAYER_STUB: &str = "key:n…";

/// The widths at most of a row's name and layer, `name` and `layer` wide
/// as they are, in `room` (the room between them taken out): both whole
/// where they fit. Else the name comes first: the layer takes at most
/// what the whole name leaves, and never more than [`LAYER_SHARE`] of
/// the room, cut with an ellipsis; where that is less than `stub` (a
/// readable stub of a layer, [`LAYER_STUB`]) and less than the whole
/// layer, the layer is not shown (`None`) and the name has the room.
fn share(room: f32, name: f32, layer: f32, stub: f32) -> (f32, Option<f32>) {
    let room = room.max(0.0);
    if name + layer <= room {
        return (name, Some(layer));
    }
    let left = (room * LAYER_SHARE).min(room - name);
    if left < layer.min(stub) {
        return (room, None);
    }
    let layer = layer.min(left);
    (room - layer, Some(layer))
}

/// `text` (a resource's name, `name.ext`) cut in its middle to `keep`
/// characters and an ellipsis between them, its end and extension kept
/// (`nw_ch_s…off.ncs`): resources' names differ at their ends as often
/// as at their starts. `None` where it has no more than `keep`
/// characters, or where `keep` leaves no room for the extension and a
/// character before it (then it is cut at its end).
fn middle_cut(text: &str, keep: usize) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= keep {
        return None;
    }
    let ext = chars.iter().rposition(|c| *c == '.').map_or(0, |at| chars.len() - at);
    if keep < ext + 2 {
        return None;
    }
    let tail = (keep / 2).max(ext).min(keep - 1);
    let head = keep - tail;
    let mut cut: String = chars[..head].iter().collect();
    cut.push('…');
    cut.extend(&chars[chars.len() - tail..]);
    Some(cut)
}

/// What every row of the list lays out its words with, measured once for
/// the rows drawn ([`list_row`]): the name's monospace font, the width of
/// one of its characters and of its ellipsis, the layer's small font, and
/// the width of a layer's stub ([`LAYER_STUB`]).
#[derive(Clone)]
struct RowFonts {
    mono: egui::FontId,
    small: egui::FontId,
    char_width: f32,
    ellipsis: f32,
    stub: f32,
}

impl RowFonts {
    fn of(ui: &Ui) -> RowFonts {
        let small = egui::TextStyle::Small.resolve(ui.style());
        let key = (small.clone(), ui.ctx().pixels_per_point().to_bits());
        let id = egui::Id::new("browser-row-fonts");
        if let Some((was, cached)) =
            ui.ctx().data(|d| d.get_temp::<((egui::FontId, u32), RowFonts)>(id))
            && was == key
        {
            return cached;
        }
        let mono = egui::FontId::monospace(NAME_SIZE);
        let width =
            |text: &str, font: &egui::FontId| cut(ui, text, font.clone(), f32::INFINITY).size().x;
        // (Ten, for a width that is the font's advance, not a glyph's.)
        let char_width = width("0000000000", &mono) / 10.0;
        let ellipsis = width("0…", &mono) - width("0", &mono);
        let stub = width(LAYER_STUB, &small);
        let fonts = RowFonts { mono, small, char_width, ellipsis, stub };
        ui.ctx().data_mut(|d| d.insert_temp(id, (key, fonts.clone())));
        fonts
    }

    /// `name` laid out in the name's font in `width` at most: whole where
    /// it fits (`natural`, laid out already), else cut in its middle
    /// ([`middle_cut`]), else at its end, with an ellipsis.
    fn name(
        &self,
        ui: &Ui,
        name: &str,
        natural: std::sync::Arc<egui::Galley>,
        width: f32,
    ) -> (std::sync::Arc<egui::Galley>, bool) {
        if natural.size().x <= width {
            return (natural, false);
        }
        if self.char_width > 0.0 {
            let mut keep = ((width - self.ellipsis) / self.char_width).floor().max(0.0) as usize;
            // (Measured as built: a font's ellipsis or kerning may make it
            // wider than counted, then a character less.)
            for _ in 0..3 {
                let Some(text) = middle_cut(name, keep) else { break };
                let galley = cut(ui, &text, self.mono.clone(), f32::INFINITY);
                if galley.size().x <= width {
                    return (galley, true);
                }
                keep = keep.saturating_sub(1);
            }
        }
        (cut(ui, name, self.mono.clone(), width), true)
    }
}

/// Lays `text` out in one line of `font`, cut with an ellipsis to `width`
/// at most (its whole text kept as the galley's: what a screen reader
/// says), in the color the painter gives it ([`Painter::galley`]'s).
///
/// [`Painter::galley`]: egui::Painter::galley
fn cut(ui: &Ui, text: &str, font: egui::FontId, width: f32) -> std::sync::Arc<egui::Galley> {
    cut_in(ui, text, font, width, egui::Color32::PLACEHOLDER)
}

/// [`cut`] in `color`.
fn cut_in(
    ui: &Ui,
    text: &str,
    font: egui::FontId,
    width: f32,
    color: egui::Color32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.into(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(0.0));
    ui.fonts_mut(|f| f.layout_job(job))
}

/// A row of the list (the board's `.tree-row` in `.res-list`), as wide as
/// the list: the kind's icon, the name (monospace) and the layer it comes
/// from right-aligned (weak, small; an overridden copy set in, its name
/// weak, its layer "hak:x, overridden by hak:y"). Where both do not fit,
/// the name comes first ([`share`]): the layer is cut with an ellipsis
/// to what the whole name leaves, or left out where less than a stub of
/// it would be; a name longer than the row is cut in its middle, its
/// extension kept (`nw_ch_s…off.ncs`, [`middle_cut`]). The whole of both
/// is in the row's tooltip then. Two parts side by side, which answer
/// the pointer as one row: up to the layer, a selectable label named by
/// its name, said selected while `marked` (the one the keyboard's Tab
/// reaches); the layer, a label of its own (which file provides the
/// resource), clicked as the row is. The row's response: both parts'.
fn list_row(ui: &mut Ui, row: &Row, marked: bool, fonts: &RowFonts) -> egui::Response {
    let t = mg_uikit::tokens(ui.ctx());
    let name = row.key.to_string();
    let layer: std::borrow::Cow<'_, str> = match &row.over {
        Some(over) => format!("{}, overridden by {over}", row.layer).into(),
        None => std::borrow::Cow::Borrowed(&row.layer),
    };
    let height = mg_uikit::row_height(ui);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover());
    let left = rect.left() + ROW_LEFT + if row.over.is_some() { COPY_INDENT } else { 0.0 };
    let name_x = left + mg_uikit::metrics::ICON + ROW_GAP;
    let end = rect.right() - ROW_RIGHT;
    let gaps = 2.0 * ROW_GAP;
    let room = end - name_x - gaps;
    // (The name in 12 px monospace, the layer small: `.name.mono`, `.src`.)
    let natural_name = cut(ui, &name, fonts.mono.clone(), f32::INFINITY);
    let natural_layer = cut(ui, &layer, fonts.small.clone(), f32::INFINITY);
    let (name_most, layer_most) =
        share(room, natural_name.size().x, natural_layer.size().x, fonts.stub);
    // (Without the layer, the name has the gaps before it too.)
    let name_most = if layer_most.is_some() { name_most } else { end - name_x };
    let (name_galley, name_cut) = fonts.name(ui, &name, natural_name, name_most);
    let layer_galley = layer_most.map(|most| match natural_layer.size().x <= most {
        true => natural_layer,
        false => cut(ui, &layer, fonts.small.clone(), most),
    });
    let layer_cut = layer_galley.as_ref().is_none_or(|g| g.elided);
    // (The layer's part from the gap before its words to the row's end.)
    let layer_left = layer_galley.as_ref().map(|g| end - g.size().x);
    let split = layer_left.map_or(rect.right(), |l| (l - ROW_GAP).max(rect.left() + 1.0));
    let id = ui.id().with(("browser-row", row.key, &*row.layer));
    let named = ui.interact(rect.with_max_x(split), id, egui::Sense::click());
    named.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, marked, &name)
    });
    // (Clicked as the row, not a stop of its own for the keyboard's Tab.)
    let response = if layer_galley.is_some() {
        let said = ui.interact(rect.with_min_x(split), id.with("layer"), egui::Sense::CLICK);
        said.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &layer));
        named | said
    } else {
        named
    };
    if ui.is_rect_visible(rect) {
        let hovered = response.hovered() || response.has_focus();
        let (fill, text, weak, icon) = if marked {
            (t.accent_fill, t.on_fill, t.on_fill.gamma_multiply(0.85), t.on_fill)
        } else {
            let fill = if hovered { t.hover_row } else { egui::Color32::TRANSPARENT };
            let text = if row.over.is_some() { t.text_3 } else { t.text };
            // (Its kind's hue with Options › General › Colored icons; an
            // overridden copy, dimmed, keeps the gray.)
            let icon = match row.over {
                Some(_) => t.text_2,
                None => mg_uikit::hues::tint(ui.ctx(), kind_icon(row.key.restype), t.text_2),
            };
            (fill, text, t.text_3, icon)
        };
        let p = ui.painter();
        if fill != egui::Color32::TRANSPARENT {
            p.rect_filled(rect, 0.0, fill);
        }
        let middle = rect.center().y;
        let side = egui::Vec2::splat(mg_uikit::metrics::ICON);
        let at = egui::pos2(left, middle - side.y / 2.0);
        kind_icon(row.key.restype).paint(p, egui::Rect::from_min_size(at, side), icon);
        let y = |g: &egui::Galley| (middle - g.size().y / 2.0).round();
        p.galley(egui::pos2(name_x, y(&name_galley)), name_galley.clone(), text);
        if let (Some(galley), Some(left)) = (&layer_galley, layer_left) {
            p.galley(egui::pos2(left, y(galley)), galley.clone(), weak);
        }
    }
    if name_cut || layer_cut {
        response.on_hover_text(format!("{name} from {layer}"))
    } else {
        response
    }
}

/// The Resources panel under its header (`.res-bar` over `.res-list`):
/// the Type, the Name filter and Overridden copies (and Models as text,
/// where models are listed), the count, a line, and the list, at the
/// Left Dock's width. `pane` is the panel's whole rectangle, its header
/// with it: Find's keys there go to the Name.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, pane: egui::Rect) {
    use crate::palette_view::{Keys, give_keys, keys_are};
    let Moonglow { browser, game, ws, actions, resources_here, .. } = app;
    browser.refresh(game.as_deref());
    let t = mg_uikit::tokens(ui.ctx());
    if game.is_none() {
        egui::Frame::new().inner_margin(8).show(ui, |ui| {
            ui.add(egui::Label::new("The browser needs the game data (Tools › Options).").wrap());
        });
        return;
    }
    let count = browser.filtered().len();
    let found = count - browser.shown_copies;
    egui::Frame::new().inner_margin(8).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 6.0;
        let width = ui.available_width();
        let types = browser.types.clone();
        crate::widgets::dropdown("browser-type")
            .selected_text(type_summary(&browser.restypes))
            .width(width)
            .filtered(true)
            .stays_open(true)
            .show_menu(ui, |ui, keys| {
                let any_matching_unselected = types.iter().any(|t| {
                    let ext = t.extension().unwrap_or("?");
                    keys.shows(ext) && !browser.restypes.contains(t)
                });
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !browser.restypes.is_empty(),
                            egui::Button::new("Clear All").small(),
                        )
                        .clicked()
                    {
                        browser.restypes.clear();
                    }
                    if ui
                        .add_enabled(
                            any_matching_unselected,
                            egui::Button::new("Select Matching").small(),
                        )
                        .clicked()
                    {
                        for &t in types.iter() {
                            if keys.shows(t.extension().unwrap_or("?")) {
                                browser.restypes.insert(t);
                            }
                        }
                    }
                });
                mg_uikit::menu_separator(ui);
                for &t in types.iter() {
                    let ext = t.extension().unwrap_or("?");
                    let mut on = browser.restypes.contains(&t);
                    if keys.checkbox(ui, &mut on, ext) {
                        if on {
                            browser.restypes.insert(t);
                        } else {
                            browser.restypes.remove(&t);
                        }
                    }
                }
            })
            .on_hover_text("Filter by one or more resource types");
        // The Name's keys, as the side panes' Filters have them (GitHub
        // issue 12): Find's (Ctrl+F) with the pointer over the panel go to
        // it, Escape clears it and then leaves it, Down goes into the list.
        let field_id = egui::Id::new("browser-filter-field");
        let over = ui.ctx().pointer_hover_pos().is_some_and(|p| pane.contains(p));
        // (The Status Bar's hints are the panel's while it is in use: the
        // pointer over it or its list with the keys, `shell::shown`.)
        *resources_here = (over || keys_are(ui.ctx(), Keys::Browser)).then_some(over);
        let left = crate::widgets::SearchField::new(field_id, &mut browser.filter)
            .icon(mg_uikit::Icon::Filter)
            .hint("Name")
            .name(FILTER)
            .clear_tip("Clear the name")
            .clear_name("Clear Name")
            .find_when(over)
            .down()
            .show(ui)
            .left;
        if left == Some(crate::widgets::FilterLeft::Down) {
            browser.cursor = Some(0);
            browser.reveal = true;
            give_keys(ui.ctx(), Keys::Browser);
        }
        ui.check_box(&mut browser.show_copies, "Overridden copies").on_hover_text(
            "Also list each resource's copies lower in the load order, which it overrides",
        );
        if browser.shown_has_mdl {
            ui.check_box(&mut browser.models_as_text, "Models as text").on_hover_text(
                "Export as Files writes compiled models as the text they compile from",
            );
        }
        // (The resources, the copies the game reads; and the copies they
        // override, listed under them. Cut short at the dock's width, the
        // whole in its tip. How to open one is the Status Bar's hints'.)
        debug_assert_eq!(found + browser.shown_copies, browser.shown.len());
        let said = browser.count_text();
        let body = egui::TextStyle::Body.resolve(ui.style());
        let galley = cut_in(ui, &said, body, ui.available_width(), t.text_3);
        let elided = galley.elided;
        let r = ui.add(egui::Label::new(galley).show_tooltip_when_elided(false).selectable(false));
        if elided {
            r.on_hover_text(&said);
        }
    });
    // (The bar's line under it, as the board's `border-bottom`.)
    let (line, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(line, 0.0, t.line);
    // The keys in the list, from a click on a row or Down from the
    // Filter: a row at a time, a page, the ends; Enter opens.
    let mut entered = false;
    if keys_are(ui.ctx(), Keys::Browser) && !ui.ctx().egui_wants_keyboard_input() && count > 0 {
        use egui::Key;
        let steps = [
            (Key::ArrowDown, 1),
            (Key::ArrowUp, -1),
            (Key::PageDown, 10),
            (Key::PageUp, -10),
            (Key::End, isize::MAX / 2),
            (Key::Home, isize::MIN / 2),
        ];
        let pressed = ui.input_mut(|i| {
            steps.into_iter().find(|(k, _)| i.consume_key(egui::Modifiers::NONE, *k))
        });
        if let Some((_, by)) = pressed {
            let last = count as isize - 1;
            let at = browser.cursor.map_or(0, |at| (at as isize).saturating_add(by));
            browser.cursor = Some(at.clamp(0, last) as usize);
            browser.reveal = true;
        }
        entered = browser.cursor.is_some()
            && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter));
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            give_keys(ui.ctx(), Keys::Area);
        }
    }
    browser.cursor = browser.cursor.filter(|c| *c < count);
    // (A row of controls: as high as one, and no gap between rows.)
    let pitch = mg_uikit::row_metrics(ui).1;
    let in_module = |k: &ResKey| ws.as_ref().is_some_and(|w| w.module.contains(k));
    let shown = &browser.shown;
    // (Under an id of its own: the module tree's scroll area is drawn in
    // the same place of the Left Dock, and with egui's default id the two
    // shared one place, each taking the other's when its tab came back.
    // The tree keeps the default id, under which eframe keeps its place.)
    let mut list =
        egui::ScrollArea::vertical().id_salt("resources-list").auto_shrink([false, false]);
    if std::mem::take(&mut browser.reveal)
        && let Some(at) = browser.cursor
    {
        let above = (ui.available_height() / 2.0 - pitch).max(0.0);
        list = list.vertical_scroll_offset((at as f32 * pitch - above).max(0.0));
    }
    let cursor = browser.cursor;
    let mut clicked = None;
    let fonts = RowFonts::of(ui);
    let out = mg_uikit::show_rows(list, ui, count, |ui, range| {
        // (The rows one under another, as wide as the list.)
        ui.spacing_mut().item_spacing.y = 0.0;
        for (at, &i) in range.clone().zip(&shown[range]) {
            let row = &browser.entries[i];
            let Row { key: k, layer, over } = row;
            {
                let at_cursor = cursor == Some(at) && keys_are(ui.ctx(), Keys::Browser);
                // A copy another overrides: under it, set in, its own
                // layer's (opened as that layer has it).
                let r = list_row(ui, row, at_cursor, &fonts);
                let copy_arc = over.as_ref().map(|_| layer);
                let game_talk = is_game_talk(row);
                // (The game's dialog.tlk: shown in the Talk Table tab.)
                let open = || match game_talk {
                    true => Action::ShowGameTalk,
                    false => {
                        Action::OpenTab(tab_of(*k, copy_arc.map(|l| l.to_string()), in_module(k)))
                    }
                };
                if r.clicked() {
                    clicked = Some(at);
                }
                if r.double_clicked() || (entered && cursor == Some(at)) {
                    actions.push(open());
                }
                mg_uikit::context_menu(&r, |ui| {
                    if ui.button("Open").clicked() {
                        actions.push(open());
                    }
                    // (A copy's as its layer has it: a model, a blueprint.)
                    let previewable = crate::model_view::previewable(k.restype)
                        && (copy_arc.is_some() || k.restype != ResType::MDL);
                    if previewable
                        && ui.button("Preview").on_hover_text("See it in the 3D viewer").clicked()
                    {
                        let tab = match copy_arc {
                            Some(layer) => Tab::ModelCopy { key: *k, layer: layer.to_string() },
                            None => Tab::Model(*k),
                        };
                        actions.push(Action::OpenTab(tab));
                    }
                    if !game_talk
                        && ui.button("Save As…").on_hover_text("As a file, as it is").clicked()
                    {
                        actions.push(Action::SaveResource(*k, copy_arc.map(|l| l.to_string())));
                    }
                    if k.restype == ResType::MDL
                        && ui
                            .button("Save As Text…")
                            .on_hover_text("A compiled model as the text it compiles from")
                            .clicked()
                    {
                        actions.push(Action::SaveModelText(*k, copy_arc.map(|l| l.to_string())));
                    }
                });
            }
        }
    });
    if let Some(at) = clicked {
        browser.cursor = Some(at);
        give_keys(ui.ctx(), Keys::Browser);
    }
    crate::palette_view::keys_outline(ui, out.inner_rect, Keys::Browser);
}

/// The layer the game's `dialog.tlk` is listed in (`lang:en`): the
/// install's language's folder, no layer of the load order.
pub(crate) fn game_talk_layer(language: mg_core::Language) -> String {
    format!("lang:{}", language.short_code().unwrap_or("en"))
}

/// Whether a row of the list is the game's `dialog.tlk`.
fn is_game_talk(row: &Row) -> bool {
    row.key == crate::talk_view::dialog_key() && row.layer.starts_with("lang:")
}

/// The tab a resource of the load order opens in from the Resources
/// panel (and Quick Open): an overridden copy (`copy`, its layer's label)
/// as that layer has it, a model in the model viewer, a resource the
/// module has in its editor, else read-only (`Tab::Resource`).
fn tab_of(key: ResKey, copy: Option<String>, in_module: bool) -> Tab {
    match (copy, Tab::for_resource(key)) {
        (Some(layer), _) => Tab::ResourceCopy { key, layer },
        (None, Some(t @ Tab::Model(_))) => t,
        (None, Some(t)) if in_module => t,
        _ => Tab::Resource(key),
    }
}

/// [`tab_of`] for the window's module.
pub(crate) fn tab_for(app: &Moonglow, key: ResKey, copy: Option<String>) -> Tab {
    let in_module = app.ws.as_ref().is_some_and(|w| w.module.contains(&key));
    tab_of(key, copy, in_module)
}

/// Where a table shown in the browser is to be edited.
pub(crate) enum EditTable {
    InModule,
    Copy(crate::table_view::Home),
    CopyInFile,
    /// The table a hak of the module has: the hak's file.
    InHak(std::path::PathBuf),
}

/// The module's haks that have `key`, highest first: each one's name and
/// its file, where that is the builder's own (not one of the game's).
fn haks_with(app: &Moonglow, key: ResKey) -> Vec<(String, std::path::PathBuf)> {
    let (Some(game), Some(install)) = (&app.game, &app.install) else { return Vec::new() };
    let mut layers: Vec<_> = game.resman.layers().iter().collect();
    layers.sort_by_key(|l| std::cmp::Reverse(l.priority));
    layers
        .into_iter()
        .filter_map(|l| {
            let name = l.label.strip_prefix("hak:")?;
            if !l.container.contains(&key) {
                return None;
            }
            let path = install.find_hak(name)?;
            (!path.starts_with(&install.root)).then(|| (name.to_string(), path))
        })
        .collect()
}

/// The places a table can be edited in, as buttons (the read-only tab's
/// Edit… menu, and Area Properties' table buttons): the one clicked.
pub(crate) fn table_homes(app: &Moonglow, ui: &mut Ui, key: ResKey) -> Option<EditTable> {
    use crate::table_view::Home;
    let mut out = None;
    let in_module = app.ws.as_ref().is_some_and(|w| w.module.contains(&key));
    {
        if in_module {
            if ui.button("The Module's Table").clicked() {
                out = Some(EditTable::InModule);
            }
        } else if app.ws.is_some()
            && ui
                .button("A Copy in the Module")
                .on_hover_text("The module's own table from then on, over the haks'")
                .clicked()
        {
            out = Some(EditTable::Copy(Home::Module));
        }
        // The table as a hak of the module has it: edited in the hak, which
        // the hak editor opens.
        for (name, path) in haks_with(app, key) {
            let open = app.haks.iter().any(|d| d.hak.path.as_deref() == Some(&path));
            let tip = match open {
                true => "Saves the table into the hak. The hak's Save writes its file.",
                false => "Opens the hak in the hak editor and saves the table into it",
            };
            if ui.button(format!("The Table of {name}.hak")).on_hover_text(tip).clicked() {
                out = Some(EditTable::InHak(path));
            }
        }
        let own: Vec<_> = haks_with(app, key).into_iter().map(|(_, path)| path).collect();
        for doc in &app.haks {
            // (Its own table is offered above.)
            if doc.hak.path.as_ref().is_some_and(|p| own.contains(p)) {
                continue;
            }
            let name = doc.title();
            let name = name.trim_end_matches(" *");
            let has = doc.hak.items().iter().any(|i| i.key == key);
            let label = match has {
                true => format!("A Copy in {name} (in place of its own)"),
                false => format!("A Copy in {name}"),
            };
            if ui.button(label).on_hover_text("A hak open in the hak editor").clicked() {
                out = Some(EditTable::Copy(Home::Hak(doc.id)));
            }
        }
        if ui.button("A Copy in a File…").clicked() {
            out = Some(EditTable::CopyInFile);
        }
    }
    out
}

/// Opens table `key` in the 2DA editor where `home` says; its document's
/// id, where it opened.
pub(crate) fn open_table(app: &mut Moonglow, key: ResKey, home: EditTable) -> Option<u32> {
    match home {
        EditTable::InModule => crate::table_view::open_in_module(app, key),
        EditTable::Copy(home) => crate::table_view::edit_copy(app, key, home),
        EditTable::CopyInFile => crate::table_view::edit_copy_in_file(app, key),
        EditTable::InHak(path) => {
            let hak = crate::hak_view::open_at(app, &path)?;
            crate::table_view::open_in_hak(app, hak, key)
        }
    }
}

/// Save As on a resource of the load order (the copy the game reads, or
/// the one `layer` has): its bytes to a file; or, `as_text`, a compiled
/// model as its text (one that is text already is saved as it is).
pub(crate) fn save_as(app: &mut Moonglow, key: ResKey, layer: Option<&str>, as_text: bool) {
    let Some(game) = &app.game else { return };
    let Some(data) = read_copy(&game.resman, &key, layer) else {
        match layer {
            Some(layer) => app.log.error(format!("{layer} has no {key}.")),
            None => app.log.error(format!("{key} is not in the load order.")),
        }
        return;
    };
    let skeleton = |n: &str| game.resman.get_named(n, ResType::MDL).ok().map(|d| d.into_owned());
    let data = match as_text.then(|| mg_mdl::decompile_with(&data, &skeleton)) {
        Some(Ok(text)) => text.into_bytes().into(),
        Some(Err(mg_mdl::DecompileError::NotBinary)) | None => data,
        Some(Err(e)) => {
            app.log.error(format!("{key} can't be written as text: {e}"));
            return;
        }
    };
    let Some(path) = app.dialogs.save_file(FileKind::Any, Some(&PathBuf::from(key.to_string())))
    else {
        return;
    };
    match mg_module::write_replacing(&path, &data) {
        Ok(()) => app.log.info(format!("Saved {key} to {}", path.display())),
        Err(e) => app.log.error(format!("Could not write {}: {e}", path.display())),
    }
}

/// The folder an overridden copy goes into, under Export as Files'
/// folder: its layer's label, in letters, digits, `-` and `_`
/// (`hak:mg_low` is `hak_mg_low`), a name every system takes.
fn layer_folder(layer: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    layer.chars().map(|c| if safe(c) { c } else { '_' }).collect()
}

/// Export as Files: the resources listed, each as `name.ext` in a folder
/// the user chooses; an overridden copy (with its layer's label) as its
/// layer has it, in a folder of the layer's under that one
/// ([`layer_folder`]).
pub(crate) fn export(app: &mut Moonglow, keys: &[(ResKey, Option<String>)]) {
    if app.game.is_none() {
        return;
    }
    let Some(dir) = app.dialogs.pick_folder("Export as files into", app.export_dir.as_deref())
    else {
        return;
    };
    // The files the folder has already: asked about before they are
    // written over (they were replaced without a word).
    let exporting = Exporting {
        there: (keys.iter())
            .filter(|(key, layer)| {
                export_path(&dir, key, layer.as_deref()).is_ok_and(|p| p.exists())
            })
            .cloned()
            .collect(),
        keys: keys.to_vec(),
        dir,
    };
    if exporting.there.is_empty() {
        export_as_job(app, exporting, false);
    } else {
        app.browser.exporting = Some(exporting);
    }
}

/// Where Export as Files writes a resource under `dir`: an overridden
/// copy in its layer's folder ([`layer_folder`]).
fn export_path(
    dir: &std::path::Path,
    key: &ResKey,
    layer: Option<&str>,
) -> std::io::Result<PathBuf> {
    match layer {
        Some(layer) => key.path_in(&dir.join(layer_folder(layer))),
        None => key.path_in(dir),
    }
}

/// An Export as Files into a folder that has some of its files already.
#[derive(Debug, Clone)]
pub(crate) struct Exporting {
    keys: Vec<(ResKey, Option<String>)>,
    dir: PathBuf,
    /// Those the folder has a file of.
    there: Vec<(ResKey, Option<String>)>,
}

/// Asks about the files an Export as Files would write over (with the
/// frame's other windows: the panel may be folded or another tab shown
/// before it is answered).
pub(crate) fn exporting_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(exporting) = app.browser.exporting.clone() else { return };
    let (n, all) = (exporting.there.len(), exporting.keys.len());
    let said = format!("{} already has {n} of the {all} files:", exporting.dir.display());
    let files: Vec<_> = (exporting.there.iter())
        .map(|(key, layer)| (key.to_string(), layer.as_ref().map(|l| format!("as {l} has it"))))
        .collect();
    let tips = [
        "Write all the files: these take the place of the folder's",
        "Write the other files and leave the folder's as they are",
    ];
    use crate::widgets::Already;
    let there = &mut app.settings.files_there;
    let Some(answer) = crate::widgets::files_already(ctx, there, "Folder", &said, &files, tips)
    else {
        return;
    };
    app.browser.exporting = None;
    match answer {
        Already::Replace => export_as_job(app, exporting, false),
        Already::Skip => export_as_job(app, exporting, true),
        Already::Cancel => {}
    }
}

/// What an Export as Files wrote.
#[derive(Debug, Default)]
struct Exported {
    written: usize,
    as_text: usize,
    copies: usize,
    /// Stopped by a file that could not be written.
    failed: Option<String>,
}

/// Writes an export's files (but for those there already, with `skip`)
/// as a job, with how far it is and Cancel: every listed resource was
/// read, a compiled model decompiled, and written inside one frame.
fn export_as_job(app: &mut Moonglow, exporting: Exporting, skip: bool) {
    let as_text = app.browser.models_as_text;
    let Exporting { keys, dir, there } = exporting;
    let to = dir.clone();
    app.start_job_anywhere(
        "Export as Files",
        move |cx| {
            let mut out = Exported::default();
            let Some(game) = cx.game.as_deref() else { return out };
            let there: std::collections::HashSet<&(ResKey, Option<String>)> =
                there.iter().filter(|_| skip).collect();
            let all = keys.len();
            for (at, entry) in keys.iter().enumerate() {
                if cx.progress.cancelled() {
                    break;
                }
                cx.progress.step(at, all);
                let (key, layer) = entry;
                if there.contains(entry) {
                    continue;
                }
                let Some(data) = read_copy(&game.resman, key, layer.as_deref()) else { continue };
                // (Asked for: a compiled model as its text; one that can't
                // be read so goes out as it is.)
                let text = as_text.then(|| model_text(*key, &data, Some(&game.resman))).flatten();
                out.as_text += usize::from(text.is_some());
                let data = text.map_or(data, |t| t.into_bytes().into());
                let wrote = export_path(&dir, key, layer.as_deref())
                    .and_then(|path| mg_module::write_replacing_one_of_many(&path, &data));
                if let Err(e) = wrote {
                    out.failed = Some(format!("Could not write {key} to {}: {e}", dir.display()));
                    break;
                }
                out.written += 1;
                out.copies += usize::from(layer.is_some());
            }
            out
        },
        move |app, out: Exported| {
            let models = match out.as_text {
                0 => String::new(),
                n => format!(" ({n} compiled models as text)"),
            };
            let copies = match out.copies {
                0 => String::new(),
                n => format!(", {n} overridden copies in folders of their layers"),
            };
            let n = out.written;
            app.log.info(format!("Exported {n} resources to {}{models}{copies}", to.display()));
            if let Some(e) = out.failed {
                app.log.error(format!("{e} (the rest were not written)"));
            }
            app.export_dir = Some(to);
        },
    );
}

/// A resource read and parsed once for its viewer.
#[derive(Debug)]
pub(crate) struct Viewed {
    data: Vec<u8>,
    origin: String,
    content: Content,
    /// Which reading this is, of all made ([`made`]): what is kept for
    /// its content (a 2DA's view) is this one's alone.
    made: u64,
    /// Its first bytes as the tab shows them in place of its viewer (a
    /// picture's, a sound's: Show Bytes), made once, when first asked.
    bytes: std::sync::OnceLock<LongText>,
}

impl Viewed {
    /// Its first bytes in hexadecimal, made once.
    fn bytes(&self) -> &LongText {
        self.bytes.get_or_init(|| hex(&self.data[..self.data.len().min(512)]).into())
    }
}

/// A number no other reading of a resource has had: what is made from
/// one and kept (a 2DA's view, with its own copy of the cells) is told
/// from another's by it. It was told by where the table lay in memory,
/// and a table read again after a change may lie where the old one did:
/// the view then showed the old cells under the new table's marks.
pub(crate) fn made() -> u64 {
    static MADE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Debug)]
enum Content {
    Gff(crate::ro_gff::ReadGff),
    TwoDa(mg_module::table_layers::Layered),
    Script(LongText),
    Text(LongText),
    Binary(LongText),
    Error(String),
    /// A TGA, a DDS or a PLT, shown as the hak editor's Preview shows it
    /// (`viewers`, M5's W6); decoded off the frame (made in
    /// [`resource_ui`], which has the window).
    Picture(Option<crate::viewers::Picture>),
    /// A WAV or a BMU, played as the hak editor's Preview plays it.
    Sound(std::sync::Arc<[u8]>),
    /// A model, drawn in the model viewer; its text (a compiled model's
    /// as it compiles from) made once, when Show as Text asks for it.
    Model(std::sync::OnceLock<LongText>),
}

/// A text shown read-only, drawn by its lines in sight ([`LongText::ui`]):
/// where its lines are, found once, and a script's lines colored, made
/// once for the colors they are shown in. The whole text was copied into
/// a label, hashed and (a script's) highlighted token by token every
/// frame it showed: `nwscript.nss` is a hundred thousand tokens, a
/// compiled model as text megabytes.
#[derive(Debug, Default)]
pub(crate) struct LongText {
    text: String,
    /// Each line's place in the text (without its line end), and the
    /// longest line's characters.
    lines: std::sync::OnceLock<(Vec<std::ops::Range<usize>>, usize)>,
    colored: std::sync::Mutex<Option<Colored>>,
}

/// A script's lines as laid out in a palette's colors.
type Colored = (crate::script_view::Palette, std::sync::Arc<Vec<egui::text::LayoutJob>>);

impl From<String> for LongText {
    fn from(text: String) -> LongText {
        LongText { text, ..Default::default() }
    }
}

impl LongText {
    /// Where each line is, and the longest's characters.
    fn lines(&self) -> &(Vec<std::ops::Range<usize>>, usize) {
        self.lines.get_or_init(|| {
            let (mut spans, mut widest, mut from) = (Vec::new(), 0, 0);
            for line in self.text.split('\n') {
                let trimmed = line.strip_suffix('\r').unwrap_or(line);
                spans.push(from..from + trimmed.len());
                widest = widest.max(trimmed.chars().count());
                from += line.len() + 1;
            }
            (spans, widest)
        })
    }

    /// The script's lines in `palette`'s colors: highlighted whole once
    /// (a comment may run over lines), and cut at the line ends.
    fn colored(
        &self,
        palette: &crate::script_view::Palette,
    ) -> std::sync::Arc<Vec<egui::text::LayoutJob>> {
        let mut kept = self.colored.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((of, jobs)) = &*kept
            && of == palette
        {
            return jobs.clone();
        }
        // (Each line as the script editor lays it out: its pitch.)
        let jobs = crate::script_view::highlight_lines(&self.text, palette);
        let jobs = std::sync::Arc::new(jobs);
        *kept = Some((*palette, jobs.clone()));
        jobs
    }

    /// Draws the text, scrolling both ways, its lines in sight alone: in
    /// the script editor's colors and on its background with `palette`,
    /// else plain. `salt`: the scroll area's.
    pub(crate) fn ui(
        &self,
        ui: &mut Ui,
        salt: impl std::hash::Hash + std::fmt::Debug,
        palette: Option<&crate::script_view::Palette>,
    ) {
        self.draw(ui, egui::Id::new(salt), palette, None);
    }

    /// [`LongText::ui`] with its Find bar, marked line and right-click menu
    /// (`find`, the tab's): `keys` the window's; `here` whether its tab is
    /// the one in front of the focused pane, `any_here` whether a
    /// read-only text's is ([`view_has_the_keys`]).
    pub(crate) fn ui_finding(
        &self,
        ui: &mut Ui,
        salt: impl std::hash::Hash + std::fmt::Debug,
        palette: Option<&crate::script_view::Palette>,
        find: &mut TextFind,
        (keys, here, any_here): (&crate::keys::Keymap, bool, bool),
    ) {
        let id = egui::Id::new(salt);
        let ctx = ui.ctx().clone();
        let pointer = ui.rect_contains_pointer(ui.max_rect());
        let takes = view_has_the_keys(&ctx, (here, any_here, pointer));
        menu_keys(&ctx, find);
        find_keys(self, ui, id, find, keys, takes);
        // (A right press in the text takes egui's choice of its lines away:
        // kept, and put back the frame after, as a field's selection is.)
        let labels = |f: &mut dyn FnMut(&mut egui::text_selection::LabelSelectionState)| {
            ctx.with_plugin::<egui::text_selection::LabelSelectionState, _>(|s| f(s));
        };
        if let Some(kept) = find.kept.take() {
            labels(&mut |s| *s = kept.clone());
        }
        let right_press = ctx.input(|i| {
            i.pointer.button_pressed(egui::PointerButton::Secondary)
                && (i.pointer.interact_pos().zip(find.view_rect))
                    .is_some_and(|(p, r)| r.contains(p))
        });
        if find.picked && ((right_press && pointer) || on_menu(&ctx, id)) {
            labels(&mut |s| find.kept = Some(s.clone()));
        }
        if std::mem::take(&mut find.copy_next) {
            ctx.input_mut(|i| i.events.push(egui::Event::Copy));
        }
        if find.open {
            find_bar(ui, id, find, keys);
            // (A slice of the text a frame, the window going on: the frames
            // go on until it is all looked through.)
            if find.sweep(self, FIND_BUDGET) {
                ui.ctx().request_repaint();
            }
        }
        self.draw(ui, id, palette, Some(find));
        let right_click = ctx.input(|i| {
            i.pointer.button_clicked(egui::PointerButton::Secondary)
                && (i.pointer.interact_pos().zip(find.view_rect))
                    .is_some_and(|(p, r)| r.contains(p))
        });
        if right_click && pointer {
            let at = ctx.pointer_interact_pos().unwrap_or_default();
            open_menu(&ctx, id, find, at, None);
        }
        if find.menu.is_some() {
            self.menu(&ctx, id, find, keys);
        }
    }

    /// The text's right-click menu, while open: the text fields' rows
    /// (`widgets::text_menu`'s words, order and keys), Cut and Paste grayed
    /// as in any text only to be read, Copy with lines chosen; then Find….
    fn menu(
        &self,
        ctx: &egui::Context,
        id: egui::Id,
        find: &mut TextFind,
        keys: &crate::keys::Keymap,
    ) {
        let Some(mut open) = find.menu else { return };
        let menu_id = menu_id(id);
        let selected = find.all || find.picked;
        let rows = menu_rows(selected);
        if open.lit.is_some_and(|i| !rows[i]) {
            open.lit = rows.iter().position(|&on| on);
        }
        let mac = ctx.os() == egui::os::OperatingSystem::Mac;
        let shortcut = |key| {
            crate::keys::shown(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key), mac)
        };
        let names = [
            ("Cut", shortcut(egui::Key::X)),
            ("Copy", shortcut(egui::Key::C)),
            ("Paste", shortcut(egui::Key::V)),
            ("Select All", shortcut(egui::Key::A)),
            ("Find…", keys.label(crate::keys::Cmd::Find, ctx)),
        ];
        let mut chosen = None;
        let layer = egui::LayerId::new(egui::Order::Foreground, menu_id);
        let shown = egui::Popup::new(menu_id, ctx.clone(), open.at, layer)
            .kind(egui::PopupKind::Menu)
            .style(mg_uikit::menu_style)
            .open_memory(None)
            .show(|ui| {
                mg_uikit::menu_rows(ui);
                for (i, (name, keys)) in names.iter().enumerate() {
                    if i == 3 || i == 4 {
                        mg_uikit::menu_separator(ui);
                    }
                    let lit = open.lit == Some(i) && rows[i];
                    let button = egui::Button::new(*name).selected(lit);
                    let r = ui.add_enabled_ui(rows[i], |ui| {
                        mg_uikit::menu_row_with_keys(ui, button, keys)
                    });
                    if rows[i] && (r.inner.clicked() || (lit && open.choose)) {
                        chosen = Some(i);
                    }
                }
            });
        open.choose = false;
        let closed = shown.as_ref().is_none_or(|s| s.response.should_close())
            || !egui::Popup::is_id_open(ctx, menu_id);
        find.menu = (chosen.is_none() && !closed).then_some(open);
        if find.menu.is_none() {
            egui::Popup::close_id(ctx, menu_id);
        }
        match chosen {
            Some(1) if find.all => ctx.copy_text(self.text.clone()),
            Some(1) => {
                // (egui's Copy, sent to the lines as they are next drawn:
                // what was chosen of them is theirs to copy.)
                find.copy_next = true;
                ctx.request_repaint();
            }
            Some(3) => find.all = true,
            Some(4) => find.open(),
            _ => {}
        }
    }

    /// Draws the text, its lines in sight alone, with `find`'s matches and
    /// marked line.
    fn draw(
        &self,
        ui: &mut Ui,
        id: egui::Id,
        palette: Option<&crate::script_view::Palette>,
        mut find: Option<&mut TextFind>,
    ) {
        let (spans, widest) = self.lines();
        let font = match palette {
            Some(p) => p.font(),
            None => egui::TextStyle::Monospace.resolve(ui.style()),
        };
        let (height, wide) = ui.fonts_mut(|f| (f.row_height(&font), f.glyph_width(&font, 'm')));
        // (A script's lines the script editor's distance apart, as egui
        // rounds them: `show_rows` counts the lines in sight by it.)
        let height = palette.map_or(height, |p| p.row_height(ui.ctx().pixels_per_point()));
        let jobs = palette.map(|p| self.colored(p));
        let ctx = ui.ctx().clone();
        // (The marked line's band as the script editor draws its cursor's:
        // in the colors of the ground's lightness.)
        let tokens = match palette.and_then(|p| p.background) {
            Some(g) if u32::from(g.r()) + u32::from(g.g()) + u32::from(g.b()) < 384 => {
                mg_uikit::Tokens::DARK
            }
            Some(_) => mg_uikit::Tokens::LIGHT,
            None => mg_uikit::tokens(&ctx),
        };
        let selection = ui.visuals().selection;
        ui.scope(|ui| {
            // (A line under the last, as one text's lines are.)
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut rows = egui::ScrollArea::both().id_salt(id).auto_shrink([false, false]);
            // (A line sent to, or a match found, brought into sight.)
            let mut sent = None;
            if let Some(f) = find.as_deref_mut()
                && let Some((line, chars)) = f.reveal.take()
            {
                let (offset, size) = f.view.unwrap_or((egui::Vec2::ZERO, ui.available_size()));
                let top = line as f32 * height;
                let y = if top < offset.y || top + height > offset.y + size.y {
                    (top - (size.y - height) / 2.0).max(0.0)
                } else {
                    offset.y
                };
                let (left, right) = (chars.start as f32 * wide, chars.end as f32 * wide);
                let x = if left < offset.x || right > offset.x + size.x {
                    (left - size.x / 3.0).max(0.0)
                } else {
                    offset.x
                };
                rows = rows.scroll_offset(egui::vec2(x, y));
                sent = Some(((line, chars), y));
            }
            let shown = rows.show_rows(ui, height, spans.len(), |ui, range| {
                // (On the script editor's background, where one is chosen:
                // its colors are chosen for it.)
                if let Some(fill) = palette.and_then(|p| p.background) {
                    ui.painter().rect_filled(ui.clip_rect(), 0.0, fill);
                }
                ui.set_min_width(*widest as f32 * wide);
                let sight = ui.clip_rect().x_range();
                for at in range {
                    let line = &self.text[spans[at].clone()];
                    // (What is drawn under the line: its band, its matches.)
                    let under = ui.painter().add(egui::Shape::Noop);
                    let rect = if line.is_empty() {
                        ui.allocate_exact_size(egui::vec2(0.0, height), egui::Sense::hover()).0
                    } else if let Some(job) = jobs.as_ref().and_then(|j| j.get(at)) {
                        ui.add(egui::Label::new(job.clone()).extend()).rect
                    } else {
                        let text = egui::RichText::new(line).font(font.clone());
                        ui.add(egui::Label::new(text).extend()).rect
                    };
                    let Some(f) = find.as_deref() else { continue };
                    let mut shapes = Vec::new();
                    let band = egui::Rangef::new(rect.top(), rect.top() + height);
                    if f.marked == Some(at) {
                        let whole = egui::Rect::from_x_y_ranges(sight, band);
                        shapes.push(egui::Shape::rect_filled(whole, 0.0, tokens.current_line));
                        for edge in crate::script_view::hairlines(band, ctx.pixels_per_point()) {
                            let edge = egui::Rect::from_x_y_ranges(sight, edge);
                            shapes.push(egui::Shape::rect_filled(
                                edge,
                                0.0,
                                tokens.current_line_edge,
                            ));
                        }
                    }
                    if f.all && !line.is_empty() {
                        shapes.push(egui::Shape::rect_filled(rect, 0.0, selection.bg_fill));
                    }
                    let found = if f.open { f.in_line(at) } else { Vec::new() };
                    if !found.is_empty() {
                        // (Where its characters are as the line is laid
                        // out: a tab is wider than a letter.)
                        let galley = match jobs.as_ref().and_then(|j| j.get(at)) {
                            Some(job) => ui.fonts_mut(|x| x.layout_job(job.clone())),
                            None => ui.fonts_mut(|x| {
                                x.layout_no_wrap(
                                    line.to_string(),
                                    font.clone(),
                                    selection.stroke.color,
                                )
                            }),
                        };
                        let x = |c: usize| {
                            let at = galley.pos_from_cursor(egui::text::CCursor::new(c));
                            rect.left() + at.left()
                        };
                        for (i, m) in found {
                            let x = egui::Rangef::new(x(m.chars.0), x(m.chars.1));
                            let r = egui::Rect::from_x_y_ranges(x, band);
                            if Some(i) == f.current {
                                shapes.push(egui::Shape::rect_filled(r, 2.0, selection.bg_fill));
                                let stroke = egui::Stroke::new(1.0, selection.stroke.color);
                                let inside = egui::StrokeKind::Inside;
                                shapes.push(egui::Shape::rect_stroke(r, 2.0, stroke, inside));
                            } else {
                                let fill = selection.bg_fill.gamma_multiply(0.45);
                                shapes.push(egui::Shape::rect_filled(r, 2.0, fill));
                            }
                        }
                    }
                    ui.painter().set(under, egui::Shape::Vec(shapes));
                }
            });
            if let Some(f) = find {
                // (Not there yet: the view had no room, as a window's first
                // frame, which sizes it, has none. Again next frame, a few
                // times at most.)
                if let Some((s, y)) = sent {
                    let room = (shown.content_size.y - shown.inner_rect.height()).max(0.0);
                    let short = (shown.state.offset.y - y.min(room)).abs() > 1.0;
                    if short && f.tries < 5 {
                        f.tries += 1;
                        f.reveal = Some(s);
                        ctx.request_repaint();
                    } else {
                        f.tries = 0;
                    }
                }
                f.view = Some((shown.state.offset, shown.inner_rect.size()));
                f.view_rect = Some(shown.inner_rect);
                f.top_line = (shown.state.offset.y / height.max(1.0)) as usize;
                // (A press in the text: what Select All chose and the
                // marked line let go of, as the editor's band moves.)
                // (A press on the text's own menu is the menu's.)
                if on_menu(&ctx, id) {
                    return;
                }
                let pressed = ctx.input(|i| i.pointer.any_pressed())
                    && ui.rect_contains_pointer(shown.inner_rect);
                // (Lines chosen by egui: dragged over from a press in the
                // text, or a word or line clicked twice or three times; a
                // press, Escape or a press elsewhere lets go of them, as
                // egui does.)
                let (primary, dragging, twice, escape, elsewhere) = ctx.input(|i| {
                    let p = &i.pointer;
                    (
                        p.primary_pressed(),
                        p.primary_down() && p.is_decidedly_dragging(),
                        p.button_double_clicked(egui::PointerButton::Primary)
                            || p.button_triple_clicked(egui::PointerButton::Primary),
                        i.key_pressed(egui::Key::Escape),
                        p.any_pressed() && !pressed,
                    )
                });
                if (pressed && primary) || escape || elsewhere {
                    f.picked = false;
                }
                let inside =
                    ctx.pointer_interact_pos().is_some_and(|p| shown.inner_rect.contains(p));
                if inside && (twice || (dragging && f.pressed_in)) {
                    f.picked = true;
                }
                if pressed && primary {
                    f.pressed_in = true;
                } else if elsewhere {
                    f.pressed_in = false;
                }
                // (Only a right press keeps what was chosen, for its menu.)
                if pressed && primary {
                    f.all = false;
                    f.marked = None;
                }
                if pressed {
                    // (The keys the text's, as a click in the editor area
                    // takes them from a list: Find's Ctrl+F the view's.)
                    crate::palette_view::give_keys(&ctx, crate::palette_view::Keys::Area);
                }
            }
        });
    }
}

/// How long Find looks through a read-only text a frame: `nwscript.nss`
/// (700 KB) is looked through over several, the window going on, as the
/// talk table's Find is.
const FIND_BUDGET: std::time::Duration = std::time::Duration::from_millis(3);

/// How many lines Find looks through between two looks at the clock.
const FIND_SLICE: usize = 64;

/// A read-only text's Find bar (Ctrl+F: the script editor's Find words
/// and keys, without Replace) and the line it was sent to (Go to
/// Definition in a game script), the tab's while it is open.
#[derive(Debug, Default)]
pub(crate) struct TextFind {
    /// The bar is shown.
    pub(crate) open: bool,
    /// What is looked for (Find What), and how.
    pub(crate) find: String,
    pub(crate) options: crate::script_tools::SearchOptions,
    /// The field is to take the keys when next drawn.
    focus: bool,
    /// The field had the keys as it was last drawn (egui lets go of them
    /// at an Escape or an Enter before anything of the frame is drawn).
    field_had: bool,
    /// What was found for what was typed.
    found: Option<Found>,
    /// The match chosen, in `found`'s.
    current: Option<usize>,
    /// The first line in sight as last drawn: a new search's match is
    /// chosen from it on.
    top_line: usize,
    /// Where a new search's match is chosen from.
    from_line: usize,
    /// A line to bring into sight when next drawn, and the characters of
    /// it to show.
    reveal: Option<(usize, std::ops::Range<usize>)>,
    /// The line marked as the script editor marks the line it is sent to.
    pub(crate) marked: Option<usize>,
    /// Select All: the whole text chosen, which Copy copies.
    all: bool,
    /// The view's scroll and size, as last drawn.
    view: Option<(egui::Vec2, egui::Vec2)>,
    /// Its tab has drawn a text (not yet: a tab sent to a line, opening).
    shown: bool,
    /// How many frames a line to bring into sight was sent again for.
    tries: u8,
    /// Where the text's view was drawn last (for tests).
    view_rect: Option<egui::Rect>,
    /// Its right-click menu, while open.
    menu: Option<ViewMenu>,
    /// Lines in sight dragged over (or a word or line clicked twice or
    /// three times): egui's own choice of the labels, which Copy copies.
    picked: bool,
    /// The last press of the pointer was in the text (a drag from it
    /// chooses lines).
    pressed_in: bool,
    /// egui's choice of the labels as a right press began, put back the
    /// frame after (a press takes it away, on a line or beside one).
    kept: Option<egui::text_selection::LabelSelectionState>,
    /// The menu's Copy, done as the next frame draws the lines: egui's
    /// Copy sent to them.
    copy_next: bool,
}

/// A read-only text's right-click menu: where it opened and the row the
/// keys are at (Shift+F10 opens it at the first, Up and Down move,
/// Enter or Space chooses).
#[derive(Debug, Clone, Copy)]
struct ViewMenu {
    at: egui::Pos2,
    lit: Option<usize>,
    choose: bool,
}

/// A search through a read-only text: what was looked for in which text
/// (its place and length), the matches so far (each its line and its
/// characters in the line, in order) and the next line to look at
/// (`None`: all looked through).
#[derive(Debug, Clone)]
struct Found {
    query: String,
    options: crate::script_tools::SearchOptions,
    of: (usize, usize),
    matches: Vec<Match>,
    next: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Match {
    line: usize,
    chars: (usize, usize),
}

/// Every match of `needle` in `line`, each its characters, as the
/// script editor's Find finds them (`script_tools::find`'s rules: not
/// overlapping, from the start; without Match Case, letter by letter with
/// case and accents aside; whole words of letters, digits and `_`), in
/// one pass (a text of one long line, minified, is looked through once,
/// not once a match).
fn line_matches(
    line: &str,
    needle: &str,
    o: &crate::script_tools::SearchOptions,
) -> Vec<(usize, usize)> {
    let pat: Vec<char> = needle.chars().collect();
    let mut out = Vec::new();
    if pat.is_empty() {
        return out;
    }
    let hay: Vec<char> = line.chars().collect();
    let Some(last) = hay.len().checked_sub(pat.len()) else { return out };
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let same =
        |a: char, b: char| if o.match_case { a == b } else { mg_core::text::same_letter(a, b) };
    let mut s = 0;
    while s <= last {
        let whole = !o.whole_word
            || ((s == 0 || !ident(hay[s - 1]))
                && hay.get(s + pat.len()).is_none_or(|&c| !ident(c)));
        if whole && hay[s..s + pat.len()].iter().zip(&pat).all(|(&h, &p)| same(h, p)) {
            out.push((s, s + pat.len()));
            s += pat.len();
        } else {
            s += 1;
        }
    }
    out
}

impl TextFind {
    /// Opens the bar, its field with the keys.
    pub(crate) fn open(&mut self) {
        self.open = true;
        self.focus = true;
        self.all = false;
    }

    /// Marks `line` and brings it into sight when next drawn.
    pub(crate) fn go_to_line(&mut self, line: usize) {
        self.marked = Some(line);
        self.reveal = Some((line, 0..0));
    }

    /// Looks on through `text` for what is typed, from where it was, for
    /// at most `budget` (the clock asked every [`FIND_SLICE`] lines);
    /// begun again where what is typed, its options or the text changed.
    /// Whether there is more to look through.
    fn sweep(&mut self, text: &LongText, budget: std::time::Duration) -> bool {
        let of = (text.text.as_ptr() as usize, text.text.len());
        let fresh = self
            .found
            .as_ref()
            .is_some_and(|s| s.query == self.find && s.options == self.options && s.of == of);
        if !fresh {
            self.current = None;
            self.from_line = self.top_line;
            self.found = (!self.find.is_empty()).then(|| Found {
                query: self.find.clone(),
                options: self.options.clone(),
                of,
                matches: Vec::new(),
                next: Some(0),
            });
        }
        let Some(s) = &mut self.found else { return false };
        let Some(mut at) = s.next else { return false };
        let spans = &text.lines().0;
        let options = crate::script_tools::SearchOptions { backwards: false, ..s.options.clone() };
        let began = std::time::Instant::now();
        loop {
            let end = (at + FIND_SLICE).min(spans.len());
            for line in at..end {
                let found = line_matches(&text.text[spans[line].clone()], &s.query, &options);
                s.matches.extend(found.into_iter().map(|chars| Match { line, chars }));
            }
            at = end;
            if at >= spans.len() || began.elapsed() >= budget {
                break;
            }
        }
        s.next = (at < spans.len()).then_some(at);
        // The first match from the line in sight on, as soon as found
        // (else the first of all, once all are looked through).
        if self.current.is_none() {
            let from = s.matches.partition_point(|m| m.line < self.from_line);
            let pick = match from < s.matches.len() {
                true => Some(from),
                false => (s.next.is_none() && !s.matches.is_empty()).then_some(0),
            };
            if let Some(i) = pick {
                self.choose(i);
            }
        }
        self.found.as_ref().is_some_and(|s| s.next.is_some())
    }

    /// The match `i` chosen and brought into sight.
    fn choose(&mut self, i: usize) {
        let Some(m) = self.found.as_ref().and_then(|s| s.matches.get(i)) else { return };
        self.current = Some(i);
        self.reveal = Some((m.line, m.chars.0..m.chars.1));
    }

    /// The next match (`forward`) or the previous, round from the last to
    /// the first once all are found.
    fn step(&mut self, forward: bool) {
        let Some(s) = &self.found else { return };
        let (n, done) = (s.matches.len(), s.next.is_none());
        if n == 0 {
            return;
        }
        let i = match (self.current, forward) {
            (None, true) => 0,
            (None, false) => n - 1,
            (Some(i), true) if i + 1 < n => i + 1,
            (Some(_), true) if done => 0,
            (Some(i), false) if i > 0 => i - 1,
            (Some(_), false) if done => n - 1,
            (Some(i), _) => i,
        };
        self.choose(i);
    }

    /// The matches in `line`, each with its place among all.
    fn in_line(&self, line: usize) -> Vec<(usize, Match)> {
        let Some(s) = &self.found else { return Vec::new() };
        let from = s.matches.partition_point(|m| m.line < line);
        let to = from + s.matches[from..].partition_point(|m| m.line == line);
        (from..to).map(|i| (i, s.matches[i])).collect()
    }

    /// What the bar's count says: the match chosen of how many, how many
    /// so far while it looks, or none.
    fn count(&self) -> String {
        let Some(s) = &self.found else { return String::new() };
        let n = s.matches.len();
        match (s.next.is_some(), n, self.current) {
            (true, _, _) => format!("{n} found so far"),
            (false, 0, _) => "No matches".into(),
            (false, 1, _) => "1 match".into(),
            (false, n, Some(i)) => format!("{} of {n} matches", i + 1),
            (false, n, None) => format!("{n} matches"),
        }
    }

    /// For tests: the matches found, whether all is looked through, the
    /// match chosen (its line, from 0) and the marked line.
    pub(crate) fn state(&self) -> (usize, bool, Option<usize>, Option<usize>) {
        let found = self.found.as_ref();
        let line = self.current.and_then(|i| Some(found?.matches.get(i)?.line));
        let n = found.map_or(0, |s| s.matches.len());
        (n, found.is_none_or(|s| s.next.is_none()), line, self.marked)
    }
}

/// The Find bar's field's id, for a view's `id`.
fn find_field_id(id: egui::Id) -> egui::Id {
    id.with("find-field")
}

/// The right-click menu's id (an egui popup), for a view's `id`.
fn menu_id(id: egui::Id) -> egui::Id {
    id.with("menu")
}

/// Whether a press of the pointer landed on the view `id`'s menu.
fn on_menu(ctx: &egui::Context, id: egui::Id) -> bool {
    let layer = egui::LayerId::new(egui::Order::Foreground, menu_id(id));
    ctx.input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()))
        .flatten()
        .is_some_and(|p| ctx.layer_id_at(p) == Some(layer))
}

/// Which of the menu's rows are on (Cut, Copy, Paste, Select All, Find…):
/// Copy with lines chosen (`selected`), never Cut or Paste.
fn menu_rows(selected: bool) -> [bool; 5] {
    [false, selected, false, true, true]
}

/// Opens the view `id`'s menu at `at`; from the keys (`lit`), its first
/// row that is on lit.
fn open_menu(
    ctx: &egui::Context,
    id: egui::Id,
    find: &mut TextFind,
    at: egui::Pos2,
    lit: Option<()>,
) {
    let rows = menu_rows(find.all || find.picked);
    let lit = lit.and_then(|()| rows.iter().position(|&on| on));
    find.menu = Some(ViewMenu { at, lit, choose: false });
    egui::Popup::open_id(ctx, menu_id(id));
    ctx.request_repaint();
}

/// While the view's menu is open, its keys: Up and Down move along its
/// rows that are on, Enter or Space chooses, Escape or Tab closes it
/// (not the Find bar).
fn menu_keys(ctx: &egui::Context, find: &mut TextFind) {
    use egui::{Event, Key};
    let Some(open) = &mut find.menu else { return };
    let rows = menu_rows(find.all || find.picked);
    let mut close = false;
    ctx.input_mut(|i| {
        i.events.retain(|e| {
            let Event::Key { key, pressed, .. } = e else { return true };
            match key {
                Key::ArrowDown | Key::ArrowUp if *pressed => {
                    let down = *key == Key::ArrowDown;
                    let n = rows.len();
                    let mut at = open.lit;
                    for _ in 0..n {
                        let i = match (at, down) {
                            (None, true) => 0,
                            (None, false) => n - 1,
                            (Some(i), true) => (i + 1) % n,
                            (Some(i), false) => (i + n - 1) % n,
                        };
                        at = Some(i);
                        if rows[i] {
                            break;
                        }
                    }
                    open.lit = at;
                }
                Key::Enter | Key::Space if *pressed => open.choose = open.lit.is_some(),
                Key::Escape | Key::Tab if *pressed => close = true,
                Key::ArrowDown | Key::ArrowUp | Key::Enter | Key::Space => {}
                Key::Escape | Key::Tab => {}
                _ => return true,
            }
            false
        });
        i.events.retain(|e| !matches!(e, Event::Text(t) if t == " "));
    });
    if close {
        find.menu = None;
    }
}

/// The view (a read-only text, `id`) that takes the keys of Find, Select
/// All, Copy and its menu, when no field has them: exactly one, the
/// first of
/// - the one whose tab is in front of the focused pane (`here`; a press
///   in a view brings its tab there);
/// - with none such (`any_here`), the one the pointer is over, only while
///   no view was ever pressed in (a window just opened).
///
/// A view pressed in, then a press elsewhere, then none here: none takes
/// them (the pointer resting over a text the builder is not in takes no
/// keys).
fn view_has_the_keys(ctx: &egui::Context, (here, any_here, pointer): (bool, bool, bool)) -> bool {
    let key = egui::Id::new("mg-read-only-view-pressed");
    let pressed = ctx.input(|i| i.pointer.any_pressed());
    if pressed && pointer {
        ctx.data_mut(|d| d.insert_temp(key, true));
    }
    let ever = ctx.data(|d| d.get_temp::<bool>(key)).is_some();
    here || (!any_here && !ever && pointer)
}

/// The keys of a read-only text with its Find: Find (Ctrl+F) opens the
/// bar, Find Next (F3) and Shift+F3 go on, Escape closes it and gives the
/// keys back to the text, Select All (Ctrl+A) chooses the whole text and
/// Copy copies it. Not while another field has the keys.
fn find_keys(
    text: &LongText,
    ui: &Ui,
    id: egui::Id,
    find: &mut TextFind,
    keys: &crate::keys::Keymap,
    here: bool,
) {
    use crate::keys::Cmd;
    let ctx = ui.ctx();
    let field = find_field_id(id);
    // (The bar's field, or nothing, has the keys.)
    let ours = find.field_had || ctx.memory(|m| m.has_focus(field));
    // (Nor while a list drawn before the editor area has the keys: the
    // module tree's Ctrl+F is its Filter's.)
    let free = !ctx.egui_wants_keyboard_input() && !crate::palette_view::a_list_has_the_keys(ctx);
    if !(ours || (here && free)) {
        return;
    }
    if ctx.input_mut(|i| keys.consume(i, Cmd::Find)) {
        find.open();
    }
    if find.open {
        if ctx.input_mut(|i| keys.consume(i, Cmd::FindNext)) {
            find.step(true);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::F3)) {
            find.step(false);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            find.open = false;
            find.field_had = false;
            ctx.memory_mut(|m| m.surrender_focus(field));
        }
    }
    if free && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::A)) {
        find.all = true;
    }
    // (Shift+F10: the menu, at the text's top left, its first row on lit.
    // In the bar's field, the field's own.)
    if free
        && find.menu.is_none()
        && let Some(view) = find.view_rect
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, egui::Key::F10))
    {
        open_menu(ctx, id, find, view.left_top() + egui::vec2(8.0, 8.0), Some(()));
    }
    if find.all && ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy))) {
        ctx.copy_text(text.text.clone());
    }
}

/// The Find bar over a read-only text: Find What, the previous and next
/// match, Match Case, Match Whole Word Only, the count and Close.
fn find_bar(ui: &mut Ui, id: egui::Id, find: &mut TextFind, keys: &crate::keys::Keymap) {
    use crate::keys::Cmd;
    let ctx = ui.ctx().clone();
    let field_id = find_field_id(id);
    let next_keys = keys.label(Cmd::FindNext, &ctx);
    let tip = |what: &str, keys: &str| match keys.is_empty() {
        true => what.to_string(),
        false => format!("{what} ({keys})"),
    };
    let mut forward = None;
    let mut close = false;
    egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 4)).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            crate::widgets::field_label(ui, "Find What");
            let r = ui.add(mg_uikit::text_field(&mut find.find).id(field_id).desired_width(220.0));
            r.ctx.accesskit_node_builder(r.id, |node| node.set_label("Find What"));
            if std::mem::take(&mut find.focus) {
                r.request_focus();
            }
            // (Enter and Shift+Enter: the next and the previous, the
            // field keeping the keys.)
            if find.field_had && r.lost_focus() {
                let enter =
                    ctx.input(|i| i.key_pressed(egui::Key::Enter).then_some(!i.modifiers.shift));
                if let Some(ahead) = enter {
                    forward = Some(ahead);
                    r.request_focus();
                }
            }
            find.field_had = r.has_focus() || forward.is_some();
            let previous = mg_uikit::IconButton::new(mg_uikit::Icon::ChevronUp, "Find Previous");
            let b = ui.add(previous.icon_only(true));
            if b.on_hover_text(tip("The previous match", "Shift+F3")).clicked() {
                forward = Some(false);
            }
            let next = mg_uikit::IconButton::new(mg_uikit::Icon::ChevronDown, "Find Next");
            let b = ui.add(next.icon_only(true));
            if b.on_hover_text(tip("The next match", &next_keys)).clicked() {
                forward = Some(true);
            }
            ui.check_box(&mut find.options.match_case, "Match Case");
            ui.check_box(&mut find.options.whole_word, "Match Whole Word Only");
            ui.weak(find.count());
            let b = ui.add(crate::widgets::clear_button().name("Close Find"));
            close = b.on_hover_text(tip("Close Find", "Esc")).clicked();
        });
    });
    if let Some(ahead) = forward {
        find.step(ahead);
    }
    if close {
        find.open = false;
        find.field_had = false;
    }
}

/// The read-only tab's count at the Status Bar's right (the Resources
/// board's "nw_chicken.utc from key:nwn_base, 3078 bytes"): the resource,
/// the layer it is read from and its size. `None` before it is read.
pub(crate) fn resource_count(app: &Moonglow, key: ResKey, layer: Option<&str>) -> Option<String> {
    let v = app.viewed.get(&(key, layer.map(str::to_string)))?;
    let bytes = match v.data.len() {
        1 => "1 byte".to_string(),
        n => format!("{n} bytes"),
    };
    Some(format!("{key} from {}, {bytes}", v.origin))
}

/// The read-only tabs' pictures not drawn for a while (their tabs closed,
/// behind others, their windows shut: as long as a model view keeps its
/// targets, `model_view::UNSEEN_AFTER`) let go of with what was read of
/// them; a tab shown again reads and decodes its picture anew. `now`:
/// the window's clock.
pub(crate) fn let_go_of_pictures(app: &mut Moonglow, now: f64) {
    if app.browser.pictures.is_empty() {
        return;
    }
    let after = crate::model_view::UNSEEN_AFTER;
    let unseen: Vec<(ResKey, Option<String>)> = (app.browser.pictures.iter())
        .filter(|(_, t)| now - **t > after)
        .map(|(k, _)| k.clone())
        .collect();
    for at in unseen {
        app.browser.pictures.remove(&at);
        app.viewed.remove(&at);
    }
}

/// The read-only GFF views' counts, wherever they are drawn (the tab, the
/// hak editor's Preview: `ro_gff::counts`, one counter for all since
/// M4's U23 fix): how many times their shown rows were listed, all told,
/// and the most rows one drew in the last pass that drew one.
pub(crate) fn read_only_counts() -> (usize, usize) {
    crate::ro_gff::counts()
}

impl Browser {
    /// Evicts the least-recently-used cached read-only 2DA view when at capacity.
    fn evict_lru_table_view(&mut self) {
        if self.table_views.len() < VIEWS_KEPT {
            return;
        }
        if let Some(evict) =
            self.table_views.iter().min_by_key(|(_, v)| v.last_used).map(|(k, _)| k.clone())
        {
            self.table_views.remove(&evict);
        }
    }

    /// The panel's count (the Resources board's "627 of 113538
    /// resources"), as its list was last filtered: also the Status Bar's
    /// while the panel is in use.
    pub(crate) fn count_text(&self) -> String {
        let copies = match self.shown_copies {
            0 => String::new(),
            1 => ", 1 overridden copy".into(),
            n => format!(", {n} overridden copies"),
        };
        let found = self.shown.len().saturating_sub(self.shown_copies);
        format!("{found} of {} resources{copies}", self.resources)
    }
}

impl Viewed {
    /// The copy the game reads, or (`layer`, by its label) the one that
    /// layer has.
    fn load(game: &mg_rules::GameData, key: ResKey, layer: Option<&str>) -> Option<Viewed> {
        let rm = &game.resman;
        let at = match layer {
            Some(label) => rm.layers().iter().position(|l| l.label == label)?,
            None => rm.find(&key)?,
        };
        let data = rm.get_in(at, &key).ok()?.into_owned();
        let origin = rm.layers()[at].label.clone();
        let codepage = game.codepage();
        let content = if key.restype.is_gff() {
            Gff::read(&data)
                .map_or_else(|e| Content::Error(e.to_string()), |g| Content::Gff(g.into()))
        } else if key.restype == ResType::TWODA {
            // (A copy that another overrides: as it stands on those below.)
            match mg_module::table_layers::layered_from(rm, &key, codepage, at) {
                Some(l) => Content::TwoDa(l),
                None => match mg_2da::TwoDa::parse(&data, codepage) {
                    Err(e) => Content::Error(e.to_string()),
                    Ok(_) => Content::Error("the copy the game reads can't be read".into()),
                },
            }
        } else if crate::viewers::is_picture(key.restype) {
            Content::Picture(None)
        } else if crate::viewers::is_sound(key.restype) {
            let data: std::sync::Arc<[u8]> = data.as_slice().into();
            match mg_audio::Stream::open(data.clone()) {
                Ok(_) => Content::Sound(data),
                Err(e) => Content::Error(e.to_string()),
            }
        } else if key.restype == ResType::MDL {
            Content::Model(std::sync::OnceLock::new())
        } else if key.restype == ResType::NSS {
            // (Shown and copied with plain newlines, as the script editor
            // has them; Copy to Module and Save copy the bytes.)
            Content::Script(crate::text::to_editor(&crate::text::decode_script(&data)).0.into())
        } else if is_text(key.restype, &data) {
            Content::Text(crate::text::to_editor(&codepage.decode(&data)).0.into())
        } else if let Some(text) = model_text(key, &data, Some(rm)) {
            Content::Text(text.into())
        } else {
            Content::Binary(hex(&data[..data.len().min(512)]).into())
        };
        Some(Viewed { data, origin, content, made: made(), bytes: Default::default() })
    }
}

/// A resource's bytes as something to read: a GFF's fields, text, or the
/// first bytes in hexadecimal.
#[derive(Debug)]
pub(crate) enum Plain {
    Gff(Box<crate::ro_gff::ReadGff>),
    Text(LongText),
    Error(String),
}

/// What `data`, a resource of `key`'s type, shows as.
/// (`rm`: where a body part's skeleton is found, for a model as text.)
pub(crate) fn plain(key: ResKey, data: &[u8], rm: Option<&mg_resman::ResMan>) -> Plain {
    if key.restype.is_gff() {
        match Gff::read(data) {
            Ok(g) => Plain::Gff(Box::new(g.into())),
            Err(e) => Plain::Error(e.to_string()),
        }
    } else if is_text(key.restype, data) {
        // (Other text UTF-8 where it is: Moonglow and NUI write JUI so,
        // and LUA, MTR and TXT files are often saved so. Else the game's
        // codepage.)
        let decoded = if key.restype == ResType::NSS {
            crate::text::decode_script(data)
        } else {
            std::str::from_utf8(data).map_or_else(|_| crate::text::decode(data), str::to_owned)
        };
        Plain::Text(crate::text::to_editor(&decoded).0.into())
    } else if let Some(text) = model_text(key, data, rm) {
        Plain::Text(text.into())
    } else {
        Plain::Text(hex(&data[..data.len().min(512)]).into())
    }
}

/// A compiled model as the text it compiles from (`mg_mdl::decompile`);
/// nothing for anything else, or a model that can't be read.
pub(crate) fn model_text(
    key: ResKey,
    data: &[u8],
    rm: Option<&mg_resman::ResMan>,
) -> Option<String> {
    // (A body part bound to its skeleton: its bones named from it.)
    let skeleton = |n: &str| rm?.get_named(n, ResType::MDL).ok().map(|d| d.into_owned());
    (key.restype == ResType::MDL).then(|| mg_mdl::decompile_with(data, &skeleton).ok()).flatten()
}

/// Draws a [`Plain`] view, scrolling, its rows or lines in sight alone
/// (a GFF's: the read-only view's three columns, `ro_gff`).
pub(crate) fn plain_ui(ui: &mut Ui, key: ResKey, view: &Plain, names: crate::ro_gff::Names<'_>) {
    match view {
        // (A text by its lines in sight: a compiled model's is megabytes.)
        Plain::Text(text) => text.ui(ui, ("plain", key), None),
        Plain::Gff(g) => crate::ro_gff::ui(ui, g, names, ("plain", key)),
        Plain::Error(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
    }
}

/// The read-only tab of `key` (the copy the game reads), or of `layer`'s
/// copy.
fn read_only_tab(key: ResKey, layer: Option<&str>) -> Tab {
    match layer {
        None => Tab::Resource(key),
        Some(layer) => Tab::ResourceCopy { key, layer: layer.to_string() },
    }
}

/// The key and layer of a read-only tab.
fn shown_at(tab: &Tab) -> Option<(ResKey, Option<String>)> {
    match tab {
        Tab::Resource(key) => Some((*key, None)),
        Tab::ResourceCopy { key, layer } => Some((*key, Some(layer.clone()))),
        _ => None,
    }
}

/// A read-only tab's text with its Find bar and marked line: the keys are
/// its own while its tab is the one in front of the focused pane (a
/// window's, or the main window's) and no list has them.
fn text_ui(
    app: &mut Moonglow,
    ui: &mut Ui,
    text: &LongText,
    salt: impl std::hash::Hash + std::fmt::Debug,
    palette: Option<&crate::script_view::Palette>,
    tab: &Tab,
) {
    // (As the frame began: the dock is the editor area's while it draws.)
    let active = crate::table_view::active_tab(app);
    let here = active.as_ref() == Some(tab);
    // (Whether a read-only text's tab is in front of the focused pane:
    // then it alone takes the keys, wherever the pointer is.)
    let any_here = (active.as_ref().and_then(shown_at))
        .is_some_and(|at| app.browser.finds.get(&at).is_some_and(|f| f.shown));
    let Some(at) = shown_at(tab) else { return };
    let Moonglow { browser, keymap, panes_tabs, .. } = app;
    // (The finds of tabs closed let go of: not one just sent to a line,
    // whose tab opens. By the tabs as the frame began: the dock is the
    // editor area's while it draws, and empty here.)
    browser.finds.retain(|(k, l), f| {
        f.reveal.is_some()
            || (k, l) == (&at.0, &at.1)
            || panes_tabs.iter().flatten().any(|t| *t == read_only_tab(*k, l.as_deref()))
    });
    let find = browser.finds.entry(at).or_default();
    find.shown = true;
    text.ui_finding(ui, salt, palette, find, (keymap, here, any_here));
}

impl Browser {
    /// A read-only tab sent to a line (from 0): marked and brought into
    /// sight when next drawn, open or not yet.
    pub(crate) fn go_to_line(&mut self, key: ResKey, layer: Option<&str>, line: usize) {
        let find = self.finds.entry((key, layer.map(str::to_string))).or_default();
        find.go_to_line(line);
    }
}

/// The read-only text in front, whose Find the Command Palette's Find
/// opens: its key and layer.
fn text_in_front(app: &Moonglow) -> Option<(ResKey, Option<String>)> {
    let at = shown_at(&app.front_tab()?)?;
    app.browser.finds.get(&at).is_some_and(|f| f.shown).then_some(at)
}

/// The key commands of a read-only text in front that the Command Palette
/// lists (Options › Keyboard's Script group): Find, as its Ctrl+F.
pub(crate) fn palette_commands(app: &Moonglow) -> Vec<(crate::keys::Cmd, bool)> {
    match text_in_front(app) {
        Some(_) => vec![(crate::keys::Cmd::Find, true)],
        None => Vec::new(),
    }
}

/// Runs a command of [`palette_commands`] on the read-only text in front.
pub(crate) fn run_palette_command(app: &mut Moonglow, cmd: crate::keys::Cmd) {
    if cmd != crate::keys::Cmd::Find {
        return;
    }
    if let Some(at) = text_in_front(app)
        && let Some(f) = app.browser.finds.get_mut(&at)
    {
        f.open();
    }
}

/// Whether a read-only tab shows a text (its Find's keys are hinted in the
/// Status Bar).
pub(crate) fn shows_text(app: &Moonglow, key: ResKey, layer: Option<&str>) -> bool {
    app.browser.finds.get(&(key, layer.map(str::to_string))).is_some_and(|f| f.shown)
}

impl Moonglow {
    /// A read-only tab's Find, for tests: the matches found, whether all is
    /// looked through, the line of the match chosen and the marked line.
    #[doc(hidden)]
    pub fn read_only_find_for_test(
        &self,
        key: ResKey,
        layer: Option<&str>,
    ) -> Option<(usize, bool, Option<usize>, Option<usize>)> {
        Some(self.browser.finds.get(&(key, layer.map(str::to_string)))?.state())
    }

    /// Where a read-only tab's text was drawn last, for tests.
    #[doc(hidden)]
    pub fn read_only_view_rect_for_test(&self, key: ResKey) -> Option<egui::Rect> {
        self.browser.finds.get(&(key, None))?.view_rect
    }

    /// Whether a read-only tab's Find bar is open, for tests.
    #[doc(hidden)]
    pub fn read_only_find_open_for_test(&self, key: ResKey, layer: Option<&str>) -> bool {
        self.browser.finds.get(&(key, layer.map(str::to_string))).is_some_and(|f| f.open)
    }
}

/// Read-only view of a resource from the load order: the copy the game
/// reads, or (`layer`, by its label) the copy that layer has, which a
/// higher one overrides.
pub(crate) fn resource_ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey, layer: Option<&str>) {
    let Some(game) = app.game.clone() else { return };
    let game = &game;
    let at = (key, layer.map(str::to_string));
    let viewed = match app.viewed.get(&at) {
        Some(v) => v.clone(),
        None => match Viewed::load(game, key, layer) {
            Some(mut v) => {
                // (A picture decoded off the frame, a PLT from the game's
                // palettes; a sound the game is silent on said, once: as
                // the hak editor's Preview does, M5's W6.)
                if let Content::Picture(p) = &mut v.content {
                    let data: std::sync::Arc<[u8]> = v.data.as_slice().into();
                    *p = Some(crate::viewers::Picture::decode(app, ui.ctx(), key, data));
                    app.browser.pictures.insert(at.clone(), ui.input(|i| i.time));
                }
                if let Content::Sound(data) = &v.content {
                    app.warn_silent(crate::audio::Role::Viewed, key.resref, key.restype, data);
                }
                let v = std::sync::Arc::new(v);
                app.viewed.insert(at, v.clone());
                v
            }
            None => {
                match layer {
                    Some(layer) => ui.label(format!("{layer} has no {key} now.")),
                    None => ui.label(format!("{key} is not in the load order.")),
                };
                return;
            }
        },
    };
    // (A copy's: what overrides it, which the game reads.)
    let over = layer.and_then(|_| game.resman.origin(&key)).map(str::to_string);
    // (A picture, a sound or a model: its first bytes, or a model's text,
    // in place of it by the header's More, M5's W6.)
    let toggle = match &viewed.content {
        Content::Picture(_) | Content::Sound(_) => Some(crate::viewers::SHOW_BYTES),
        Content::Model(_) => Some(crate::viewers::SHOW_TEXT),
        _ => None,
    };
    let shown_at = (key, layer.map(str::to_string));
    let mut bytes = app.browser.bytes_shown.contains(&shown_at);
    let was = bytes;
    let (copy, save, edit_table) = resource_header(
        app,
        ui,
        (key, layer),
        &viewed.origin,
        toggle.map(|name| (name, &mut bytes)),
    );
    if bytes != was {
        match bytes {
            true => app.browser.bytes_shown.insert(shown_at),
            false => app.browser.bytes_shown.remove(&shown_at),
        };
    }
    if let Some(over) = &over {
        egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 4)).show(ui, |ui| {
            ui.weak(format!("Overridden by {over}, whose copy the game reads."));
        });
    }
    if let Some(home) = edit_table {
        open_table(app, key, home);
    }
    if copy {
        app.actions.push(Action::Apply(Command::new(
            format!("Copy {key} to the module"),
            vec![Edit::SetResource { key, data: Some(viewed.data.clone()) }],
        )));
        if let Some(tab) = Tab::for_resource(key) {
            app.actions.push(Action::OpenTab(tab));
        }
    }
    if save
        && let Some(path) =
            app.dialogs.save_file(FileKind::Any, Some(&PathBuf::from(key.to_string())))
    {
        match mg_module::write_replacing(&path, &viewed.data) {
            Ok(()) => app.log.info(format!("Saved {key} to {}", path.display())),
            Err(e) => app.log.error(format!("Could not write {}: {e}", path.display())),
        }
    }
    if let Content::TwoDa(l) = &viewed.content {
        let tab = match layer {
            None => Tab::Resource(key),
            Some(layer) => Tab::ResourceCopy { key, layer: layer.to_string() },
        };
        two_da(app, ui, key, layer, (l, viewed.made), &tab);
        return;
    }
    // A text by its lines in sight, a script's in the editor's colors.
    let salt = ("resource-text", key, layer);
    // A picture, a sound, a model: the viewers the hak editor's Preview
    // shows them with (M5's W6), or their bytes.
    let tab = match layer {
        None => Tab::Resource(key),
        Some(layer) => Tab::ResourceCopy { key, layer: layer.to_string() },
    };
    // (A picture is kept while its tab is drawn, its bytes shown or not:
    // let go of under Show Bytes, it was read and decoded anew each
    // `UNSEEN_AFTER`, its work never done.)
    if matches!(viewed.content, Content::Picture(_)) {
        app.browser.pictures.insert((key, layer.map(str::to_string)), ui.input(|i| i.time));
    }
    match &viewed.content {
        Content::Picture(_) | Content::Sound(_) if bytes => {
            return text_ui(app, ui, viewed.bytes(), salt, None, &tab);
        }
        Content::Picture(Some(picture)) => {
            let room = ui.available_rect_before_wrap();
            picture.ui(ui, key, room);
            ui.allocate_rect(room, egui::Sense::hover());
            return;
        }
        Content::Sound(data) => {
            let room = ui.available_rect_before_wrap();
            let by = crate::viewers::Player::Tab(tab);
            crate::viewers::sound_ui(app, ui, room, by, key, data);
            ui.allocate_rect(room, egui::Sense::hover());
            return;
        }
        Content::Model(text) if bytes => {
            let text = text.get_or_init(|| {
                let rm = Some(&game.resman);
                model_text(key, &viewed.data, rm)
                    .or_else(|| {
                        is_text(key.restype, &viewed.data)
                            .then(|| game.codepage().decode(&viewed.data).into_owned())
                    })
                    .unwrap_or_else(|| hex(&viewed.data[..viewed.data.len().min(512)]))
                    .into()
            });
            return text_ui(app, ui, text, salt, None, &tab);
        }
        Content::Model(_) => {
            use crate::model_view::{Shown, Source};
            let source = match layer {
                None => Source::Resource(key),
                Some(layer) => Source::Copy { key, layer: layer.to_string() },
            };
            // (Paused at first, as the hak editor's Preview's.)
            let shown = Shown { info: true, pop_out: false, restore: false, paused: true };
            crate::model_view::panel(app, ui, source, shown);
            return;
        }
        _ => {}
    }
    match &viewed.content {
        Content::Script(text) => {
            let palette = crate::script_view::Palette::for_ui(&app.settings.script_style, ui);
            return text_ui(app, ui, text, salt, Some(&palette), &tab);
        }
        Content::Text(text) | Content::Binary(text) => {
            return text_ui(app, ui, text, salt, None, &tab);
        }
        _ => {}
    }
    match &viewed.content {
        Content::Gff(g) => {
            let names = crate::ro_gff::Names { game: app.game.as_deref(), factions: &[] };
            crate::ro_gff::ui(ui, g, names, ("resource", key, layer));
        }
        Content::Error(e) => {
            egui::Frame::new().inner_margin(10).show(ui, |ui| {
                ui.colored_label(ui.visuals().error_fg_color, e);
            });
        }
        _ => {}
    }
}

/// The read-only tab's header's name (its record in `mg_uikit::header`).
pub const RESOURCE_HEADER: &str = "Resource header";

/// Where a resource of the load order is from, as the read-only tab's
/// note says it (the Resources board's "Read-only: from the game's data"):
/// by its layer's label.
pub(crate) fn from_words(label: &str) -> String {
    if label.starts_with("key:") {
        return "the game's data".into();
    }
    if let Some(hak) = label.strip_prefix("hak:") {
        return format!("{hak}.hak");
    }
    match label {
        "override" | "development" => format!("the {label} folder"),
        "module" => "the module".into(),
        l if l.starts_with("lang:") => "the game's language data".into(),
        l => format!("the game's {l}"),
    }
}

/// The read-only tab's header (the Resources board's): a lock and where
/// the resource is from, its kind, and at the right Copy to Module and
/// Save As… (a table's Edit… menu, or a hak's copy's Edit in its hak,
/// before them). What was pressed: Copy to Module, Save As…, a table's
/// home to edit it in.
fn resource_header(
    app: &Moonglow,
    ui: &mut Ui,
    (key, layer): (ResKey, Option<&str>),
    origin: &str,
    bytes: Option<(&str, &mut bool)>,
) -> (bool, bool, Option<EditTable>) {
    use mg_uikit::header::{EditorHeader, Item};
    let t = mg_uikit::theme::tokens(ui.ctx());
    let from = format!("Read-only: from {}", from_words(origin));
    let kind = format!("{} file", key.restype.extension().unwrap_or("?").to_uppercase());
    let body = egui::TextStyle::Body.resolve(ui.style());
    let wide = |s: &str| {
        ui.painter().layout_no_wrap(s.to_string(), body.clone(), t.text).size().x.ceil() + 2.0
    };
    let (from_w, kind_w) = (wide(&from) + 22.0, wide(&kind));
    // (Which it is, where it is read from and its size, as the Status Bar
    // says them: also here, for a window whose bar shows another's.)
    // (One sentence, the style guide's: Copy to Module's tip says the rest.)
    let from_tip = resource_count(app, key, layer).map_or_else(
        || "Shown as the game reads it".to_string(),
        |said| format!("{said}, shown as the game reads it"),
    );
    let note = |ui: &mut Ui, whole: bool| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (rect, r) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        mg_uikit::Icon::Lock.paint(ui.painter(), rect, t.text_3);
        if whole {
            ui.add(egui::Label::new(egui::RichText::new(&from).color(t.text_2)).selectable(false));
        } else {
            // (The lock alone: named by the words it stands for.)
            r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, &from));
        }
        // (Its tip over the lock and the words: the kit's custom items
        // leave their tips to their callers.)
        let all = ui.min_rect();
        ui.interact(all, ui.id().with("read-only-from"), egui::Sense::hover())
            .on_hover_text(&from_tip);
    };
    let (in_module, has_module) = match &app.ws {
        Some(w) => (w.module.contains(&key), true),
        None => (false, false),
    };
    // (A module kept as files names each by its kind's extension: a kind
    // with none would be written as no file it reads back, lost.)
    let loose = app.ws.as_ref().is_some_and(|w| {
        use mg_module::ModuleLocation as L;
        matches!(w.module.location, Some(L::Folder(_) | L::Project { .. }))
    });
    let no_file = loose && key.restype.extension().is_none();
    let copy_tip = match (has_module, in_module) {
        (false, _) => format!("Open a module to copy {key} into it"),
        (true, true) => format!("The module has its own {key}"),
        (true, false) if no_file => {
            "A module kept as files cannot hold this kind, which has no file extension".to_string()
        }
        (true, false) => "Put a copy in the module, where it can be edited".to_string(),
    };
    let can_copy = has_module && !in_module && !no_file;
    let table = std::cell::RefCell::new(None);
    let mut h = EditorHeader::new(RESOURCE_HEADER)
        .left(
            Item::custom(from.clone(), from_w, |ui| note(ui, true))
                .compact(18.0, |ui| note(ui, false)),
        )
        .left(Item::separator())
        .left(Item::custom(kind.clone(), kind_w, |ui| {
            ui.add(egui::Label::new(egui::RichText::new(&kind).color(t.text_3)).selectable(false));
        }));
    // (Always in More: a check row, the viewer shown while it is off.)
    if let Some((name, on)) = bytes {
        let tip = match name {
            crate::viewers::SHOW_TEXT => "Show the model's text in place of its drawing",
            _ => "Show its first bytes in place of it",
        };
        h = h.right(Item::toggle(None, name, on).tip(tip).in_more());
    }
    if key.restype == ResType::TWODA {
        match layer {
            None => {
                h = h.right(Item::menu(None, "Edit…", |ui| {
                    mg_uikit::menu_rows(ui);
                    *table.borrow_mut() = table_homes(app, ui, key);
                }));
            }
            // A copy a hak of the module has: edited in that hak.
            Some(layer) => {
                if let Some(name) = layer.strip_prefix("hak:")
                    && let Some((_, path)) =
                        haks_with(app, key).into_iter().find(|(n, _)| n == name)
                {
                    let words = format!("Edit in {name}.hak");
                    let tip =
                        "Opens the hak in the hak editor if needed, and saves the table into it";
                    *table.borrow_mut() = None;
                    h = h.right(Item::words(words.clone()).tip(tip));
                    let out_name = words;
                    let out = h
                        .right(Item::words("Copy to Module").enabled(can_copy).tip(copy_tip))
                        .right(Item::words("Save As…").tip("Save the resource to a file, as it is"))
                        .show(ui);
                    let edit = out.clicked(&out_name).then_some(EditTable::InHak(path));
                    return (out.clicked("Copy to Module"), out.clicked("Save As…"), edit);
                }
            }
        }
    }
    let out = h
        .right(Item::words("Copy to Module").enabled(can_copy).tip(copy_tip))
        .right(Item::words("Save As…").tip("Save the resource to a file, as it is"))
        .show(ui);
    let table = table.into_inner();
    (out.clicked("Copy to Module"), out.clicked("Save As…"), table)
}

/// The types shown as text whatever they hold (others are where their
/// bytes are text: [`is_text`]).
pub(crate) const TEXT_TYPES: [ResType; 9] = [
    ResType::NSS,
    ResType::JUI,
    ResType::TXT,
    ResType::INI,
    ResType::SET,
    ResType::TXI,
    ResType::MTR,
    ResType::SHD,
    ResType::LUA,
];

fn is_text(t: ResType, data: &[u8]) -> bool {
    TEXT_TYPES.contains(&t)
        || (!data.is_empty()
            && data.iter().take(4096).all(|b| b.is_ascii_graphic() || b.is_ascii_whitespace()))
}

fn hex(data: &[u8]) -> String {
    data.chunks(16)
        .enumerate()
        .map(|(i, row)| {
            let bytes: Vec<String> = row.iter().map(|b| format!("{b:02x}")).collect();
            let text: String =
                row.iter().map(|&b| if b.is_ascii_graphic() { b as char } else { '.' }).collect();
            format!("{:08x}  {:<47}  {text}", i * 16, bytes.join(" "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn value_text(v: &Value) -> String {
    if let Some(number) = v.number_text() {
        return number;
    }
    match v {
        Value::String(b) | Value::ResRef(b) => format!("{:?}", decode(b)),
        Value::LocString(ls) => {
            let mut parts: Vec<String> = Vec::new();
            if !ls.strref.is_none() {
                parts.push(format!("StrRef {}", ls.strref.0));
            }
            parts.extend(ls.strings.iter().map(|(k, b)| format!("[{}] {:?}", k.0, decode(b))));
            parts.join(", ")
        }
        Value::Void(b) => format!("{} bytes", b.len()),
        _ => String::new(),
    }
}

/// The tab that drew the read-only view of a table whose grid's id is
/// `id` last: its resource tab, a layer's copy's, or a hak editor's.
pub(crate) fn view_tab(app: &Moonglow, id: u32) -> Option<Tab> {
    app.browser.table_views.values().find(|v| v.doc.id == id)?.tab.clone()
}

/// What the Status Bar shows beside a table only looked at, the resource
/// tab's view of `key` (or of a layer's `copy` of it, which a higher layer
/// overrides; M4's U16): its rows and columns, whose copy it is
/// (the layer the game reads it from: the game's, a hak's, the module's)
/// and that it is read-only, and the keys it has
/// (`table_view::panels::shown_read_only`). `None` before it is drawn.
pub(crate) fn table_shown(
    app: &Moonglow,
    k: &crate::shell::HintKeys<'_>,
    ctx: &egui::Context,
    key: ResKey,
    copy: Option<&str>,
) -> Option<crate::shell::Shown> {
    let view = app.browser.table_views.get(&(key, copy.map(str::to_string)))?;
    // (A layer's copy, a hak's under a higher one: that layer's.)
    let layer = copy.map(str::to_string).or_else(|| {
        let g = app.game.as_deref()?;
        let at = *g.resman.copies(&key).first()?;
        Some(g.resman.layers()[at].label.clone())
    });
    let whose = match layer {
        Some(layer) => format!("from {layer}, read-only"),
        None => "read-only".to_string(),
    };
    let has = view
        .tab
        .as_ref()
        .is_some_and(|tab| crate::table_view::grid_has_keys(app, ctx, view.doc.id, tab));
    Some(crate::table_view::panels::shown_read_only(k, ctx, (&view.doc, has), &whose))
}

/// A 2DA as a table; only the rows in view are laid out. With copies of
/// it in several layers (haks over the game's), each row says which layer
/// it comes from, and the cells a layer changed are marked, with what they
/// were below. Cells of StrRef columns show their text. It is the 2DA
/// editor's grid with the editing left out: the column names stay in
/// sight over the rows and the rows' numbers beside them, a column is as
/// wide as the edge of its name was dragged to (here or there: the widths
/// are the table's), a cell is selected by a click and the arrows, Find
/// narrows the rows, Ctrl+C copies, and a column can be hidden.
pub(crate) fn two_da(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    layer: Option<&str>,
    (l, of): (&mg_module::table_layers::Layered, u64),
    tab: &Tab,
) {
    use crate::table_view::{GridOf, TableDoc, grid_in, keys_in, view_header};
    let name = key.resref.to_string();
    let kinds = crate::table_view::kinds(app, &name);
    let Moonglow { browser, .. } = app;
    let t = l.table();
    let layered = l.copies.len() > 1;
    ui.horizontal(|ui| {
        ui.check_box(&mut browser.raw_strrefs, "StrRefs as numbers")
            .on_hover_text("Show StrRef columns as numbers, not their text");
        if layered {
            let label = |i: Option<usize>| match i {
                None => "All rows".to_string(),
                Some(i) => format!("Rows from {}", l.copies[i].0),
            };
            browser.rows_from = browser.rows_from.filter(|&i| i < l.copies.len());
            let options: Vec<Option<usize>> =
                std::iter::once(None).chain((0..l.copies.len()).map(Some)).collect();
            crate::widgets::dropdown("2da-rows-from").selected_text(label(browser.rows_from)).show(
                ui,
                &mut browser.rows_from,
                &options,
                |i| label(*i),
            );
        }
    });
    if layered {
        let parts: Vec<String> = l
            .summary()
            .iter()
            .rev()
            .map(|(label, added, changed)| match (added, changed) {
                (a, 0) => format!("{label}: {a} rows"),
                (0, c) => format!("{label}: changes {c}"),
                (a, c) => format!("{label}: adds {a}, changes {c}"),
            })
            .collect();
        ui.weak(format!("{} copies, lowest first. {}", l.copies.len(), parts.join("; ")));
    }
    // The view, kept while it is of these layers (`of`: which reading of
    // them, [`made`]).
    let at = (key, layer.map(str::to_string));
    let kept = browser.table_views.get(&at).is_some_and(|v| {
        let had = v.doc.edit.table();
        v.of == of && had.len() == t.len() && had.columns() == t.columns()
    });
    browser.table_view_use += 1;
    let used = browser.table_view_use;
    if !kept {
        browser.evict_lru_table_view();
        browser.made_views += 1;
        let id = u32::MAX - browser.made_views;
        let doc = TableDoc::read_only(id, key, t.clone());
        browser
            .table_views
            .insert(at.clone(), TableView { of, rows_from: None, doc, tab: None, last_used: used });
    }
    let rows_from = browser.rows_from.filter(|_| layered);
    let strrefs = !browser.raw_strrefs;
    let view = browser.table_views.get_mut(&at).expect("made above");
    view.last_used = used;
    if view.rows_from != rows_from {
        view.rows_from = rows_from;
        view.doc.within = rows_from
            .map(|o| std::sync::Arc::new((0..t.len()).filter(|&r| l.row_origin(r) == o).collect()));
    }
    view.tab = Some(tab.clone());
    let id = view.doc.id;
    let has = crate::table_view::grid_has_keys(app, ui.ctx(), id, tab);
    let Moonglow { browser, game, settings, talk, talk_view, .. } = app;
    let view = browser.table_views.get_mut(&at).expect("made above");
    let doc = &mut view.doc;
    doc.strref_text = strrefs;
    view_header(ui, doc);
    keys_in(doc, ui, true, has);
    let talk = talk.as_ref().filter(|_| !talk_view.outside);
    let of = GridOf { settings, table_name: &name, talk, game: game.as_deref(), has_keys: has };
    grid_in(ui, doc, of, &kinds, &[], Some(l));
    crate::table_view::focus_asked(app, ui.ctx(), id, tab);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Find in a read-only text looks a slice of lines at a time: with no
    /// time to spare, one slice a call, and on until all is looked
    /// through, what is found the same as at one go.
    #[test]
    fn a_read_only_text_is_searched_a_slice_at_a_time() {
        let text: String = (0..1000).map(|n| format!("line {n} word\n")).collect();
        let text = LongText::from(text);
        let mut f = TextFind { find: "WORD".into(), ..Default::default() };
        assert!(f.sweep(&text, std::time::Duration::ZERO), "more to look through");
        let found = f.found.as_ref().unwrap();
        assert_eq!(found.next, Some(FIND_SLICE), "one slice");
        assert_eq!(found.matches.len(), FIND_SLICE);
        let mut calls = 1;
        while f.sweep(&text, std::time::Duration::ZERO) {
            calls += 1;
        }
        assert_eq!(calls + 1, 1001_usize.div_ceil(FIND_SLICE), "a slice a call");
        assert_eq!(f.state(), (1000, true, Some(0), None));
        let mut once = TextFind { find: "WORD".into(), ..Default::default() };
        assert!(!once.sweep(&text, std::time::Duration::from_secs(60)));
        assert_eq!(once.found.unwrap().matches, f.found.unwrap().matches);
        // Each line's matches as the script editor's Find finds them.
        let o = |match_case, whole_word| crate::script_tools::SearchOptions {
            match_case,
            whole_word,
            backwards: false,
        };
        for (line, needle) in [
            ("aaaa", "aa"),
            ("Árbol arbol ARBOL", "arbol"),
            ("foo food _foo foo", "foo"),
            ("x", "xy"),
        ] {
            for opts in [o(false, false), o(true, false), o(false, true), o(true, true)] {
                let mut by_find = Vec::new();
                let mut from = 0;
                while let Some((a, b)) = crate::script_tools::find(line, needle, from, &opts) {
                    if a < from {
                        break;
                    }
                    by_find.push((a, b));
                    from = b;
                }
                assert_eq!(line_matches(line, needle, &opts), by_find, "{line} {needle} {opts:?}");
            }
        }
        // Whole words, and case, as the script editor's Find has them.
        let mut whole = TextFind { find: "wor".into(), ..Default::default() };
        whole.options.whole_word = true;
        whole.sweep(&text, std::time::Duration::from_secs(60));
        assert_eq!(whole.state().0, 0);
    }

    /// A row's name and layer share its room, the name first (the
    /// owner's ruling on the review of M3 T4): both whole where they fit;
    /// else the layer takes what the whole name leaves, at most 45% of the
    /// room, and is left out where that is less than a stub of it (30
    /// here), the name then given the room; never more than the room.
    #[test]
    fn a_row_s_name_and_layer_share_the_room() {
        let stub = 30.0;
        assert_eq!(share(200.0, 100.0, 60.0, stub), (100.0, Some(60.0)));
        assert_eq!(share(200.0, 80.0, 250.0, stub), (110.0, Some(90.0)), "the layer to 45%");
        assert_eq!(share(200.0, 150.0, 150.0, stub), (150.0, Some(50.0)), "what the name leaves");
        assert_eq!(share(200.0, 180.0, 60.0, stub), (200.0, None), "less than a stub: no layer");
        assert_eq!(share(200.0, 250.0, 60.0, stub), (200.0, None), "a name past the row");
        assert_eq!(share(200.0, 170.0, 40.0, stub), (170.0, Some(30.0)), "a stub of the layer");
        assert_eq!(share(-10.0, 50.0, 50.0, stub), (0.0, None));
        for (room, name, layer) in [(120.0, 300.0, 40.0), (120.0, 30.0, 300.0), (90.0, 90.0, 90.0)]
        {
            let (n, l) = share(room, name, layer, stub);
            let l = l.unwrap_or(0.0);
            assert!(n + l <= room + 0.001 && l <= layer, "{room} {name} {layer}");
            assert!(l <= room * LAYER_SHARE + 0.001, "{room} {name} {layer}");
        }
    }

    /// A name cut in its middle keeps its start, its end and its
    /// extension, and as many characters as asked: the names that read
    /// alike cut at their ends (the review's `nw_ch_srch…`) read apart.
    #[test]
    fn a_name_is_cut_in_its_middle_with_its_extension() {
        assert_eq!(middle_cut("nw_ch_srch_off.ncs", 14).as_deref(), Some("nw_ch_s…off.ncs"));
        assert_eq!(middle_cut("nw_ch_srch_off.ncs", 18), None, "it fits");
        assert_eq!(middle_cut("portraits.2da", 10).as_deref(), Some("portr…s.2da"));
        let names = ["nw_ch_srch_hb.ncs", "nw_ch_srch_hb2.ncs", "nw_ch_srch_off.ncs"];
        let cut: Vec<String> = names.iter().map(|n| middle_cut(n, 12).unwrap()).collect();
        assert!(cut[0] != cut[1] && cut[1] != cut[2], "{cut:?}");
        for c in &cut {
            assert_eq!(c.chars().count(), 13, "{c}: twelve and the ellipsis");
            assert!(c.ends_with(".ncs") && c.starts_with("nw_ch"), "{c}");
        }
        // No room for the extension and a character before it: cut at the
        // end instead.
        assert_eq!(middle_cut("portraits.2da", 4), None);
        assert_eq!(middle_cut("noextension", 6).as_deref(), Some("noe…ion"));
    }

    /// A long text's lines are the text's, and a script's lines colored
    /// are the whole script's highlighting cut at the line ends (a
    /// comment over several lines is a comment on each): what is drawn by
    /// the lines in sight is what the one label drew (the review of
    /// November 2026, D29). The colored lines are made once for a
    /// palette.
    #[test]
    fn a_long_text_s_lines_are_the_text_s() {
        let source = "int a = 1;\n/* over\n   two lines */\n\nvoid main() { a++; }";
        let text = LongText::from(source.to_string());
        let (spans, widest) = text.lines();
        let lines: Vec<&str> = spans.iter().map(|s| &source[s.clone()]).collect();
        assert_eq!(lines, source.split('\n').collect::<Vec<_>>());
        assert_eq!(*widest, "void main() { a++; }".len());
        let style = crate::settings::ScriptStyle::default();
        let palette = crate::script_view::Palette::new(&style, true, egui::Color32::WHITE);
        let jobs = text.colored(&palette);
        assert_eq!(jobs.len(), lines.len());
        assert_eq!(jobs.iter().map(|j| j.text.as_str()).collect::<Vec<_>>(), lines);
        // Each character's color, as the whole text's highlighting has it.
        let colors = |job: &egui::text::LayoutJob| -> Vec<(char, egui::Color32)> {
            (job.sections.iter())
                .flat_map(|s| {
                    let piece = &job.text[s.byte_range.start.0..s.byte_range.end.0];
                    piece.chars().map(|c| (c, s.format.color)).collect::<Vec<_>>()
                })
                .filter(|(c, _)| *c != '\n')
                .collect()
        };
        let whole = colors(&crate::script_view::highlight(source, &palette));
        let by_line: Vec<(char, egui::Color32)> = jobs.iter().flat_map(colors).collect();
        assert_eq!(by_line, whole);
        assert!(std::sync::Arc::ptr_eq(&jobs, &text.colored(&palette)), "made once");
        let light = crate::script_view::Palette::new(&style, false, egui::Color32::BLACK);
        assert!(!std::sync::Arc::ptr_eq(&jobs, &text.colored(&light)), "anew for other colors");
        // (An empty text is one empty line.)
        assert_eq!(LongText::from(String::new()).lines().0.len(), 1);
    }

    #[test]
    fn load_order_changed_clears_browser_table_views_and_pictures() {
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        let key = ResKey::parse("classes", ResType::TWODA).unwrap();
        let table = mg_2da::TwoDa::parse(b"2DA V2.0\n\nLABEL\n0 *\n", mg_core::Codepage::default())
            .unwrap();
        let doc = crate::table_view::TableDoc::read_only(u32::MAX, key, table);
        app.browser.table_views.insert(
            (key, None),
            TableView { of: 0, rows_from: None, doc, tab: None, last_used: 0 },
        );
        app.browser.pictures.insert((key, None), 1.0);
        app.browser.bytes_shown.insert((key, None));

        app.load_order_changed();
        assert!(app.browser.table_views.is_empty());
        assert!(app.browser.pictures.is_empty());
        assert!(!app.browser.bytes_shown.is_empty());
    }

    /// The custom talk table read again (its Save) is no load order
    /// change: a 2DA view in the browser keeps its Find (review U's 1,
    /// where every talk table Save emptied the views).
    #[test]
    fn a_talk_table_read_again_keeps_the_browser_s_2da_views() {
        use mg_resman::{LayerClass, MemContainer, ResMan, priority};
        let tlk_key = ResKey::parse("mg_t", ResType::TLK).unwrap();
        let mut tlk = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
        tlk.entries.push(mg_tlk::TlkEntry::text("Old"));
        let mut layer = MemContainer::new();
        layer.insert(tlk_key, tlk.to_bytes().unwrap());
        let mut rm = ResMan::new();
        rm.add(priority::HAK, "hak:t", LayerClass::Erf, layer);
        let game = mg_rules::GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut m = mg_module::Module::new();
        let mut ifo = mg_gff::Gff::new(*b"IFO ");
        ifo.root.set("Mod_CustomTlk", mg_gff::Value::String(b"mg_t".to_vec()));
        m.set_gff(ResKey::parse("module", ResType::IFO).unwrap(), &ifo).unwrap();
        let mut app = Moonglow::new(None, Box::new(crate::NoDialogs::default()));
        app.game = Some(std::sync::Arc::new(game));
        app.ws = Some(mg_edit::Workspace::new(m));

        let key = ResKey::parse("classes", ResType::TWODA).unwrap();
        let table = mg_2da::TwoDa::parse(b"2DA V2.0\n\nLABEL\n0 *\n", mg_core::Codepage::default())
            .unwrap();
        let mut doc = crate::table_view::TableDoc::read_only(u32::MAX, key, table);
        doc.filter = "wizard".into();
        app.browser.table_views.insert(
            (key, None),
            TableView { of: 0, rows_from: None, doc, tab: None, last_used: 0 },
        );
        app.reload_custom_tlk();
        assert!(app.game.as_deref().unwrap().custom_tlk().is_some(), "{:?}", app.log.entries);
        let view = app.browser.table_views.get(&(key, None)).expect("the view kept");
        assert_eq!(view.doc.filter, "wizard", "with its Find");
    }

    #[test]
    fn long_text_and_plain_decode_codepage_and_strip_carriage_returns() {
        let lt = LongText::from("ab\r\ncde\r\n".to_string());
        let (spans, widest) = lt.lines();
        assert_eq!(widest, &3);
        assert_eq!(&lt.text[spans[0].clone()], "ab");
        assert_eq!(&lt.text[spans[1].clone()], "cde");

        let key = ResKey::parse("sample", ResType::TXT).unwrap();
        let Plain::Text(p) = plain(key, b"\x93hi\x94\r\nnext", None) else {
            panic!("expected text");
        };
        assert_eq!(p.text, "\u{201c}hi\u{201d}\nnext");
        assert_eq!(p.lines().1, 4);
        // UTF-8 text shows as UTF-8 (a NUI window's JUI, a Lua file, a
        // material, a text file), not read through the codepage.
        for ext in ["jui", "lua", "txt", "mtr"] {
            let restype = ResType::from_extension(ext).expect(ext);
            let key = ResKey::parse("sample", restype).unwrap();
            let Plain::Text(p) = plain(key, "{\"title\": \"Café\"}".as_bytes(), None) else {
                panic!("expected text for {ext}");
            };
            assert_eq!(p.text, "{\"title\": \"Café\"}", "{ext}");
        }
    }

    #[test]
    fn browser_table_view_cache_evicts_only_lru_entry() {
        let mut browser = Browser::new();
        let table = mg_2da::TwoDa::parse(b"2DA V2.0\n\nLABEL\n0 *\n", mg_core::Codepage::default())
            .unwrap();
        for i in 0..VIEWS_KEPT {
            let key = ResKey::parse(&format!("tbl_{i:02}"), ResType::TWODA).unwrap();
            let doc =
                crate::table_view::TableDoc::read_only(u32::MAX - i as u32, key, table.clone());
            browser.table_views.insert(
                (key, None),
                TableView { of: 0, rows_from: None, doc, tab: None, last_used: (i + 1) as u64 },
            );
        }
        browser.evict_lru_table_view();
        assert_eq!(browser.table_views.len(), VIEWS_KEPT - 1);
        let oldest = ResKey::parse("tbl_00", ResType::TWODA).unwrap();
        let recent = ResKey::parse("tbl_01", ResType::TWODA).unwrap();
        assert!(!browser.table_views.contains_key(&(oldest, None)));
        assert!(browser.table_views.contains_key(&(recent, None)));
    }
}
