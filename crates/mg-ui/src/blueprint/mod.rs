//! Blueprint editors (Aurora's object Properties dialogs) for the module's
//! blueprints, one tab each. A [`Form`] lays out a blueprint's fields; each
//! change is one undoable command on the blueprint, fields keep the integer
//! type they are stored with, and fields the editor does not show are kept.

use std::collections::HashMap;

use egui::Ui;
use mg_core::{LocString, ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{FieldType, Struct, Value};
use mg_module::palette::{BlueprintKind, Palette};
use mg_resman::ResKey;
use mg_rules::Choice;

use crate::dialogs::FileKind;
use crate::text::{decode, encode, with_english};
use crate::widgets::{
    FieldTarget, LocStringEdit, VarTableEdit, commit_number, commit_text, resref_field,
};
use crate::{Action, Moonglow, Tab};

mod creature;
mod creature_lists;
mod door;
pub(crate) mod encounter;
mod inventory;
mod item;
pub(crate) mod picker;
mod placeable;
mod situated;
mod sound;
pub(crate) mod store;
mod trigger;
mod visuals;
mod waypoint;

/// Whether a blueprint type has its own editor.
pub(crate) fn has_editor(t: ResType) -> bool {
    matches!(
        t,
        ResType::UTW
            | ResType::UTC
            | ResType::UTI
            | ResType::UTS
            | ResType::UTT
            | ResType::UTE
            | ResType::UTM
            | ResType::UTD
            | ResType::UTP
    )
}

/// The pages of a blueprint type's editor.
pub fn pages(t: ResType) -> &'static [&'static str] {
    match t {
        ResType::UTW => &waypoint::PAGES,
        ResType::UTS => &sound::PAGES,
        ResType::UTT => &trigger::PAGES,
        ResType::UTE => &encounter::PAGES,
        ResType::UTM => &store::PAGES,
        ResType::UTD => &door::PAGES,
        ResType::UTP => &placeable::PAGES,
        ResType::UTI => &item::PAGES,
        ResType::UTC => &creature::PAGES,
        _ => &[""],
    }
}

/// Each open editor's page.
/// Each open editor's page, by document and the object's path in it.
pub type Pages = HashMap<(ResKey, GffPath), &'static str>;

/// A creature appearance's picture (a plain body of it), for the row the
/// pointer rests on in the list.
fn creature_picture(app: &mut Moonglow, ui: &mut Ui, row: usize) {
    let look = mg_preview::CreatureLook::new(row as u16);
    picture(ui, crate::model_view::look_thumbnail(app, look));
}

/// A placeable appearance's picture (its model).
fn placeable_picture(app: &mut Moonglow, ui: &mut Ui, row: usize) {
    let model = app.game.as_deref().and_then(|g| {
        let t = g.table("placeables").ok()?;
        ResKey::parse(&t.get(row, "ModelName")?.to_ascii_lowercase(), ResType::MDL)
    });
    let made = model.and_then(|m| crate::model_view::thumbnail(app, m));
    picture(ui, made);
}

/// A model's picture beside a list's row, or that there is none.
fn picture(ui: &mut Ui, made: Option<egui::TextureId>) {
    const SIDE: f32 = 180.0;
    match made {
        Some(id) => {
            ui.add(egui::Image::new((id, egui::vec2(SIDE, SIDE))));
        }
        None => {
            ui.weak("(no picture)");
        }
    }
}

/// A blueprint's fields, laid out.
/// Shows something of a 2DA row (a picture) in a choice's list.
type Shown = fn(&mut Moonglow, &mut Ui, usize);

/// A loading screen's picture (loadscreens.2da's `BMPResRef`), small; a
/// line instead where the row has none (the first: a random one of the
/// area's tileset) or it isn't found.
fn load_screen_picture(app: &mut Moonglow, ui: &mut Ui, row: usize) {
    const WIDTH: f32 = 260.0;
    let name = app.game.as_deref().and_then(|g| {
        let t = g.table("loadscreens").ok()?;
        t.get(row, "BMPResRef").filter(|n| *n != "****" && !n.is_empty()).map(str::to_owned)
    });
    let Some(name) = name else {
        if row == 0 {
            ui.weak("A random one of the area's tileset");
        }
        return;
    };
    match app.loader().and_then(|mut l| l.load_screen(ui.ctx(), &name)) {
        Some(p) if p.size.x > 0.0 => {
            let size = egui::vec2(WIDTH, WIDTH * p.size.y / p.size.x);
            ui.add(egui::Image::new((p.texture.id(), size))).on_hover_text(name);
        }
        _ => {
            ui.weak(format!("({name}: no picture found)"));
        }
    }
}

pub(crate) struct Form<'a> {
    pub app: &'a mut Moonglow,
    /// The document: the blueprint, or the area's GIT for a placed object.
    pub key: ResKey,
    /// Where the object is in it: the root for a blueprint, its entry in a
    /// GIT list for a placed object.
    pub path: GffPath,
    /// Other objects or blueprints of the same type edited with it
    /// (Aurora's multi-editor): each field set is set on them too. Each is
    /// its document and path.
    pub also: Vec<(ResKey, GffPath)>,
    /// The object as it is now.
    pub root: Struct,
}

/// The text in the editing language (Options › Language).
/// The right-hand side of a page beside a palette: it scrolls (sideways
/// too, in a narrow window) and keeps a gutter on its right, clear of the
/// scroll bar, so its last column is never cut off.
pub(crate) fn side_panel<R>(ui: &mut Ui, id: egui::Id, add: impl FnOnce(&mut Ui) -> R) -> R {
    let gutter = egui::Margin { right: 14, ..Default::default() };
    egui::ScrollArea::both()
        .id_salt(id)
        .show(ui, |ui| {
            egui::Frame::NONE.inner_margin(gutter).show(ui, |ui| ui.vertical(add).inner).inner
        })
        .inner
}

