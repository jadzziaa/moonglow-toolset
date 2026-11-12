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
use mg_gff::{Gff, Struct, Value};
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
    layer: String,
    over: Option<String>,
}

/// The browser's index and filters.
#[derive(Debug, Default)]
pub struct Browser {
    /// Every copy of each resource, the one the game reads first and those
    /// it overrides under it; rebuilt when stale.
    entries: Vec<Row>,
    /// How many resources there are (the copies the game reads).
    resources: usize,
    types: Vec<ResType>,
    pub restype: Option<ResType>,
    pub filter: String,
    /// The copies that others override are listed under them (a builder's
    /// report: of a table in two haks, only the higher one was found).
    pub show_copies: bool,
    /// Indices into `entries` matching the filters, for `query`.
    shown: Vec<usize>,
    /// How many of those are overridden copies.
    shown_copies: usize,
    query: Option<(Option<ResType>, String, bool)>,
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
    /// An Export as Files into a folder that has some of the files: asked
    /// about before any is written over.
    pub(crate) exporting: Option<Exporting>,
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
}

/// How many 2DA views are kept: more are made anew.
const VIEWS_KEPT: usize = 16;

impl Browser {
    pub fn new() -> Browser {
        Browser { stale: true, show_copies: true, ..Default::default() }
    }

    fn refresh(&mut self, app_resman: Option<&mg_resman::ResMan>) {
        if !self.stale {
            return;
        }
        self.stale = false;
        self.query = None;
        self.entries = match app_resman {
            Some(rm) => {
                let label = |i: usize| rm.layers()[i].label.clone();
                rm.every_copy()
                    .into_iter()
                    .flat_map(|(key, layers)| {
                        let top = label(layers[0]);
                        layers.into_iter().enumerate().map(move |(n, i)| Row {
                            key,
                            layer: label(i),
                            over: (n > 0).then(|| top.clone()),
                        })
                    })
                    .collect()
            }
            None => Vec::new(),
        };
        self.resources = self.entries.iter().filter(|r| r.over.is_none()).count();
        let mut types: Vec<ResType> = self.entries.iter().map(|r| r.key.restype).collect();
        types.sort_by_key(|t| t.extension().unwrap_or_default());
        types.dedup();
        self.types = types;
    }

    /// The keys at the row `at` of those listed, the list gone to it.
    pub(crate) fn go_to(&mut self, at: usize) {
        self.cursor = Some(at);
        self.reveal = true;
    }

    fn filtered(&mut self) -> &[usize] {
        let needle = mg_core::text::Needle::new(&self.filter);
        let query = (self.restype, needle.as_str().to_string(), self.show_copies);
        if self.query.as_ref() != Some(&query) {
            self.shown = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    (query.2 || r.over.is_none())
                        && query.0.is_none_or(|t| r.key.restype == t)
                        && needle.finds(&r.key.resref.to_string())
                })
                .map(|(i, _)| i)
                .collect();
            self.shown_copies =
                self.shown.iter().filter(|&&i| self.entries[i].over.is_some()).count();
            self.query = Some(query);
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
    browser.refresh(game.as_ref().map(|g| &g.resman));
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
        let keys = (browser.shown.iter().map(|&i| &browser.entries[i]))
            .map(|r| (r.key, r.over.as_ref().map(|_| r.layer.clone())))
            .collect();
        app.actions.push(Action::SaveResources(keys));
    }
}

