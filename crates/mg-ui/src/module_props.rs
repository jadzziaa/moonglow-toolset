//! Module Properties (module.ifo), with Aurora's tabs: Basic, Events,
//! Advanced, Description and Custom Content. Every change is one undoable
//! command.

use egui::Ui;
use mg_core::{Gender, Language, LocString, ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_resman::{GameInstall, ResKey};
use mg_schema::{ExoString, Field, GffValue, StructExt, ifo};

use crate::text::{decode, encode, from_editor, to_editor, with_english};
use crate::widgets::{FieldTarget, LocStringEdit, resref_field, variables_button};
use crate::{Action, Moonglow, Tab};

fn info_key() -> ResKey {
    ResKey::parse("module", ResType::IFO).expect("valid")
}

fn target(label: &str) -> FieldTarget {
    FieldTarget::new(info_key(), GffPath::root(), label)
}

/// A command setting one module-info field.
fn set<T: GffValue>(label: &str, f: &Field<T>, v: T) -> Action {
    set_value(label, f.label, v.into_value())
}

fn set_value(what: &str, label: &str, value: Value) -> Action {
    Action::Apply(Command::new(
        what,
        vec![Edit::SetField {
            key: info_key(),
            path: GffPath::root(),
            label: label.to_string(),
            value: Some(value),
        }],
    ))
}

/// A one-line text field that commits when focus leaves it.
fn text_field(app: &mut Moonglow, ui: &mut Ui, id: &str, current: &str) -> Option<String> {
    edit_field(app, ui, id, current, false)
}

/// A text field (one line or several) that commits when focus leaves it.
/// Multi-line text keeps the line-end style it had (CRLF or LF).
fn edit_field(
    app: &mut Moonglow,
    ui: &mut Ui,
    id: &str,
    current: &str,
    multiline: bool,
) -> Option<String> {
    let id = egui::Id::new(("ifo", id));
    let (shown, crlf) = to_editor(current);
    let buf = app.buffers.entry(id).or_insert_with(|| shown.clone());
    let edit = if multiline {
        egui::TextEdit::multiline(buf).desired_rows(12).desired_width(f32::INFINITY)
    } else {
        egui::TextEdit::singleline(buf).desired_width(320.0)
    };
    let r = ui.add(edit.id(id));
    if !r.has_focus() && !r.lost_focus() && *buf != shown {
        // Changed underneath (undo): show the document's value.
        *buf = shown.clone();
    }
    (r.lost_focus() && *buf != shown).then(|| from_editor(buf, crlf))
}

/// A number with a range, committed when it changes (a drag: when released).
fn number<T: egui::emath::Numeric + GffValue>(
    app: &mut Moonglow,
    ui: &mut Ui,
    what: &str,
    f: &Field<T>,
    current: T,
    range: std::ops::RangeInclusive<T>,
) {
    let mut v = current;
    let r = ui.add(egui::DragValue::new(&mut v).range(range).clamp_existing_to_range(false));
    if (r.drag_stopped() || (r.changed() && !r.dragged())) && v != current {
        app.actions.push(set(what, f, v));
    }
}

/// The dialog's tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Basic,
    Events,
    Advanced,
    Description,
    CustomContent,
}