/// A value of an integer field, of the type the field has (else `default`).
pub(super) fn integer(existing: Option<&Value>, v: i64, default: FieldType) -> Value {
    let t = existing.map_or(default, Value::field_type);
    match t {
        FieldType::Byte => Value::Byte(v.clamp(0, 255) as u8),
        FieldType::Char => Value::Char(v.clamp(-128, 127) as i8),
        FieldType::Word => Value::Word(v.clamp(0, 65535) as u16),
        FieldType::Short => Value::Short(v.clamp(-32768, 32767) as i16),
        FieldType::Dword => Value::Dword(v.clamp(0, u32::MAX as i64) as u32),
        FieldType::Dword64 => Value::Dword64(v.max(0) as u64),
        FieldType::Int64 => Value::Int64(v),
        _ => Value::Int(v.clamp(i32::MIN as i64, i32::MAX as i64) as i32),
    }
}

/// The color of text that is the talk table's (a StrRef's), not a
/// blueprint's own.
pub(crate) fn talk_table_color(ui: &Ui) -> egui::Color32 {
    if ui.visuals().dark_mode {
        egui::Color32::from_rgb(120, 180, 255)
    } else {
        egui::Color32::from_rgb(20, 80, 190)
    }
}

impl Form<'_> {
    /// A localized string's text as a field shows it
    /// ([`crate::widgets::loc_shown`]).
    fn shown_text(&self, ls: &LocString) -> String {
        crate::widgets::loc_shown(self.app, ls).0
    }

    /// Where that text is from, if it is not the string's own in the
    /// language edited.
    fn borrowed(&self, ls: &LocString) -> Option<(String, &'static str)> {
        crate::widgets::loc_shown(self.app, ls).1
    }

    /// A text field of a localized string, in the talk table's color where
    /// that is where its text is from, with the StrRef beside it.
    fn loc_text(
        &mut self,
        ui: &mut Ui,
        label: &str,
        current: &LocString,
        multiline: bool,
        width: f32,
    ) -> Option<String> {
        let id = self.id(label);
        let shown = self.shown_text(current);
        let Some((from, why)) = self.borrowed(current) else {
            return commit_text(self.app, ui, id, &shown, multiline, width);
        };
        crate::widgets::borrowed_field(ui, &from, why, |ui| {
            commit_text(self.app, ui, id, &shown, multiline, width)
        })
    }

    /// An item's Properties, from the inventory that holds it (item `index`
    /// of `list` in the struct at `at`): a placed object's item is its own,
    /// edited where it is held; a blueprint's is its blueprint (the
    /// module's to edit, the game's to look at).
    pub(crate) fn item_properties(
        &mut self,
        at: &GffPath,
        list: &str,
        index: usize,
        entry: &Struct,
    ) {
        if self.key.restype == ResType::GIT && entry.get("BaseItem").is_some() {
            let path = at.item(list, index);
            self.app
                .actions
                .push(Action::OpenTab(crate::Tab::Instance { area: self.key.resref, path }));
            return;
        }
        let r = inventory::entry_resref(entry);
        if !r.is_empty() {
            crate::palette_view::view_blueprint(self.app, ResKey::new(r, ResType::UTI));
        }
    }

    /// Gallery… beside an appearance: opens the Appearance Gallery at this
    /// object's, where a click gives it another.
    pub(crate) fn gallery_button(&mut self, ui: &mut Ui, kind: crate::appearance_gallery::Kind) {
        if ui
            .button("Gallery…")
            .on_hover_text(
                "The Appearance Gallery: every appearance as a picture, from this one on. Click \
                 one to give it to this object",
            )
            .clicked()
        {
            let gallery = crate::appearance_gallery::Gallery::of(kind, self.key, self.path.clone());
            self.app.placeable_gallery = Some(gallery);
            self.app.actions.push(Action::OpenTab(crate::Tab::PlaceableGallery));
        }
    }

    fn id(&self, name: &str) -> egui::Id {
        egui::Id::new(("blueprint", self.key, &self.path, name))
    }

    fn target(&self, label: &str) -> FieldTarget {
        let mut t = FieldTarget::new(self.key, self.path.clone(), label);
        t.also = self.also.clone();
        t
    }

    /// Where a field set goes: the object's document and path, and the
    /// others'.
    fn targets(&self) -> impl Iterator<Item = (ResKey, &GffPath)> {
        std::iter::once((self.key, &self.path)).chain(self.also.iter().map(|(k, p)| (*k, p)))
    }

    /// Whether the others edited with this one have another value in
    /// field `label` (not looked at past [`MIXED_LIMIT`] of them).
    pub(crate) fn differs(&mut self, label: &str) -> bool {
        if self.also.is_empty() || self.also.len() > MIXED_LIMIT {
            return false;
        }
        let Some(ws) = self.app.ws.as_mut() else { return false };
        let mine = self.root.get(label);
        self.also.iter().any(|(key, path)| {
            let theirs =
                ws.doc(key).ok().and_then(|d| path.get(&d.root)).and_then(|s| s.get(label));
            theirs != mine
        })
    }

    /// Marks the field about to be drawn if the others edited with this
    /// one have another value in it: "≠" before it. (Painted, not laid
    /// out: a table's cell holds the field alone.)
    fn mark_mixed(&mut self, ui: &Ui, label: &str) {
        if !self.differs(label) {
            return;
        }
        let at = ui.cursor().min + egui::vec2(-3.0, 2.0);
        let font = egui::TextStyle::Body.resolve(ui.style());
        ui.painter().text(at, egui::Align2::RIGHT_TOP, "≠", font, ui.visuals().warn_fg_color);
    }

    /// Several blueprints edited together (not placed objects).
    pub(crate) fn several_blueprints(&self) -> bool {
        !self.is_instance() && !self.also.is_empty()
    }

    /// Whether this is an object placed in an area (not a blueprint).
    pub(crate) fn is_instance(&self) -> bool {
        !self.path.0.is_empty()
    }

    /// The object's blueprint type (for a placed object, from its GIT
    /// list).
    pub(crate) fn restype(&self) -> ResType {
        instance_type(&self.path).unwrap_or(self.key.restype)
    }

    /// Sets a field (one undoable command named `what`).
    pub(crate) fn set(&mut self, what: &str, label: &str, value: Value) {
        let edits = self
            .targets()
            .map(|(key, path)| Edit::SetField {
                key,
                path: path.clone(),
                label: label.to_string(),
                value: Some(value.clone()),
            })
            .collect();
        self.app.actions.push(Action::Apply(Command::new(what, edits)));
    }

    /// Sets a field, or removes it (`None`).
    pub(crate) fn set_opt(&mut self, what: &str, label: &str, value: Option<Value>) {
        let edits = self
            .targets()
            .map(|(key, path)| Edit::SetField {
                key,
                path: path.clone(),
                label: label.to_string(),
                value: value.clone(),
            })
            .collect();
        self.app.actions.push(Action::Apply(Command::new(what, edits)));
    }

    /// Sets an integer field, keeping its stored type (for a field the
    /// blueprint lacks: the type the game's files give it, else `default`).
    pub(crate) fn set_int(&mut self, what: &str, label: &str, v: i64, default: FieldType) {
        let default = mg_schema::root_field_type(self.restype(), label).unwrap_or(default);
        let value = integer(self.root.get(label), v, default);
        self.set(what, label, value);
    }

    pub(crate) fn int(&self, label: &str) -> i64 {
        self.root.integer(label).unwrap_or(0)
    }

    /// A one-line text field (`CExoString`) of at most `max` characters.
    pub(crate) fn text(&mut self, ui: &mut Ui, what: &str, label: &str, max: usize) {
        self.mark_mixed(ui, label);
        let current = decode(self.root.string(label).unwrap_or_default());
        let id = self.id(label);
        let (shown, _) = crate::text::to_editor(&current);
        let buf = self.app.buffers.entry(id).or_insert_with(|| shown.clone());
        let r = ui.add(egui::TextEdit::singleline(buf).id(id).char_limit(max).desired_width(240.0));
        if !r.has_focus() && !r.lost_focus() && *buf != shown {
            *buf = shown.clone();
        }
        if r.lost_focus() && *buf != shown {
            let v = buf.clone();
            self.set(what, label, Value::String(encode(&v)));
        }
    }

    /// A multi-line text field (`CExoString`), line ends kept.
    pub(crate) fn memo(&mut self, ui: &mut Ui, what: &str, label: &str) {
        let current = decode(self.root.string(label).unwrap_or_default());
        let id = self.id(label);
        if let Some(v) = commit_text(self.app, ui, id, &current, true, f32::INFINITY) {
            self.set(what, label, Value::String(encode(&v)));
        }
    }

    /// A localized name: the English text, and `…` for every language.
    pub(crate) fn locstring(&mut self, ui: &mut Ui, what: &str, label: &str) {
        let current = self.root.locstring(label).cloned().unwrap_or_default();
        ui.horizontal(|ui| {
            if let Some(v) = self.loc_text(ui, label, &current, false, 240.0) {
                self.set(what, label, Value::LocString(with_english(current.clone(), &v)));
            }
            if ui.small_button("…").on_hover_text("Edit text in multiple languages").clicked() {
                self.app.loc_edit = Some(LocStringEdit::new(self.target(label), what, &current));
            }
        });
    }

    /// A localized multi-line text (descriptions).
    pub(crate) fn locstring_memo(&mut self, ui: &mut Ui, what: &str, label: &str) {
        let current = self.root.locstring(label).cloned().unwrap_or_default();
        ui.horizontal(|ui| {
            let language = crate::text::edit_language().name().unwrap_or("?");
            crate::widgets::field_label(ui, format!("{what} ({language})"));
            if ui.small_button("…").on_hover_text("Edit text in multiple languages").clicked() {
                self.app.loc_edit = Some(LocStringEdit::new(self.target(label), what, &current));
            }
        });
        if let Some(v) = self.loc_text(ui, label, &current, true, f32::INFINITY) {
            self.set(what, label, Value::LocString(with_english(current, &v)));
        }
    }

    /// Sets several fields in one command.
    pub(crate) fn set_fields(&mut self, what: &str, fields: Vec<(&str, Value)>) {
        let edits = self
            .targets()
            .flat_map(|(key, path)| {
                fields.iter().map(move |(label, value)| Edit::SetField {
                    key,
                    path: path.clone(),
                    label: label.to_string(),
                    value: Some(value.clone()),
                })
            })
            .collect();
        self.app.actions.push(Action::Apply(Command::new(what, edits)));
    }

    /// Sets several integer fields in one command.
    pub(crate) fn set_many(&mut self, what: &str, fields: &[(&str, i64, FieldType)]) {
        let values: Vec<(&str, Value)> = fields
            .iter()
            .map(|(label, v, t)| {
                let t = mg_schema::root_field_type(self.restype(), label).unwrap_or(*t);
                (*label, integer(self.root.get(label), *v, t))
            })
            .collect();
        let values = &values;
        let edits = self
            .targets()
            .flat_map(|(key, path)| {
                values.iter().map(move |(label, value)| Edit::SetField {
                    key,
                    path: path.clone(),
                    label: label.to_string(),
                    value: Some(value.clone()),
                })
            })
            .collect();
        self.app.actions.push(Action::Apply(Command::new(what, edits)));
    }

    /// A slider for an integer field.
    pub(crate) fn slider(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        range: std::ops::RangeInclusive<i64>,
    ) {
        let current = self.int(label);
        let mut v = current;
        let r = ui.add(egui::Slider::new(&mut v, range).clamping(egui::SliderClamping::Edits));
        if (r.drag_stopped() || (r.changed() && !r.dragged())) && v != current {
            self.set_int(what, label, v, FieldType::Byte);
        }
    }

    /// A float field in `range`, dragged in steps of `speed`.
    pub(crate) fn float(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        range: std::ops::RangeInclusive<f32>,
        speed: f64,
    ) {
        self.mark_mixed(ui, label);
        let current = self.root.float(label).unwrap_or(0.0);
        if let Some(v) = crate::widgets::drag_number(ui, current, |d| {
            d.range(range).speed(speed).max_decimals(2)
        }) {
            self.set(what, label, Value::Float(v));
        }
    }

    /// Milliseconds (DWORD) shown and edited as seconds.
    pub(crate) fn millis(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        range: std::ops::RangeInclusive<f32>,
    ) {
        let current = self.int(label) as f32 / 1000.0;
        if let Some(v) = crate::widgets::drag_number(ui, current, |d| {
            d.range(range).speed(0.1).max_decimals(3).suffix(" s")
        }) {
            self.set_int(what, label, (v * 1000.0).round() as i64, FieldType::Dword);
        }
    }

    /// A colour stored as 0x00BBGGRR (ARE lighting), with a colour picker;
    /// what is picked is stored when the picker closes (one command).
    pub(crate) fn color(&mut self, ui: &mut Ui, what: &str, label: &str) {
        self.mark_mixed(ui, label);
        let current = self.int(label) as u32;
        let pending_id = self.id(label).with("pending");
        let pending: Option<u32> = ui.data(|d| d.get_temp(pending_id));
        let shown = pending.unwrap_or(current);
        let mut rgb = [shown & 0xFF, (shown >> 8) & 0xFF, (shown >> 16) & 0xFF].map(|c| c as u8);
        let (changed, open) = ui
            .push_id(self.id(label), |ui| {
                let popup = ui.auto_id_with("popup");
                let changed = ui.color_edit_button_srgb(&mut rgb).changed();
                (changed, egui::Popup::is_id_open(ui.ctx(), popup))
            })
            .inner;
        let picked = u32::from(rgb[0]) | u32::from(rgb[1]) << 8 | u32::from(rgb[2]) << 16;
        if changed {
            ui.data_mut(|d| d.insert_temp(pending_id, picked));
        }
        if !open && let Some(v) = ui.data(|d| d.get_temp::<u32>(pending_id)) {
            ui.data_mut(|d| d.remove::<u32>(pending_id));
            if v != current {
                self.set_int(what, label, i64::from(v), FieldType::Dword);
            }
        }
    }

    /// A checkbox for a flag (BYTE 0 or 1); the value.
    pub(crate) fn check(&mut self, ui: &mut Ui, text: &str, label: &str) -> bool {
        self.mark_mixed(ui, label);
        let current = self.int(label) != 0;
        let mut v = current;
        if ui.checkbox(&mut v, text).changed() {
            self.set_int(text, label, i64::from(v), FieldType::Byte);
        }
        v
    }

    /// A number in `range`.
    pub(crate) fn number(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        range: std::ops::RangeInclusive<i64>,
    ) {
        self.mark_mixed(ui, label);
        if let Some(v) = commit_number(ui, self.int(label), range) {
            self.set_int(what, label, v, FieldType::Byte);
        }
    }

    /// A PLT colour (0–175) picked from a palette's swatches (Aurora's
    /// colour chooser; `palette` as `pal_cloth01`); the game is given where
    /// the page has lent it out.
    pub(crate) fn palette_color(
        &mut self,
        ui: &mut Ui,
        game: Option<&mg_rules::GameData>,
        what: &str,
        label: &str,
        palette: &str,
    ) {
        let current = self.int(label).clamp(0, 175) as u8;
        let pick = self.palette_pick(ui, game, what, label, palette, current, None);
        if let Some(v) = pick.filter(|&v| v != current) {
            self.set_int(what, label, i64::from(v), FieldType::Byte);
        }
    }

    /// [`Form::palette_color`]'s chooser alone, showing colour `current`
    /// (with `text` in place of its number): the colour picked.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn palette_pick(
        &mut self,
        ui: &mut Ui,
        game: Option<&mg_rules::GameData>,
        what: &str,
        label: &str,
        palette: &str,
        current: u8,
        text: Option<&str>,
    ) -> Option<u8> {
        let pal = {
            let app = &mut *self.app;
            let game = game.or(app.game.as_deref());
            game.and_then(|game| {
                let mut l = crate::images::Loader {
                    pictures: &mut app.pictures,
                    palettes: &mut app.palettes,
                    ws: app.ws.as_ref(),
                    game,
                };
                l.palette(palette)
            })
        };
        let mut pick = None;
        ui.horizontal(|ui| {
            if let Some(p) = &pal {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(28.0, 16.0), egui::Sense::hover());
                crate::images::swatch(ui, rect, &crate::images::tones(p, current));
            }
            egui::ComboBox::from_id_salt(("palette-color", self.key, label))
                .selected_text(text.map_or_else(|| current.to_string(), str::to_string))
                .width(56.0)
                .height(11.0 * 16.0 + 24.0)
                .show_ui(ui, |ui| {
                    let Some(p) = &pal else {
                        ui.weak("No palette");
                        return;
                    };
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                    // 176 colours, 16 to a row.
                    for row in 0..11u8 {
                        ui.horizontal(|ui| {
                            for i in row * 16..row * 16 + 16 {
                                let size = egui::vec2(16.0, 14.0);
                                let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click());
                                crate::images::swatch(ui, rect, &crate::images::tones(p, i));
                                if i == current && text.is_none() {
                                    let stroke = ui.visuals().selection.stroke;
                                    ui.painter().rect_stroke(
                                        rect,
                                        0.0,
                                        stroke,
                                        egui::StrokeKind::Outside,
                                    );
                                }
                                let name = format!("{what} {i}");
                                r.widget_info(|| {
                                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name)
                                });
                                if r.on_hover_text(&name).clicked() {
                                    pick = Some(i);
                                    ui.close();
                                }
                            }
                        });
                    }
                });
        });
        pick
    }

    /// A choice among 2DA rows (the field holds the row; `default` is its
    /// type when the blueprint lacks it). Long lists get a filter.
    pub(crate) fn choice(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        choices: &[Choice],
        default: FieldType,
    ) {
        self.mark_mixed(ui, label);
        self.choice_shown(ui, what, label, choices, default, None);
    }

    /// A creature's or a placeable's appearance: [`choice`](Self::choice),
    /// with each appearance's picture beside the row the pointer rests on.
    pub(crate) fn appearance_choice(
        &mut self,
        ui: &mut Ui,
        label: &str,
        choices: &[Choice],
        default: FieldType,
        creature: bool,
    ) {
        let show: Shown = if creature { creature_picture } else { placeable_picture };
        self.choice_shown(ui, "Appearance", label, choices, default, Some(show));
    }

    /// The loading screen (a loadscreens.2da row): its picture shows under
    /// the choice, and each one's on the row the pointer rests on.
    pub(crate) fn load_screen(&mut self, ui: &mut Ui, choices: &[Choice]) {
        ui.vertical(|ui| {
            let show: Shown = load_screen_picture;
            self.choice_shown(
                ui,
                "Loading screen",
                "LoadScreenID",
                choices,
                FieldType::Word,
                Some(show),
            );
            let row = self.int("LoadScreenID");
            if let Ok(row) = usize::try_from(row) {
                load_screen_picture(self.app, ui, row);
            }
        });
    }

    /// [`choice`](Self::choice), with what `about` shows of a row beside
    /// the one the pointer rests on.
    fn choice_shown(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        choices: &[Choice],
        default: FieldType,
        about: Option<Shown>,
    ) {
        let current = self.int(label);
        let shown = choices
            .iter()
            .find(|c| c.row as i64 == current)
            .map_or_else(|| format!("({current})"), |c| c.text.clone());
        let filter_id = self.id(&format!("{label}#filter"));
        let mut pick = None;
        // (Open until a choice is made or a click lands outside: a click in
        // its Filter field must not close it.)
        let combo = egui::ComboBox::from_id_salt(self.id(label))
            .selected_text(shown)
            .width(220.0)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        // A long list by name; a short one as the table orders it.
        let sorted;
        let choices = if choices.len() > mg_rules::ORDERED_CHOICES {
            sorted = mg_rules::by_name(choices.to_vec());
            sorted.as_slice()
        } else {
            choices
        };
        let list = combo.show_ui(ui, |ui| {
            let mut filter: String = ui.data(|d| d.get_temp(filter_id)).unwrap_or_default();
            if choices.len() > 30 {
                ui.add(egui::TextEdit::singleline(&mut filter).hint_text("Filter"));
                ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
            }
            let filter = filter.to_lowercase();
            for c in choices
                .iter()
                .filter(|c| filter.is_empty() || c.text.to_lowercase().contains(&filter))
            {
                let mut r = ui.selectable_label(c.row as i64 == current, &c.text);
                if let Some(about) = about {
                    let app = &mut *self.app;
                    r = r.on_hover_ui_at_pointer(|ui| about(app, ui, c.row));
                }
                if r.clicked() {
                    pick = Some(c.row as i64);
                    ui.close();
                }
            }
        });
        // The arrow keys step through it, as in Aurora (in the order
        // shown).
        let rows: Vec<i64> = choices.iter().map(|c| c.row as i64).collect();
        let pick = pick.or_else(|| crate::widgets::arrow_pick(ui, &list.response, &rows, current));
        if let Some(v) = pick.filter(|&v| v != current) {
            self.set_int(what, label, v, default);
        }
    }

    /// A resource name with a picker.
    #[allow(dead_code)] // for the editors that follow
    pub(crate) fn resref(&mut self, ui: &mut Ui, what: &str, label: &str, types: &[ResType]) {
        self.mark_mixed(ui, label);
        let current = self.root.resref(label).unwrap_or(ResRef::EMPTY);
        if let Some(v) = resref_field(self.app, ui, self.id(label), current, what, types) {
            self.set(what, label, Value::resref(v));
        }
    }

    /// A script field: name, picker and Edit.
    pub(crate) fn script(&mut self, ui: &mut Ui, what: &str, label: &str) {
        self.mark_mixed(ui, label);
        let types = [ResType::NSS, ResType::NCS];
        let what = format!("{what} script");
        let current = self.root.resref(label).unwrap_or(ResRef::EMPTY);
        ui.horizontal(|ui| {
            if let Some(v) = resref_field(self.app, ui, self.id(label), current, &what, &types) {
                self.set(&what, label, Value::resref(v));
            }
            let field = self.id(label);
            if let Some(name) = crate::widgets::edit_script_button(self.app, ui, field, current) {
                self.app.actions.push(Action::EditScript { name, condition: false });
            }
        });
    }

    /// Load Script Set and Save Script Set (`.ini`, `mg_module::script_set`)
    /// for a page's events (label, field); nothing for a page whose events
    /// script sets do not name (a store's). Loading sets every event of the
    /// page, those the file leaves out to none, in one command.
    pub(crate) fn script_set_buttons(&mut self, ui: &mut Ui, events: &[(&str, &str)]) {
        use mg_module::script_set;
        let keyed: Vec<(&'static str, &str)> = events
            .iter()
            .filter_map(|(_, field)| Some((script_set::key(field)?, *field)))
            .collect();
        if keyed.is_empty() {
            return;
        }
        let (mut load, mut save) = (false, false);
        ui.horizontal(|ui| {
            load = ui
                .button("Load Script Set")
                .on_hover_text("Set the scripts from a script set file (.ini)")
                .clicked();
            save = ui
                .button("Save Script Set")
                .on_hover_text("Save these scripts as a script set file (.ini)")
                .clicked();
        });
        if load {
            // The game's script sets are in data/scr.
            let start = self.app.install.as_ref().map(|i| i.root.join("data").join("scr"));
            let Some(path) = self.app.dialogs.open_file(FileKind::ScriptSet, start.as_deref())
            else {
                return;
            };
            let text = match std::fs::read(&path) {
                Ok(b) => decode(&b),
                Err(e) => {
                    self.app.log.error(format!("{}: {e}", path.display()));
                    return;
                }
            };
            let set = script_set::read(&text);
            let mut fields = Vec::new();
            for (key, field) in &keyed {
                let name = set.get(key).map_or("", String::as_str);
                match ResRef::from_str(name) {
                    Ok(r) => fields.push((*field, Value::resref(r))),
                    Err(_) => self
                        .app
                        .log
                        .warn(format!("{}: {key}: {name:?} is not a script name", path.display())),
                }
            }
            self.set_fields("Load script set", fields);
        }
        if save
            && let Some(path) = self
                .app
                .dialogs
                .save_file(FileKind::ScriptSet, Some(std::path::Path::new("scripts.ini")))
        {
            let names: Vec<(&str, String)> = keyed
                .iter()
                .map(|(key, field)| {
                    (*key, self.root.resref(field).unwrap_or(ResRef::EMPTY).to_string())
                })
                .collect();
            let pairs: Vec<(&str, &str)> = names.iter().map(|(k, v)| (*k, v.as_str())).collect();
            if let Err(e) = std::fs::write(&path, encode(&script_set::write(&pairs))) {
                self.app.log.error(format!("{}: {e}", path.display()));
            }
        }
    }

    /// The conversation: name, picker and Edit.
    pub(crate) fn conversation(&mut self, ui: &mut Ui) {
        self.linked(ui, "Conversation", "Conversation", &[ResType::DLG], ResType::DLG, Tab::Dialog);
    }

    /// A resource name with a picker (of `types`) and Edit, which opens the
    /// module's `edit` resource in its editor (`tab`), else the game's.
    fn linked(
        &mut self,
        ui: &mut Ui,
        what: &str,
        label: &str,
        types: &[ResType],
        edit: ResType,
        tab: fn(ResKey) -> Tab,
    ) {
        let current = self.root.resref(label).unwrap_or(ResRef::EMPTY);
        ui.horizontal(|ui| {
            if let Some(v) = resref_field(self.app, ui, self.id(label), current, what, types) {
                self.set(what, label, Value::resref(v));
            }
            let key = ResKey::new(current, edit);
            let in_module = self.app.ws.as_ref().is_some_and(|w| w.module.contains(&key));
            let in_game = self.app.game.as_deref().is_some_and(|g| g.resman.contains(&key));
            if ui
                .add_enabled(
                    !current.is_empty() && (in_module || in_game),
                    egui::Button::new("Edit").small(),
                )
                .clicked()
            {
                let tab = if in_module { tab(key) } else { Tab::Resource(key) };
                self.app.actions.push(Action::OpenTab(tab));
            }
        });
    }

    /// The palette category (Aurora: read-only text and `…`): a list of the
    /// type's categories.
    pub(crate) fn category(&mut self, ui: &mut Ui, kind: BlueprintKind) {
        if self.is_instance() {
            ui.weak("(placed in an area)");
            return;
        }
        let Some(game) = self.app.game.as_deref() else { return };
        // The module's own categories (Palette › Categories…), else the
        // game's.
        let own = self.app.ws.as_mut().and_then(|ws| ws.doc(&kind.skeleton_key()).ok().cloned());
        let skeleton = own.or_else(|| {
            let data = game.resman.get(&kind.skeleton_key()).ok()?;
            mg_gff::Gff::read(&data).ok()
        });
        let categories = skeleton.map(|g| Palette::read(&g).categories(game)).unwrap_or_default();
        let label = kind.palette_field();
        let current = self.int(label);
        let shown = categories
            .iter()
            .find(|(id, _)| i64::from(*id) == current)
            .map_or_else(|| format!("({current})"), |(_, name)| name.clone());
        let mut pick = None;
        let width = ui.available_width().clamp(120.0, 260.0);
        egui::ComboBox::from_id_salt(self.id(label)).selected_text(shown).width(width).show_ui(
            ui,
            |ui| {
                for (id, name) in &categories {
                    if ui.selectable_label(i64::from(*id) == current, name).clicked() {
                        pick = Some(*id);
                    }
                }
            },
        );
        if let Some(id) = pick.filter(|&id| i64::from(id) != current) {
            self.set_int("Palette category", label, i64::from(id), FieldType::Byte);
        }
    }

    /// The Variables… button.
    pub(crate) fn variables(&mut self, ui: &mut Ui) {
        let list = self.root.list("VarTable").unwrap_or(&[]).to_vec();
        let label = format!("Variables ({})…", list.len());
        if ui.button(label).on_hover_text("Edit scripting variables").clicked() {
            self.app.var_edit = Some(VarTableEdit::new(self.target("VarTable"), &list));
        }
    }

    /// Update Instances (Aurora's Advanced pages, for blueprints of every
    /// type but waypoints): every object placed from this blueprint, in
    /// every area, made again from it where it stands, facing as it faces
    /// (a trigger or encounter keeping its outline and spawn points). One
    /// command.
    pub(crate) fn update_instances(&mut self, ui: &mut Ui) {
        if self.is_instance() {
            return;
        }
        let r = ui
            .button("Update Instances")
            .on_hover_text("Update all instances created from this Blueprint");
        if r.clicked() {
            let keys = self.targets().map(|(k, _)| k).collect();
            self.app.update_instances_of(keys);
        }
    }

    /// The blueprint's resref (Aurora's "Blueprint ResRef"): changing it
    /// renames the blueprint.
    pub(crate) fn blueprint_resref(&mut self, ui: &mut Ui) {
        if self.several_blueprints() {
            ui.weak(format!("{} and {} more", self.key.resref, self.also.len()))
                .on_hover_text("≠ before a field: the others have another value there");
            return;
        }
        if self.is_instance() {
            // A placed object names the blueprint it was made from.
            let field = BlueprintKind::from_restype(self.restype())
                .map_or("TemplateResRef", |k| k.resref_field());
            let template = self.root.resref(field).map(|r| r.to_string()).unwrap_or_default();
            ui.label(template).on_hover_text("The blueprint it was placed from");
            return;
        }
        let current = self.key.resref;
        let id = self.id("TemplateResRef");
        let buf = self.app.buffers.entry(id).or_insert_with(|| current.to_string());
        let r = ui
            .add(egui::TextEdit::singleline(buf).id(id).char_limit(16).desired_width(160.0))
            .on_hover_text("Edit the Blueprint ResRef");
        if !r.has_focus() && !r.lost_focus() && *buf != current.to_string() {
            *buf = current.to_string();
        }
        if r.lost_focus() && *buf != current.to_string() {
            let text = buf.trim().to_ascii_lowercase();
            match ResRef::from_str(&text) {
                Ok(new) if !new.is_empty() => {
                    self.app.actions.push(Action::RenameBlueprint { from: self.key, to: new })
                }
                Ok(_) => self.app.log.error("A blueprint needs a resref"),
                Err(e) => self.app.log.error(e.to_string()),
            }
        }
    }
}