/// A resource's kind as its row's icon shows it.
fn kind_icon(t: ResType) -> mg_uikit::Icon {
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
struct RowFonts {
    mono: egui::FontId,
    small: egui::FontId,
    char_width: f32,
    ellipsis: f32,
    stub: f32,
}

impl RowFonts {
    fn of(ui: &Ui) -> RowFonts {
        let mono = egui::FontId::monospace(NAME_SIZE);
        let small = egui::TextStyle::Small.resolve(ui.style());
        let width =
            |text: &str, font: &egui::FontId| cut(ui, text, font.clone(), f32::INFINITY).size().x;
        // (Ten, for a width that is the font's advance, not a glyph's.)
        let char_width = width("0000000000", &mono) / 10.0;
        let ellipsis = width("0…", &mono) - width("0", &mono);
        let stub = width(LAYER_STUB, &small);
        RowFonts { mono, small, char_width, ellipsis, stub }
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
    let layer = match &row.over {
        Some(over) => format!("{}, overridden by {over}", row.layer),
        None => row.layer.clone(),
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
    let id = ui.id().with(("browser-row", row.key, &row.layer));
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
            (fill, text, t.text_3, t.text_2)
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
    let Moonglow { browser, game, ws, actions, .. } = app;
    browser.refresh(game.as_ref().map(|g| &g.resman));
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
        let current = browser.restype.and_then(|t| t.extension()).unwrap_or("all types");
        let types: Vec<Option<ResType>> =
            std::iter::once(None).chain(browser.types.iter().copied().map(Some)).collect();
        crate::widgets::dropdown("browser-type")
            .selected_text(current)
            .width(width)
            .show(ui, &mut browser.restype, &types, |t| {
                t.map_or("all types", |t| t.extension().unwrap_or("?"))
            })
            .on_hover_text("Only the resources of one type");
        // The Name's keys, as the side panes' Filters have them (GitHub
        // issue 12): Find's (Ctrl+F) with the pointer over the panel go to
        // it, Escape clears it and then leaves it, Down goes into the list.
        let field_id = egui::Id::new("browser-filter-field");
        let over = ui.ctx().pointer_hover_pos().is_some_and(|p| pane.contains(p));
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
        if browser.shown.iter().any(|&i| browser.entries[i].key.restype == ResType::MDL) {
            ui.check_box(&mut browser.models_as_text, "Models as text").on_hover_text(
                "Export as Files writes compiled models as the text they compile from",
            );
        }
        // (The resources, the copies the game reads; and the copies they
        // override, listed under them. Cut short at the dock's width, the
        // whole in its tip; whole, the tip says how to open one.)
        let copies = match browser.shown_copies {
            0 => String::new(),
            1 => ", 1 overridden copy".into(),
            n => format!(", {n} overridden copies"),
        };
        let said = format!("{found} of {} resources{copies}", browser.resources);
        let body = egui::TextStyle::Body.resolve(ui.style());
        let galley = cut_in(ui, &said, body, ui.available_width(), t.text_3);
        let elided = galley.elided;
        let r = ui.add(egui::Label::new(galley).show_tooltip_when_elided(false).selectable(false));
        if elided {
            r.on_hover_text(&said);
        } else {
            r.on_hover_text("Double-click a resource or press Enter to open it");
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
                let copy = over.as_ref().map(|_| layer.clone());
                let tab = || match (&copy, Tab::for_resource(*k)) {
                    (Some(layer), _) => Tab::ResourceCopy { key: *k, layer: layer.clone() },
                    (None, Some(t @ Tab::Model(_))) => t,
                    (None, Some(t)) if in_module(k) => t,
                    _ => Tab::Resource(*k),
                };
                if r.clicked() {
                    clicked = Some(at);
                }
                if r.double_clicked() || (entered && cursor == Some(at)) {
                    actions.push(Action::OpenTab(tab()));
                }
                mg_uikit::context_menu(&r, |ui| {
                    if ui.button("Open").clicked() {
                        actions.push(Action::OpenTab(tab()));
                    }
                    // (A copy's as its layer has it: a model, a blueprint.)
                    let previewable = crate::model_view::previewable(k.restype)
                        && (copy.is_some() || k.restype != ResType::MDL);
                    if previewable
                        && ui.button("Preview").on_hover_text("See it in the 3D viewer").clicked()
                    {
                        let tab = match &copy {
                            Some(layer) => Tab::ModelCopy { key: *k, layer: layer.clone() },
                            None => Tab::Model(*k),
                        };
                        actions.push(Action::OpenTab(tab));
                    }
                    if ui.button("Save As…").on_hover_text("As a file, as it is").clicked() {
                        actions.push(Action::SaveResource(*k, copy.clone()));
                    }
                    if k.restype == ResType::MDL
                        && ui
                            .button("Save As Text…")
                            .on_hover_text("A compiled model as the text it compiles from")
                            .clicked()
                    {
                        actions.push(Action::SaveModelText(*k, copy.clone()));
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

/// A 2DA's Edit menu: the module's own table as it is, or a copy of the
/// one the game reads, in the module, in a hak open in the hak editor,
/// or in a file. (The game's own tables are never changed.)
fn table_menu(app: &Moonglow, ui: &mut Ui, key: ResKey) -> Option<EditTable> {
    mg_uikit::menu_button(ui, "Edit…", |ui| {
        mg_uikit::menu_rows(ui);
        table_homes(app, ui, key)
    })
    .inner
    .flatten()
}

/// The places a table can be edited in, as buttons ([`table_menu`]'s):
/// the one clicked.
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

/// A copy of a table that a hak of the module has (`layer`, `hak:name`),
/// where the hak is the builder's own: a button to edit it in the hak.
fn hak_table_button(app: &Moonglow, ui: &mut Ui, key: ResKey, layer: &str) -> Option<EditTable> {
    let name = layer.strip_prefix("hak:")?;
    let (_, path) = haks_with(app, key).into_iter().find(|(n, _)| n == name)?;
    let tip = "Opens the hak in the hak editor if needed, and saves the table into it";
    let edit = ui.button(format!("Edit in {name}.hak")).on_hover_text(tip).clicked();
    edit.then_some(EditTable::InHak(path))
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
    Gff(Gff),
    TwoDa(mg_module::table_layers::Layered),
    Script(LongText),
    Text(LongText),
    Binary(LongText),
    Error(String),
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
                spans.push(from..from + line.len());
                widest = widest.max(line.chars().count());
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
        let whole = crate::script_view::highlight(&self.text, palette);
        let mut jobs = vec![egui::text::LayoutJob::default()];
        for section in &whole.sections {
            let piece = &whole.text[section.byte_range.start.0..section.byte_range.end.0];
            for (at, part) in piece.split('\n').enumerate() {
                if at > 0 {
                    jobs.push(egui::text::LayoutJob::default());
                }
                if !part.is_empty() {
                    jobs.last_mut().expect("one").append(part, 0.0, section.format.clone());
                }
            }
        }
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
        salt: impl egui::AsIdSalt,
        palette: Option<&crate::script_view::Palette>,
    ) {
        let (spans, widest) = self.lines();
        let font = match palette {
            Some(p) => p.font(),
            None => egui::TextStyle::Monospace.resolve(ui.style()),
        };
        let (height, wide) = ui.fonts_mut(|f| (f.row_height(&font), f.glyph_width(&font, 'm')));
        let jobs = palette.map(|p| self.colored(p));
        ui.scope(|ui| {
            // (A line under the last, as one text's lines are.)
            ui.spacing_mut().item_spacing.y = 0.0;
            let rows = egui::ScrollArea::both().id_salt(salt).auto_shrink([false, false]);
            rows.show_rows(ui, height, spans.len(), |ui, range| {
                // (On the script editor's background, where one is chosen:
                // its colors are chosen for it.)
                if let Some(fill) = palette.and_then(|p| p.background) {
                    ui.painter().rect_filled(ui.clip_rect(), 0.0, fill);
                }
                ui.set_min_width(*widest as f32 * wide);
                for at in range {
                    let line = &self.text[spans[at].clone()];
                    if line.is_empty() {
                        ui.allocate_exact_size(egui::vec2(0.0, height), egui::Sense::hover());
                    } else if let Some(job) = jobs.as_ref().and_then(|j| j.get(at)) {
                        ui.add(egui::Label::new(job.clone()).extend());
                    } else {
                        let text = egui::RichText::new(line).font(font.clone());
                        ui.add(egui::Label::new(text).extend());
                    }
                }
            });
        });
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
            Gff::read(&data).map_or_else(|e| Content::Error(e.to_string()), Content::Gff)
        } else if key.restype == ResType::TWODA {
            // (A copy that another overrides: as it stands on those below.)
            match mg_module::table_layers::layered_from(rm, &key, codepage, at) {
                Some(l) => Content::TwoDa(l),
                None => match mg_2da::TwoDa::parse(&data, codepage) {
                    Err(e) => Content::Error(e.to_string()),
                    Ok(_) => Content::Error("the copy the game reads can't be read".into()),
                },
            }
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
        Some(Viewed { data, origin, content, made: made() })
    }
}

/// A resource's bytes as something to read: a GFF's fields, text, or the
/// first bytes in hexadecimal.
#[derive(Debug)]
pub(crate) enum Plain {
    Gff(Box<Gff>),
    Text(LongText),
    Error(String),
}

/// What `data`, a resource of `key`'s type, shows as.
/// (`rm`: where a body part's skeleton is found, for a model as text.)
pub(crate) fn plain(key: ResKey, data: &[u8], rm: Option<&mg_resman::ResMan>) -> Plain {
    if key.restype.is_gff() {
        match Gff::read(data) {
            Ok(g) => Plain::Gff(Box::new(g)),
            Err(e) => Plain::Error(e.to_string()),
        }
    } else if is_text(key.restype, data) {
        // (Its bytes as Latin-1 where they aren't UTF-8.)
        let latin = || data.iter().map(|&b| b as char).collect();
        Plain::Text(String::from_utf8(data.to_vec()).unwrap_or_else(|_| latin()).into())
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

/// Draws a [`Plain`] view, scrolling.
pub(crate) fn plain_ui(ui: &mut Ui, key: ResKey, view: &Plain) {
    // (A text by its lines in sight: a compiled model's is megabytes.)
    if let Plain::Text(text) = view {
        return text.ui(ui, ("plain", key), None);
    }
    egui::ScrollArea::both().id_salt(("plain", key)).auto_shrink([false, false]).show(ui, |ui| {
        match view {
            Plain::Gff(g) => gff_tree(ui, &g.root, &key.to_string()),
            Plain::Text(_) => {}
            Plain::Error(e) => {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
        }
    });
}

/// Read-only view of a resource from the load order: the copy the game
/// reads, or (`layer`, by its label) the copy that layer has, which a
/// higher one overrides.
pub(crate) fn resource_ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey, layer: Option<&str>) {
    let Some(game) = &app.game else { return };
    let at = (key, layer.map(str::to_string));
    let viewed = match app.viewed.get(&at) {
        Some(v) => v.clone(),
        None => match Viewed::load(game, key, layer) {
            Some(v) => {
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
    let mut copy = false;
    let mut save = false;
    let mut edit_table = None;
    ui.horizontal(|ui| {
        ui.strong(key.to_string());
        ui.weak(format!("from {}, {} bytes", viewed.origin, viewed.data.len()));
        let can_copy = app.ws.as_ref().is_some_and(|w| !w.module.contains(&key));
        copy = ui.add_enabled(can_copy, egui::Button::new("Copy to Module")).clicked();
        save = ui.button("Save As…").clicked();
        if key.restype == ResType::TWODA {
            edit_table = match layer {
                None => table_menu(app, ui, key),
                // A copy a hak of the module has: edited in that hak.
                Some(layer) => hak_table_button(app, ui, key, layer),
            };
        }
    });
    if let Some(over) = &over {
        ui.weak(format!("Overridden by {over}, whose copy the game reads."));
    }
    ui.separator();
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
        two_da(app, ui, key, layer, (l, viewed.made));
        return;
    }
    // A text by its lines in sight, a script's in the editor's colors.
    let salt = ("resource-text", key, layer);
    match &viewed.content {
        Content::Script(text) => {
            let palette = crate::script_view::Palette::for_ui(&app.settings.script_style, ui);
            return text.ui(ui, salt, Some(&palette));
        }
        Content::Text(text) | Content::Binary(text) => return text.ui(ui, salt, None),
        _ => {}
    }
    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| match &viewed.content {
        Content::Gff(g) => gff_tree(ui, &g.root, &key.to_string()),
        Content::Error(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
        _ => {}
    });
}

fn is_text(t: ResType, data: &[u8]) -> bool {
    matches!(
        t,
        ResType::NSS
            | ResType::JUI
            | ResType::TXT
            | ResType::INI
            | ResType::SET
            | ResType::TXI
            | ResType::MTR
            | ResType::SHD
            | ResType::LUA
    ) || (!data.is_empty()
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

fn value_text(v: &Value) -> String {
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

/// A GFF struct as a read-only tree.
fn gff_tree(ui: &mut Ui, s: &Struct, id: &str) {
    mg_uikit::list_rows(ui); // (Rows 22 apart, as the trees' are.)
    for (i, f) in s.fields.iter().enumerate() {
        let label = f.label.to_string_lossy();
        let child_id = format!("{id}/{i}");
        match &f.value {
            Value::Struct(child) => {
                egui::CollapsingHeader::new(format!("{label} (struct {})", child.id))
                    .icon(mg_uikit::collapsing_icon)
                    .id_salt(&child_id)
                    .show(ui, |ui| gff_tree(ui, child, &child_id));
            }
            Value::List(items) => {
                egui::CollapsingHeader::new(format!("{label} (list, {} items)", items.len()))
                    .icon(mg_uikit::collapsing_icon)
                    .id_salt(&child_id)
                    .show(ui, |ui| {
                        for (j, item) in items.iter().enumerate() {
                            let item_id = format!("{child_id}/{j}");
                            // (Closed and out of sight: its header's room.)
                            if crate::gff_view::closed_out_of_sight(ui, &item_id) {
                                continue;
                            }
                            egui::CollapsingHeader::new(format!("[{j}] struct {}", item.id))
                                .icon(mg_uikit::collapsing_icon)
                                .id_salt(&item_id)
                                .show(ui, |ui| gff_tree(ui, item, &item_id));
                        }
                    });
            }
            v => {
                ui.horizontal(|ui| {
                    ui.label(&label);
                    ui.weak(v.field_type().json_name());
                    ui.monospace(value_text(v));
                });
            }
        }
    }
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
) {
    use crate::table_view::{GridOf, TableDoc, grid_in, keys_in, view_header};
    let name = key.resref.to_string();
    let kinds = crate::table_view::kinds(app, &name);
    let Moonglow { browser, game, settings, talk, talk_view, .. } = app;
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
    if !kept {
        if browser.table_views.len() >= VIEWS_KEPT {
            browser.table_views.clear();
        }
        browser.made_views += 1;
        let id = u32::MAX - browser.made_views;
        let doc = TableDoc::read_only(id, key, t.clone());
        browser.table_views.insert(at.clone(), TableView { of, rows_from: None, doc });
    }
    let rows_from = browser.rows_from.filter(|_| layered);
    let strrefs = !browser.raw_strrefs;
    let view = browser.table_views.get_mut(&at).expect("made above");
    if view.rows_from != rows_from {
        view.rows_from = rows_from;
        view.doc.within = rows_from
            .map(|o| std::sync::Arc::new((0..t.len()).filter(|&r| l.row_origin(r) == o).collect()));
    }
    let doc = &mut view.doc;
    doc.strref_text = strrefs;
    view_header(ui, doc);
    keys_in(doc, ui, true);
    let talk = talk.as_ref().filter(|_| !talk_view.outside);
    let of = GridOf { settings, table_name: &name, talk, game: game.as_deref() };
    grid_in(ui, doc, of, &kinds, &[], Some(l));
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