/// Module events in Aurora's order, with its labels. Aurora hides
/// OnModuleStart; it is shown here only when set.
pub(crate) const EVENTS: [(&str, &str); 22] = [
    ("OnClientEnter", "Mod_OnClientEntr"),
    ("OnModuleLoad", "Mod_OnModLoad"),
    ("OnHeartbeat", "Mod_OnHeartbeat"),
    ("OnUserDefined", "Mod_OnUsrDefined"),
    ("OnClientLeave", "Mod_OnClientLeav"),
    ("OnActivateItem", "Mod_OnActvtItem"),
    ("OnAcquireItem", "Mod_OnAcquirItem"),
    ("OnUnAcquireItem", "Mod_OnUnAqreItem"),
    ("OnPlayerDeath", "Mod_OnPlrDeath"),
    ("OnPlayerDying", "Mod_OnPlrDying"),
    ("OnPlayerRespawn", "Mod_OnSpawnBtnDn"),
    ("OnPlayerRest", "Mod_OnPlrRest"),
    ("OnPlayerLevelUp", "Mod_OnPlrLvlUp"),
    ("OnCutsceneAbort", "Mod_OnCutsnAbort"),
    ("OnPlayerEquipItem", "Mod_OnPlrEqItm"),
    ("OnPlayerUnEquipItem", "Mod_OnPlrUnEqItm"),
    ("OnPlayerChat", "Mod_OnPlrChat"),
    ("OnNuiEvent", "Mod_OnNuiEvent"),
    ("OnPlayerGuiEvent", "Mod_OnPlrGuiEvt"),
    ("OnPlayerTarget", "Mod_OnPlrTarget"),
    ("OnPlayerTileAction", "Mod_OnPlrTileAct"),
    ("OnModuleStart", "Mod_OnModStart"),
];

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let Some(ws) = &mut app.ws else {
        ui.label("No module is open.");
        return;
    };
    let root: Struct = match ws.doc(&info_key()) {
        Ok(g) => g.root.clone(),
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return;
        }
    };
    ui.horizontal(|ui| {
        for (page, label) in [
            (Page::Basic, "Basic"),
            (Page::Events, "Events"),
            (Page::Advanced, "Advanced"),
            (Page::Description, "Description"),
            (Page::CustomContent, "Custom Content"),
        ] {
            ui.selectable_value(&mut app.module_page, page, label);
        }
    });
    ui.separator();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match app.module_page {
        Page::Basic => basic(app, ui, &root),
        Page::Events => events(app, ui, &root),
        Page::Advanced => advanced(app, ui, &root),
        Page::Description => description(app, ui, &root),
        Page::CustomContent => custom_content(app, ui, &root),
    });
}

fn english(ls: &LocString) -> String {
    ls.text(Language::ENGLISH, Gender::Male).map(|t| t.into_owned()).unwrap_or_default()
}

fn basic(app: &mut Moonglow, ui: &mut Ui, root: &Struct) {
    egui::Grid::new("ifo-basic").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Name");
        let name: LocString = root.read(&ifo::MOD_NAME);
        ui.horizontal(|ui| {
            if let Some(v) = text_field(app, ui, "name", &english(&name)) {
                app.actions.push(set(
                    "Module name",
                    &ifo::MOD_NAME,
                    with_english(name.clone(), &v),
                ));
            }
            if ui.small_button("…").on_hover_text("Edit text in multiple languages").clicked() {
                app.loc_edit =
                    Some(LocStringEdit::new(target(ifo::MOD_NAME.label), "Module name", &name));
            }
        });
        ui.end_row();

        ui.label("Tag");
        let tag = decode(root.read(&ifo::MOD_TAG).as_bytes());
        if let Some(v) = text_field(app, ui, "tag", &tag) {
            app.actions.push(set("Module tag", &ifo::MOD_TAG, ExoString(encode(&v))));
        }
        ui.end_row();

        // Set by placing the start location in an area, as in Aurora.
        ui.label("Start area");
        ui.label(root.read(&ifo::MOD_ENTRY_AREA).to_string());
        ui.end_row();
        ui.label("Start location");
        ui.label(format!(
            "{:.2}, {:.2}, {:.2}",
            root.read(&ifo::MOD_ENTRY_X),
            root.read(&ifo::MOD_ENTRY_Y),
            root.read(&ifo::MOD_ENTRY_Z)
        ));
        ui.end_row();
    });
}