/// After a command: values derived from what it changed (an item's cost, a
/// creature's maximum hit points), as part of it.
/// The objects of type `restype` a command changed (other than by setting
/// `derived`, the field kept from them): blueprints at their root, placed
/// objects at their GIT entry.
pub(crate) fn changed_objects(
    cmd: &Command,
    restype: ResType,
    derived: &[&str],
) -> Vec<(ResKey, GffPath)> {
    let list = mg_module::instances::git_list(restype).map(|(l, _)| l);
    let mut out: Vec<(ResKey, GffPath)> = Vec::new();
    for e in &cmd.edits {
        let (key, path) = match e {
            Edit::SetField { key, path, label, .. } if !derived.contains(&label.as_str()) => {
                (key, path)
            }
            Edit::InsertItem { key, path, .. } | Edit::RemoveItem { key, path, .. } => (key, path),
            _ => continue,
        };
        let object = if key.restype == restype {
            GffPath::root()
        } else if key.restype == ResType::GIT
            && restype == ResType::UTI
            && let Some(n) = held_item_at(path)
        {
            // (An item a placed object holds, changed in its own
            // Properties.)
            GffPath(path.0[..n].to_vec())
        } else if key.restype == ResType::GIT
            && let Some(step @ mg_edit::Step::Item(l, _)) = path.0.first()
            && Some(l.as_str()) == list
        {
            GffPath(vec![step.clone()])
        } else {
            continue;
        };
        if !out.contains(&(*key, object.clone())) {
            out.push((*key, object));
        }
    }
    out
}

