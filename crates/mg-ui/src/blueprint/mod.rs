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
mod store;
mod trigger;
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

/// A blueprint's fields, laid out.
pub(crate) struct Form<'a> {
    pub app: &'a mut Moonglow,
    /// The document: the blueprint, or the area's GIT for a placed object.
    pub key: ResKey,
    /// Where the object is in it: the root for a blueprint, its entry in a
    /// GIT list for a placed object.
    pub path: GffPath,
    /// Other objects of the same type edited with it (Aurora's
    /// multi-editor): each field set is set on them too.
    pub also: Vec<GffPath>,
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

fn english(ls: &LocString) -> String {
    crate::text::edited_text(ls)
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

impl Form<'_> {
    /// A localized string's English text, else its talk-table string (what
    /// Aurora shows for the game's blueprints).
    fn shown_text(&self, ls: &LocString) -> String {
        let text = english(ls);
        if !text.is_empty() || ls.strref.is_none() {
            return text;
        }
        self.app.game.as_ref().and_then(|g| g.string(ls.strref)).unwrap_or_default()
    }

    fn id(&self, name: &str) -> egui::Id {
        egui::Id::new(("blueprint", self.key, &self.path, name))
    }

    fn target(&self, label: &str) -> FieldTarget {
        let mut t = FieldTarget::new(self.key, self.path.clone(), label);
        t.also = self.also.clone();
        t
    }

    /// The paths a field set goes to: the object's and the others'.
    fn paths(&self) -> impl Iterator<Item = &GffPath> {
        std::iter::once(&self.path).chain(&self.also)
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
            .paths()
            .map(|path| Edit::SetField {
                key: self.key,
                path: path.clone(),
                label: label.to_string(),
                value: Some(value.clone()),
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
        let shown = self.shown_text(&current);
        ui.horizontal(|ui| {
            let id = self.id(label);
            if let Some(v) = commit_text(self.app, ui, id, &shown, false, 240.0) {
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
            ui.label(format!("{what} ({language})"));
            if ui.small_button("…").on_hover_text("Edit text in multiple languages").clicked() {
                self.app.loc_edit = Some(LocStringEdit::new(self.target(label), what, &current));
            }
        });
        let id = self.id(label);
        let shown = self.shown_text(&current);
        if let Some(v) = commit_text(self.app, ui, id, &shown, true, f32::INFINITY) {
            self.set(what, label, Value::LocString(with_english(current, &v)));
        }
    }

    /// Sets several fields in one command.
    pub(crate) fn set_fields(&mut self, what: &str, fields: Vec<(&str, Value)>) {
        let edits = self
            .paths()
            .flat_map(|path| {
                fields.iter().map(|(label, value)| Edit::SetField {
                    key: self.key,
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
        let edits = self
            .paths()
            .flat_map(|path| {
                fields.iter().map(|(label, v, t)| Edit::SetField {
                    key: self.key,
                    path: path.clone(),
                    label: label.to_string(),
                    value: Some(integer(
                        self.root.get(label),
                        *v,
                        mg_schema::root_field_type(self.restype(), label).unwrap_or(*t),
                    )),
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
        let current = self.root.float(label).unwrap_or(0.0);
        let mut v = current;
        let r = ui.add(
            egui::DragValue::new(&mut v)
                .range(range)
                .clamp_existing_to_range(false)
                .speed(speed)
                .max_decimals(2),
        );
        if (r.drag_stopped() || (r.changed() && !r.dragged())) && v != current {
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
        let mut v = current;
        let r = ui.add(
            egui::DragValue::new(&mut v)
                .range(range)
                .clamp_existing_to_range(false)
                .speed(0.1)
                .max_decimals(3)
                .suffix(" s"),
        );
        if (r.drag_stopped() || (r.changed() && !r.dragged())) && v != current {
            self.set_int(what, label, (v * 1000.0).round() as i64, FieldType::Dword);
        }
    }

    /// A colour stored as 0x00BBGGRR (ARE lighting), with a colour picker;
    /// what is picked is stored when the picker closes (one command).
    pub(crate) fn color(&mut self, ui: &mut Ui, what: &str, label: &str) {
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
        let pal = {
            let app = &mut *self.app;
            let game = game.or(app.game.as_ref());
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
                .selected_text(current.to_string())
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
                                if i == current {
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
        if let Some(v) = pick.filter(|&v| v != current) {
            self.set_int(what, label, i64::from(v), FieldType::Byte);
        }
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
        let current = self.int(label);
        let shown = choices
            .iter()
            .find(|c| c.row as i64 == current)
            .map_or_else(|| format!("({current})"), |c| c.text.clone());
        let filter_id = self.id(&format!("{label}#filter"));
        let mut pick = None;
        egui::ComboBox::from_id_salt(self.id(label)).selected_text(shown).width(220.0).show_ui(
            ui,
            |ui| {
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
                    if ui.selectable_label(c.row as i64 == current, &c.text).clicked() {
                        pick = Some(c.row as i64);
                    }
                }
            },
        );
        if let Some(v) = pick.filter(|&v| v != current) {
            self.set_int(what, label, v, default);
        }
    }

    /// A resource name with a picker.
    #[allow(dead_code)] // for the editors that follow
    pub(crate) fn resref(&mut self, ui: &mut Ui, what: &str, label: &str, types: &[ResType]) {
        let current = self.root.resref(label).unwrap_or(ResRef::EMPTY);
        if let Some(v) = resref_field(self.app, ui, self.id(label), current, what, types) {
            self.set(what, label, Value::resref(v));
        }
    }

    /// A script field: name, picker and Edit.
    pub(crate) fn script(&mut self, ui: &mut Ui, what: &str, label: &str) {
        let types = [ResType::NSS, ResType::NCS];
        self.linked(ui, &format!("{what} script"), label, &types, ResType::NSS, Tab::Script);
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
            let in_game = self.app.game.as_ref().is_some_and(|g| g.resman.contains(&key));
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
        let Some(game) = self.app.game.as_ref() else { return };
        let categories = game
            .resman
            .get_named(&format!("{}pal", kind.name()), ResType::ITP)
            .ok()
            .and_then(|d| mg_gff::Gff::read(&d).ok())
            .map(|g| Palette::read(&g).categories(game))
            .unwrap_or_default();
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
        if !r.clicked() {
            return;
        }
        let (Some(ws), Some(game)) = (self.app.ws.as_mut(), self.app.game.as_ref()) else { return };
        let Some(kind) = mg_area::ObjectKind::from_restype(self.key.restype) else { return };
        let template_field = BlueprintKind::from_restype(self.key.restype)
            .map_or("TemplateResRef", |k| k.resref_field());
        let areas = ws.module.areas().unwrap_or_default();
        let gits: Vec<(ResKey, Struct)> = areas
            .into_iter()
            .filter_map(|a| {
                let k = ResKey::new(a, ResType::GIT);
                ws.doc(&k).ok().map(|g| (k, g.root.clone()))
            })
            .collect();
        let module = &ws.module;
        let items = |r: ResRef| -> Option<Struct> {
            let k = ResKey::new(r, ResType::UTI);
            let data = module
                .get(&k)
                .map(<[u8]>::to_vec)
                .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()))?;
            mg_gff::Gff::read(&data).ok().map(|g| g.root)
        };
        let placing = mg_module::instances::Placing { game, item: &items };
        let mut edits = Vec::new();
        let mut updated = 0;
        for (git_key, git) in gits {
            let Some(list) = git.list(kind.list()) else { continue };
            let mut new = list.to_vec();
            let mut changed = false;
            for (i, s) in list.iter().enumerate() {
                if s.resref(template_field) != Some(self.key.resref) {
                    continue;
                }
                let o = mg_area::AreaObject::read(game, kind, i, s);
                let at = mg_module::instances::Placement {
                    position: o.position.to_array(),
                    rotation: o.rotation,
                };
                if let Some(made) =
                    mg_module::instances::instance(&placing, self.key.restype, &self.root, at, &[])
                {
                    new[i] = made;
                    changed = true;
                    updated += 1;
                }
            }
            if changed {
                edits.push(Edit::SetField {
                    key: git_key,
                    path: GffPath::root(),
                    label: kind.list().into(),
                    value: Some(Value::List(new)),
                });
            }
        }
        if edits.is_empty() {
            self.app.log.info(format!("No instances of {} to update", self.key.resref));
            return;
        }
        self.app.actions.push(crate::Action::Apply(mg_edit::Command::new(
            format!("Update instances of {}", self.key.resref),
            edits,
        )));
        self.app.log.info(format!("Updated {updated} instances of {}", self.key.resref));
    }

    /// The blueprint's resref (Aurora's "Blueprint ResRef"): changing it
    /// renames the blueprint.
    pub(crate) fn blueprint_resref(&mut self, ui: &mut Ui) {
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

pub(crate) fn after_apply(app: &mut Moonglow, cmd: &Command) {
    item::refresh_costs(app, cmd);
    creature::refresh_hit_points(app, cmd);
}

/// Renames a blueprint (its resource and `TemplateResRef`, a store's
/// `ResRef`) as one undoable
/// command, and points its editor at the new name.
pub(crate) fn rename(app: &mut Moonglow, from: ResKey, to: ResRef) {
    let Some(ws) = &mut app.ws else { return };
    let new = ResKey::new(to, from.restype);
    if ws.module.contains(&new) {
        app.log.error(format!("{new} already exists"));
        return;
    }
    let result = ws.flush().map_err(|e| e.to_string()).and_then(|()| {
        let mut g = ws.doc(&from).map_err(|e| e.to_string())?.clone();
        let field = BlueprintKind::from_restype(from.restype)
            .map_or("TemplateResRef", |k| k.resref_field());
        g.root.set(field, Value::resref(to));
        let data = g.to_bytes().map_err(|e| e.to_string())?;
        ws.apply(Command::new(
            format!("Rename {from} to {new}"),
            vec![
                Edit::SetResource { key: new, data: Some(data) },
                Edit::SetResource { key: from, data: None },
            ],
        ))
        .map_err(|e| e.to_string())
    });
    match result {
        Ok(()) => {
            for (_, tab) in app.dock.iter_all_tabs_mut() {
                if *tab == Tab::Blueprint(from) {
                    *tab = Tab::Blueprint(new);
                }
            }
            if let Some(p) = app.blueprint_pages.remove(&(from, GffPath::root())) {
                app.blueprint_pages.insert((new, GffPath::root()), p);
            }
        }
        Err(e) => app.log.error(e),
    }
}

/// A blueprint editor tab.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    edit(app, ui, key, GffPath::root());
}

/// The blueprint type of the object at `path` in a GIT (its list's).
pub(crate) fn instance_type(path: &GffPath) -> Option<ResType> {
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

/// The Properties of several objects of one type at once (Aurora's
/// multi-editor): shown as the first, each field changed set on all.
pub(crate) fn edit_many(
    app: &mut Moonglow,
    ui: &mut Ui,
    key: ResKey,
    path: GffPath,
    also: Vec<GffPath>,
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
    let pages: Vec<&str> =
        if path.0.is_empty() { pages(restype).to_vec() } else { instance_pages(restype) };
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
    if !also.is_empty() {
        ui.weak(format!(
            "{} objects: shown as the first; what you change is set on each",
            also.len() + 1
        ));
    }
    let mut form = Form { app, key, path, also, root };
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match restype {
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
    });
}