fn events(app: &mut Moonglow, ui: &mut Ui, root: &Struct) {
    egui::Grid::new("ifo-events").num_columns(3).spacing([12.0, 4.0]).show(ui, |ui| {
        for (label, field) in EVENTS {
            let current = root.resref(field).unwrap_or(ResRef::EMPTY);
            if field == "Mod_OnModStart" && current.is_empty() {
                continue;
            }
            ui.label(label);
            let id = egui::Id::new(("ifo-event", field));
            let types = [ResType::NSS, ResType::NCS];
            if let Some(v) = resref_field(app, ui, id, current, "Select a script", &types) {
                app.actions.push(set_value(&format!("{label} script"), field, Value::resref(v)));
            }
            let key = ResKey::new(current, ResType::NSS);
            let in_module = app.ws.as_ref().is_some_and(|w| w.module.contains(&key));
            let in_game = app.game.as_ref().is_some_and(|g| g.resman.contains(&key));
            let can_edit = !current.is_empty() && (in_module || in_game);
            if ui.add_enabled(can_edit, egui::Button::new("Edit").small()).clicked() {
                let tab = if in_module { Tab::Script(key) } else { Tab::Resource(key) };
                app.actions.push(Action::OpenTab(tab));
            }
            ui.end_row();
        }
    });
}

fn advanced(app: &mut Moonglow, ui: &mut Ui, root: &Struct) {
    egui::Grid::new("ifo-advanced").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        let rows: [(&str, &Field<u8>, std::ops::RangeInclusive<u8>); 6] = [
            ("Starting month", &ifo::MOD_START_MONTH, 1..=12),
            ("Starting day", &ifo::MOD_START_DAY, 1..=31),
            ("Starting hour", &ifo::MOD_START_HOUR, 0..=23),
            ("Minutes per hour", &ifo::MOD_MIN_PER_HOUR, 1..=240),
            ("Dawn start hour", &ifo::MOD_DAWN_HOUR, 0..=23),
            ("Dusk start hour", &ifo::MOD_DUSK_HOUR, 0..=23),
        ];
        for (label, f, range) in rows {
            ui.label(label);
            number(app, ui, label, f, root.read(f), range);
            ui.end_row();
        }
        ui.label("Starting year");
        number(
            app,
            ui,
            "Starting year",
            &ifo::MOD_START_YEAR,
            root.read(&ifo::MOD_START_YEAR),
            0..=30000,
        );
        ui.end_row();

        ui.label("XP scale");
        let mut xp = root.read(&ifo::MOD_XP_SCALE);
        let r = ui.add(egui::Slider::new(&mut xp, 0..=200).clamping(egui::SliderClamping::Edits));
        if r.drag_stopped() || (r.changed() && !r.dragged()) {
            app.actions.push(set("Experience scale", &ifo::MOD_XP_SCALE, xp));
        }
        ui.end_row();

        ui.label("Starting movie");
        let movie = root.read(&ifo::MOD_START_MOVIE);
        let movies = app
            .install
            .as_ref()
            .map(|i| GameInstall::file_names(&i.movie_dirs(), "bik"))
            .unwrap_or_default();
        let shown = if movie.is_empty() { "(none)".to_string() } else { movie.to_string() };
        egui::ComboBox::from_id_salt("ifo-movie").selected_text(shown).show_ui(ui, |ui| {
            if ui.selectable_label(movie.is_empty(), "(none)").clicked() && !movie.is_empty() {
                app.actions.push(set("Starting movie", &ifo::MOD_START_MOVIE, ResRef::EMPTY));
            }
            for m in &movies {
                if let Ok(r) = ResRef::from_str(m)
                    && ui.selectable_label(r == movie, m).clicked()
                    && r != movie
                {
                    app.actions.push(set("Starting movie", &ifo::MOD_START_MOVIE, r));
                }
            }
        });
        ui.end_row();

        ui.label("Variables");
        variables_button(app, ui, target(ifo::VAR_TABLE.label), root.items(&ifo::VAR_TABLE));
        ui.end_row();
    });
}

fn description(app: &mut Moonglow, ui: &mut Ui, root: &Struct) {
    let desc: LocString = root.read(&ifo::MOD_DESCRIPTION);
    ui.horizontal(|ui| {
        ui.label("Description (English)");
        if ui.small_button("…").on_hover_text("Edit text in multiple languages").clicked() {
            let t = target(ifo::MOD_DESCRIPTION.label);
            app.loc_edit = Some(LocStringEdit::new(t, "Module description", &desc));
        }
    });
    if let Some(v) = edit_field(app, ui, "desc", &english(&desc), true) {
        app.actions.push(set("Module description", &ifo::MOD_DESCRIPTION, with_english(desc, &v)));
    }
}