/// The objects whose derived values a command leaves to bring up to date
/// (worked out before it is applied, which takes the command).
pub(crate) struct Derived {
    items: Vec<(ResKey, GffPath)>,
    creatures: Vec<(ResKey, GffPath)>,
}

pub(crate) fn derived_of(cmd: &Command) -> Derived {
    Derived {
        items: changed_objects(cmd, ResType::UTI, &["Cost"]),
        creatures: changed_objects(cmd, ResType::UTC, &["MaxHitPoints", "ChallengeRating"]),
    }
}

pub(crate) fn after_apply(app: &mut Moonglow, derived: Derived) {
    item::refresh_costs(app, derived.items);
    creature::refresh_hit_points(app, derived.creatures);
}

/// Renames a blueprint everywhere (its resource, its `TemplateResRef` or a
/// store's `ResRef`, and the objects placed from it) as one undoable
/// command, and points its editor at the new name.
pub(crate) fn rename(app: &mut Moonglow, from: ResKey, to: ResRef) {
    app.rename_resource(from, to, false);
}

/// A blueprint editor tab.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    edit(app, ui, key, GffPath::root());
}

/// The blueprint type of the object at `path` in a GIT (its list's).
pub(crate) fn instance_type(path: &GffPath) -> Option<ResType> {
    // An item held by a placed object (in its inventory, or equipped) is
    // an item whatever holds it.
    if held_item_at(path).is_some_and(|n| n == path.0.len()) {
        return Some(ResType::UTI);
    }
    let Some(mg_edit::Step::Item(list, _)) = path.0.first() else { return None };
    [
        ResType::UTC,
        ResType::UTD,
        ResType::UTE,
        ResType::UTI,
        ResType::UTP,
        ResType::UTS,
        ResType::UTM,
        ResType::UTT,
        ResType::UTW,
    ]
    .into_iter()
    .find(|t| mg_module::instances::git_list(*t).is_some_and(|(l, _)| l == list))
}

