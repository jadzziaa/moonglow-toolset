//! Blueprint editors (Aurora's object Properties dialogs) for the module's
//! blueprints, one tab each. A [`Form`] lays out a blueprint's fields; each
//! change is one undoable command on the blueprint, fields keep the integer
//! type they are stored with, and fields the editor does not show are kept.

use std::collections::HashMap;

use egui::Ui;
use mg_core::{Gender, Language, LocString, ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{FieldType, Struct, Value};
use mg_module::palette::{BlueprintKind, Palette};
use mg_resman::ResKey;
use mg_rules::Choice;

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
pub type Pages = HashMap<ResKey, &'static str>;

/// A blueprint's fields, laid out.
pub(crate) struct Form<'a> {
    pub app: &'a mut Moonglow,
    pub key: ResKey,
    /// The blueprint as it is now.
    pub root: Struct,
}

fn english(ls: &LocString) -> String {
    ls.text(Language::ENGLISH, Gender::Male).map(|t| t.into_owned()).unwrap_or_default()
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
        egui::Id::new(("blueprint", self.key, name))
    }

    fn target(&self, label: &str) -> FieldTarget {
        FieldTarget::new(self.key, GffPath::root(), label)
    }

    /// Sets a field (one undoable command named `what`).
    pub(crate) fn set(&mut self, what: &str, label: &str, value: Value) {
        self.app.actions.push(Action::Apply(Command::new(
            what,
            vec![Edit::SetField {
                key: self.key,
                path: GffPath::root(),
                label: label.to_string(),
                value: Some(value),
            }],
        )));
    }

    /// Sets an integer field, keeping its stored type (for a field the
    /// blueprint lacks: the type the game's files give it, else `default`).
    pub(crate) fn set_int(&mut self, what: &str, label: &str, v: i64, default: FieldType) {
        let default = mg_schema::root_field_type(self.key.restype, label).unwrap_or(default);
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
            ui.label(format!("{what} (English)"));
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
        let edits = fields
            .into_iter()
            .map(|(label, value)| Edit::SetField {
                key: self.key,
                path: GffPath::root(),
                label: label.to_string(),
                value: Some(value),
            })
            .collect();
        self.app.actions.push(Action::Apply(Command::new(what, edits)));
    }

    /// Sets several integer fields in one command.
    pub(crate) fn set_many(&mut self, what: &str, fields: &[(&str, i64, FieldType)]) {
        let edits = fields
            .iter()
            .map(|(label, v, t)| Edit::SetField {
                key: self.key,
                path: GffPath::root(),
                label: label.to_string(),
                value: Some(integer(
                    self.root.get(label),
                    *v,
                    mg_schema::root_field_type(self.key.restype, label).unwrap_or(*t),
                )),
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

    /// The blueprint's resref (Aurora's "Blueprint ResRef"): changing it
    /// renames the blueprint.
    pub(crate) fn blueprint_resref(&mut self, ui: &mut Ui) {
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
            if let Some(p) = app.blueprint_pages.remove(&from) {
                app.blueprint_pages.insert(new, p);
            }
        }
        Err(e) => app.log.error(e),
    }
}

/// A blueprint editor tab.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let Some(ws) = &mut app.ws else {
        ui.label("No module is open.");
        return;
    };
    let root = match ws.doc(&key) {
        Ok(g) => g.root.clone(),
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return;
        }
    };
    let pages = pages(key.restype);
    let mut page = app.blueprint_pages.get(&key).copied().unwrap_or(pages[0]);
    ui.horizontal_wrapped(|ui| {
        for p in pages {
            ui.selectable_value(&mut page, p, *p);
        }
    });
    app.blueprint_pages.insert(key, page);
    ui.separator();
    let mut form = Form { app, key, root };
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match key.restype {
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