fn custom_content(app: &mut Moonglow, ui: &mut Ui, root: &Struct) {
    let items = root.items(&ifo::MOD_HAK_LIST);
    let haks: Vec<String> =
        items.iter().map(|h| decode(h.read(&ifo::mod_hak_list::MOD_HAK).as_bytes())).collect();
    ui.label("Hak paks, highest priority first. Changes apply when the module is reopened.");
    let list = || ifo::MOD_HAK_LIST.label.to_string();
    let command = |edits: Vec<Edit>, label: &str| Action::Apply(Command::new(label, edits));
    for (i, h) in haks.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.monospace(h);
            let item = items[i].clone();
            let remove =
                Edit::RemoveItem { key: info_key(), path: GffPath::root(), list: list(), index: i };
            let insert_at = |index: usize| Edit::InsertItem {
                key: info_key(),
                path: GffPath::root(),
                list: list(),
                index,
                item: item.clone(),
            };
            if ui.add_enabled(i > 0, egui::Button::new("Move Up").small()).clicked() {
                app.actions.push(command(vec![remove.clone(), insert_at(i - 1)], "Move hak up"));
            }
            if ui.add_enabled(i + 1 < haks.len(), egui::Button::new("Move Down").small()).clicked()
            {
                app.actions.push(command(vec![remove.clone(), insert_at(i + 1)], "Move hak down"));
            }
            if ui.small_button("Remove").clicked() {
                app.actions.push(command(vec![remove], "Remove hak"));
            }
        });
    }
    let available: Vec<String> = app
        .install
        .as_ref()
        .map(|i| GameInstall::file_names(&i.hak_dirs(), "hak"))
        .unwrap_or_default()
        .into_iter()
        .filter(|h| !haks.iter().any(|x| x.eq_ignore_ascii_case(h)))
        .collect();
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("ifo-add-hak").selected_text("Add hak…").show_ui(ui, |ui| {
            for h in &available {
                if ui.selectable_label(false, h).clicked() {
                    let mut item = ifo::MOD_HAK_LIST.new_item();
                    item.write(&ifo::mod_hak_list::MOD_HAK, ExoString(encode(h)));
                    let insert = Edit::InsertItem {
                        key: info_key(),
                        path: GffPath::root(),
                        list: list(),
                        index: haks.len(),
                        item,
                    };
                    app.actions.push(command(vec![insert], "Add hak"));
                }
            }
        });
        if ui.add_enabled(!haks.is_empty(), egui::Button::new("Check for Conflicts…")).clicked() {
            app.actions.push(Action::HakReport);
        }
    });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label("Custom TLK");
        let tlk = decode(root.read(&ifo::MOD_CUSTOM_TLK).as_bytes());
        let tlks = app
            .install
            .as_ref()
            .map(|i| GameInstall::file_names(&i.tlk_dirs(), "tlk"))
            .unwrap_or_default();
        let shown = if tlk.is_empty() { "(none)".to_string() } else { tlk.clone() };
        egui::ComboBox::from_id_salt("ifo-tlk").selected_text(shown).show_ui(ui, |ui| {
            let mut chosen = None;
            if ui.selectable_label(tlk.is_empty(), "(none)").clicked() {
                chosen = Some(String::new());
            }
            for t in &tlks {
                if ui.selectable_label(t.eq_ignore_ascii_case(&tlk), t).clicked() {
                    chosen = Some(t.clone());
                }
            }
            if let Some(name) = chosen.filter(|n| !n.eq_ignore_ascii_case(&tlk)) {
                app.actions.push(set("Custom TLK", &ifo::MOD_CUSTOM_TLK, ExoString(encode(&name))));
            }
        });
    });
}