/// The lists a placed object holds items in.
pub(crate) const HELD_LISTS: [&str; 2] = ["ItemList", "Equip_ItemList"];

/// Where `path` goes into an item a placed object holds: how many of its
/// steps lead to the item (the rest go on into the item).
fn held_item_at(path: &GffPath) -> Option<usize> {
    path.0.iter().skip(1).position(|step| {
        matches!(step, mg_edit::Step::Item(list, _) if HELD_LISTS.contains(&list.as_str()))
    })
    .map(|i| i + 2)
}

/// The pages of a placed object's Properties: its blueprint's, less the
/// blueprint's comments.
pub(crate) fn instance_pages(t: ResType) -> Vec<&'static str> {
    pages(t).iter().copied().filter(|p| *p != "Comments").collect()
}

/// The Properties of the object at `path` in the document `key` (a
/// blueprint at the root, or an object placed in an area's GIT).
pub(crate) fn edit(app: &mut Moonglow, ui: &mut Ui, key: ResKey, path: GffPath) {
    edit_many(app, ui, key, path, Vec::new());
}

/// The Properties of several objects or blueprints of one type at once
/// (Aurora's multi-editor for placed objects): shown as the first, each
/// field changed set on all.
pub(crate) fn edit_many(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    path: GffPath,
    also: Vec<(ResKey, GffPath)>,
) {
    let Some(ws) = &mut app.ws else {
        ui.label("No module is open.");
        return;
    };
    let root = match ws.doc(&key) {
        Ok(g) => match path.get(&g.root) {
            Some(s) => s.clone(),
            None => {
                ui.label("This object is no longer there.");
                return;
            }
        },
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return;
        }
    };
    let restype = instance_type(&path).unwrap_or(key.restype);
    let several = path.0.is_empty() && !also.is_empty();
    let pages: Vec<&str> = if several {
        // Lists are edited one blueprint at a time.
        pages(restype).iter().copied().filter(|p| !LIST_PAGES.contains(p)).collect()
    } else if path.0.is_empty() {
        pages(restype).to_vec()
    } else {
        instance_pages(restype)
    };
    let page_key = (key, path.clone());
    let mut page = app.blueprint_pages.get(&page_key).copied().unwrap_or(pages[0]);
    if !pages.contains(&page) {
        page = pages[0];
    }
    ui.horizontal_wrapped(|ui| {
        for p in &pages {
            ui.selectable_value(&mut page, *p, *p);
        }
    });
    app.blueprint_pages.insert(page_key, page);
    ui.separator();
    let viewed = app.ws.as_ref().is_some_and(|ws| ws.is_viewed(&key));
    let refused_id = egui::Id::new(("view-refused", key));
    if viewed {
        ui.horizontal_wrapped(|ui| {
            // Said strongly for a few seconds after a change was tried.
            let now = ui.input(|i| i.time);
            let refused = ui
                .data(|d| d.get_temp::<f64>(refused_id))
                .is_some_and(|at| now - at < VIEW_REFUSED_SHOWN);
            let text = if refused {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                egui::RichText::new(
                    "Not changed: this is the game's blueprint, shown to look at. Edit Copy \
                     makes one of the module's own.",
                )
                .strong()
            } else {
                egui::RichText::new(
                    "This is the game's blueprint, shown to look at: it can't be changed here.",
                )
            };
            ui.colored_label(ui.visuals().warn_fg_color, text);
            if ui.button("Edit Copy…").on_hover_text("A copy in the module, to change").clicked()
            {
                app.actions.push(Action::CopyDialog(key));
            }
        });
        ui.separator();
    }
    if !also.is_empty() {
        let what = if path.0.is_empty() { "blueprints" } else { "objects" };
        ui.weak(format!(
            "{} {what}: shown as the first; what you change is set on each (≠: they differ there)",
            also.len() + 1
        ));
    }
    if several {
        ui.weak("Inventories, classes, skills, feats, spells and properties are edited one blueprint at a time.");
    }
    let note_id = egui::Id::new(("several-note", key));
    if let Some(note) = ui.data(|d| d.get_temp::<String>(note_id)) {
        ui.colored_label(ui.visuals().warn_fg_color, note);
    }
    let pending = app.actions.len();
    let mut form = Form { app, key, path, also, root };
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        if viewed {
            read_only_look(ui);
        }
        match restype {
            ResType::UTW => waypoint::page(&mut form, ui, page),
            ResType::UTS => sound::page(&mut form, ui, page),
            ResType::UTT => trigger::page(&mut form, ui, page),
            ResType::UTE => encounter::page(&mut form, ui, page),
            ResType::UTM => store::page(&mut form, ui, page),
            ResType::UTD => door::page(&mut form, ui, page),
            ResType::UTP => placeable::page(&mut form, ui, page),
            ResType::UTI => item::page(&mut form, ui, page),
            ResType::UTC => creature::page(&mut form, ui, page),
            _ => {}
        }
    });
    // The game's blueprint takes no change: what a page asked for is
    // dropped, and the note above says so.
    if viewed {
        let app = &mut *form.app;
        let mut tried = false;
        let mut i = pending;
        while i < app.actions.len() {
            let changes = matches!(&app.actions[i], Action::Apply(cmd)
                if cmd.edits.iter().any(|e| edit_key(e) == key));
            if changes {
                app.actions.remove(i);
                tried = true;
            } else {
                i += 1;
            }
        }
        if tried {
            let now = ui.input(|i| i.time);
            ui.data_mut(|d| d.insert_temp(refused_id, now));
            ui.ctx().request_repaint();
        }
    }
    // A change only the first would take (a page's own list edits) is
    // dropped rather than made to one of several blueprints.
    if several {
        let keys: Vec<ResKey> = form.targets().map(|(k, _)| k).collect();
        let app = form.app;
        let mut dropped = false;
        let mut i = pending;
        while i < app.actions.len() {
            let partial = match &app.actions[i] {
                Action::Apply(cmd) => {
                    let edited: Vec<ResKey> = cmd.edits.iter().map(edit_key).collect();
                    keys.iter().any(|k| edited.contains(k))
                        && !keys.iter().all(|k| edited.contains(k))
                }
                _ => false,
            };
            if partial {
                app.actions.remove(i);
                dropped = true;
            } else {
                i += 1;
            }
        }
        if dropped {
            let note = "That change can only be made to one blueprint at a time.".to_string();
            ui.data_mut(|d| d.insert_temp(note_id, note));
        }
    }
}

/// How long the note of a blueprint only looked at says a change was
/// refused, in seconds.
const VIEW_REFUSED_SHOWN: f64 = 4.0;

/// The most objects edited together whose values are compared, field by
/// field, to mark the fields that differ.
const MIXED_LIMIT: usize = 64;

/// The look of a form that can't be changed (the game's blueprint, opened
/// by View): its fields are flat and dim, and don't answer the pointer.
/// It still scrolls, and its lists and pages can be looked through.
fn read_only_look(ui: &mut Ui) {
    let v = ui.visuals_mut();
    let quiet = v.widgets.noninteractive;
    let text = egui::Stroke::new(1.0, v.weak_text_color());
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
        w.bg_fill = egui::Color32::TRANSPARENT;
        w.weak_bg_fill = egui::Color32::TRANSPARENT;
        w.bg_stroke = quiet.bg_stroke;
        w.fg_stroke = text;
        w.expansion = 0.0;
    }
    v.extreme_bg_color = v.panel_fill;
    v.text_edit_bg_color = Some(v.panel_fill);
}

/// The pages that edit lists of their own (the first blueprint's only),
/// left out when editing several blueprints.
const LIST_PAGES: [&str; 9] = [
    "Inventory",
    "Classes",
    "Skills",
    "Feats",
    "Spells",
    "Special Abilities",
    "Properties",
    "Creature List",
    "Restrictions",
];

/// The document an edit changes.
fn edit_key(e: &Edit) -> ResKey {
    match e {
        Edit::SetField { key, .. }
        | Edit::InsertItem { key, .. }
        | Edit::RemoveItem { key, .. }
        | Edit::SetResource { key, .. } => *key,
    }
}
