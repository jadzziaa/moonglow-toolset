//! The Advanced Search tab (Edit › Advanced Search…): the module's
//! blueprints, placed objects, areas and the rest found by what they hold
//! (`mg_module::search`), listed by place as Find References lists them;
//! a click opens the object.

use egui::Ui;
use mg_core::{Codepage, ResType};
use mg_module::search::{
    self, AppearanceOf, FieldValue, Hit, PropertyParts, Query, Search, VarType,
};
use mg_rules::{Choice, ChoiceColumns, GameData};

use crate::Moonglow;

/// What the window looks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Variable,
    ItemProperty,
    Class,
    Feat,
    Spell,
    Skill,
    Appearance,
    Script,
    Field,
}

impl Kind {
    pub const ALL: [Kind; 9] = [
        Kind::Variable,
        Kind::ItemProperty,
        Kind::Class,
        Kind::Feat,
        Kind::Spell,
        Kind::Skill,
        Kind::Appearance,
        Kind::Script,
        Kind::Field,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Kind::Variable => "Variables",
            Kind::ItemProperty => "Item properties",
            Kind::Class => "Creatures by class",
            Kind::Feat => "Creatures by feat",
            Kind::Spell => "Creatures by spell",
            Kind::Skill => "Creatures by skill",
            Kind::Appearance => "Appearances",
            Kind::Script => "Scripts attached",
            Kind::Field => "Any field",
        }
    }
}

/// The kinds of object a search may be kept to.
const OBJECTS: [(ResType, &str); 12] = [
    (ResType::UTC, "Creatures"),
    (ResType::UTI, "Items"),
    (ResType::UTP, "Placeables"),
    (ResType::UTD, "Doors"),
    (ResType::UTT, "Triggers"),
    (ResType::UTE, "Encounters"),
    (ResType::UTM, "Stores"),
    (ResType::UTS, "Sounds"),
    (ResType::UTW, "Waypoints"),
    (ResType::ARE, "Areas"),
    (ResType::DLG, "Conversations"),
    (ResType::IFO, "The module"),
];

/// The Advanced Search window's state.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancedSearch {
    pub kind: Kind,
    /// A variable's or script's name, a field's label.
    pub name: String,
    pub var_type: Option<VarType>,
    /// A variable's value, a field's value.
    pub value: String,
    /// A field's value is a whole number (else text).
    pub number: bool,
    /// The row looked for: a class, feat, spell, skill, appearance, or an
    /// item property's type.
    pub row: Option<usize>,
    pub subtype: Option<usize>,
    pub cost: Option<usize>,
    pub param: Option<usize>,
    pub min_rank: u8,
    pub of: AppearanceOf,
    /// The kind of object looked at; none: all.
    pub object: Option<ResType>,
    /// Only placed objects, only blueprints.
    pub placed: Option<bool>,
    /// What was found, for the query in `searched`.
    pub hits: Vec<Hit>,
    pub searched: Option<Query>,
    /// The item property type's parts, for the type they are of.
    pub parts: Option<((usize, Option<usize>), PropertyParts)>,
}

impl Default for AdvancedSearch {
    fn default() -> AdvancedSearch {
        AdvancedSearch {
            kind: Kind::Variable,
            name: String::new(),
            var_type: None,
            value: String::new(),
            number: false,
            row: None,
            subtype: None,
            cost: None,
            param: None,
            min_rank: 1,
            of: AppearanceOf::Creature,
            object: None,
            placed: None,
            hits: Vec::new(),
            searched: None,
            parts: None,
        }
    }
}

impl AdvancedSearch {
    /// The query the window asks, or what it still needs.
    pub fn query(&self) -> Result<Query, String> {
        let name = self.name.trim();
        let value = self.value.trim();
        let row = |what: &str| self.row.ok_or_else(|| format!("Choose {what}"));
        let search = match self.kind {
            Kind::Variable => {
                if name.is_empty() {
                    return Err("Type a variable's name (* for any characters)".into());
                }
                Search::Variable {
                    name: name.to_string(),
                    kind: self.var_type,
                    value: (!value.is_empty()).then(|| value.to_string()),
                }
            }
            Kind::ItemProperty => Search::ItemProperty {
                property: row("a property")? as u16,
                subtype: self.subtype.map(|s| s as u16),
                value: self.cost.map(|s| s as u16),
                param: self.param.map(|s| s as u8),
            },
            Kind::Class => Search::Class(row("a class")? as i64),
            Kind::Feat => Search::Feat(row("a feat")? as i64),
            Kind::Spell => Search::Spell(row("a spell")? as i64),
            Kind::Skill => Search::Skill { skill: row("a skill")?, min_rank: self.min_rank },
            Kind::Appearance => {
                Search::Appearance { of: self.of, row: row("an appearance")? as i64 }
            }
            Kind::Script => {
                if name.is_empty() {
                    return Err("Type a script's name (* for any characters)".into());
                }
                Search::Script(name.to_string())
            }
            Kind::Field => {
                let value = match (value.is_empty(), self.number) {
                    (true, _) => None,
                    (false, true) => Some(FieldValue::Number(
                        value.parse().map_err(|_| format!("{value} is not a whole number"))?,
                    )),
                    (false, false) => Some(FieldValue::Text(value.to_string())),
                };
                if name.is_empty() && value.is_none() {
                    return Err("Type a field's label, a value, or both".into());
                }
                Search::Field { label: (!name.is_empty()).then(|| name.to_string()), value }
            }
        };
        Ok(Query { search, types: self.object.into_iter().collect(), placed: self.placed })
    }

    /// The table the row is chosen from.
    fn table(&self) -> Option<mg_rules::rows::RowTable> {
        let search = match self.kind {
            Kind::ItemProperty => {
                Search::ItemProperty { property: 0, subtype: None, value: None, param: None }
            }
            Kind::Class => Search::Class(0),
            Kind::Feat => Search::Feat(0),
            Kind::Spell => Search::Spell(0),
            Kind::Skill => Search::Skill { skill: 0, min_rank: 1 },
            Kind::Appearance => Search::Appearance { of: self.of, row: 0 },
            _ => return None,
        };
        search.table()
    }
}

impl Moonglow {
    /// Finds what the window asks in the module (its unsaved edits
    /// included).
    pub fn advanced_search(&mut self, draft: &mut AdvancedSearch) {
        let Ok(q) = draft.query() else { return };
        let Some(ws) = self.ws.as_mut() else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let game = self.game.as_deref();
        let cp = game.map_or(Codepage::WINDOWS_1252, GameData::codepage);
        draft.hits = search::search(&ws.module, &q, game, cp);
        draft.searched = Some(q);
    }
}

/// A row of a table, chosen by name (a long list has a filter); `any`
/// names the choice of none. With no names (no game data), a number.
fn row_picker(
    ui: &mut Ui,
    id: &str,
    choices: &[Choice],
    row: &mut Option<usize>,
    any: Option<&str>,
) {
    if choices.is_empty() {
        let mut on = row.is_some() || any.is_none();
        if any.is_some() {
            ui.checkbox(&mut on, "");
        }
        let mut n = row.unwrap_or(0);
        ui.add_enabled(on, egui::DragValue::new(&mut n).range(0..=65535))
            .on_hover_text("The row's number (the game's data isn't loaded to name it)");
        *row = on.then_some(n);
        return;
    }
    let mut number = |ui: &mut Ui, row: usize| {
        ui.label(format!("Row {row}"));
    };
    let current = row.map(|r| r as i64);
    let list = crate::widgets::row_choice(egui::Id::new(id), choices, current)
        .width(260.0)
        .about(&mut number);
    let list = match any {
        Some(any) => list.none(any),
        None => list.unset("Choose…"),
    };
    if let Some(picked) = list.show_optional(ui) {
        *row = picked.and_then(|r| usize::try_from(r).ok());
    }
}

/// The choices of a table, as the game names them.
fn choices(game: Option<&GameData>, table: &mg_rules::rows::RowTable) -> Vec<Choice> {
    let Some(game) = game else { return Vec::new() };
    let columns = ChoiceColumns { name: table.name, label: table.label };
    game.choice_list(table.table, columns).map(|c| c.to_vec()).unwrap_or_default()
}

/// What to look for, by kind.
fn search_ui(ui: &mut Ui, d: &mut AdvancedSearch, game: Option<&GameData>) -> bool {
    let mut find = false;
    let text = |ui: &mut Ui, value: &mut String, hint: &str| -> bool {
        let r = ui.add(egui::TextEdit::singleline(value).hint_text(hint).desired_width(260.0));
        r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
    };
    egui::Grid::new("advanced-search").num_columns(2).show(ui, |ui| {
        crate::widgets::field_label(ui, "Look for");
        let was = d.kind;
        egui::ComboBox::from_id_salt("advanced-search-kind")
            .selected_text(d.kind.label())
            .width(260.0)
            .show_ui(ui, |ui| {
                for k in Kind::ALL {
                    ui.selectable_value(&mut d.kind, k, k.label());
                }
            });
        if d.kind != was {
            d.row = None;
            d.subtype = None;
            d.cost = None;
            d.param = None;
        }
        ui.end_row();
        match d.kind {
            Kind::Variable => {
                crate::widgets::field_label(ui, "Name");
                find |= text(ui, &mut d.name, "e.g. *sDeidad* (* for any characters)");
                ui.end_row();
                crate::widgets::field_label(ui, "Type");
                egui::ComboBox::from_id_salt("advanced-search-var-type")
                    .selected_text(d.var_type.map_or("Any", VarType::name))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut d.var_type, None, "Any");
                        for t in VarType::ALL {
                            ui.selectable_value(&mut d.var_type, Some(t), t.name());
                        }
                    });
                ui.end_row();
                crate::widgets::field_label(ui, "Value");
                find |= text(ui, &mut d.value, "any (* for any characters)");
                ui.end_row();
            }
            Kind::ItemProperty => {
                let table = d.table().expect("a table");
                crate::widgets::field_label(ui, "Property");
                let was = d.row;
                row_picker(
                    ui,
                    "advanced-search-property",
                    &choices(game, &table),
                    &mut d.row,
                    None,
                );
                if d.row != was {
                    (d.subtype, d.cost, d.param) = (None, None, None);
                }
                ui.end_row();
                let parts = match (game, d.row) {
                    (Some(g), Some(p)) => {
                        let key = (p, d.subtype);
                        if d.parts.as_ref().is_none_or(|(k, _)| *k != key) {
                            let parts =
                                search::property_parts(g, p as u16, d.subtype.map(|s| s as u16));
                            d.parts = Some((key, parts));
                        }
                        d.parts.as_ref().map(|(_, p)| p.clone()).unwrap_or_default()
                    }
                    _ => PropertyParts::default(),
                };
                // (Without the game's data, every part by number.)
                let all = game.is_none();
                for (label, choices, row) in [
                    ("Subtype", &parts.subtypes, &mut d.subtype),
                    ("Value", &parts.values, &mut d.cost),
                    ("Parameter", &parts.params, &mut d.param),
                ] {
                    if all || !choices.is_empty() {
                        crate::widgets::field_label(ui, label);
                        let id = format!("advanced-search-{label}");
                        row_picker(ui, &id, choices, row, Some("Any"));
                        ui.end_row();
                    }
                }
            }
            Kind::Class | Kind::Feat | Kind::Spell | Kind::Skill | Kind::Appearance => {
                if d.kind == Kind::Appearance {
                    crate::widgets::field_label(ui, "Of");
                    let was = d.of;
                    egui::ComboBox::from_id_salt("advanced-search-of")
                        .selected_text(d.of.label())
                        .width(260.0)
                        .show_ui(ui, |ui| {
                            for a in AppearanceOf::ALL {
                                ui.selectable_value(&mut d.of, a, a.label());
                            }
                        });
                    if d.of != was {
                        d.row = None;
                    }
                    ui.end_row();
                }
                let label = match d.kind {
                    Kind::Class => "Class",
                    Kind::Feat => "Feat",
                    Kind::Spell => "Spell",
                    Kind::Skill => "Skill",
                    _ => "Appearance",
                };
                crate::widgets::field_label(ui, label);
                let table = d.table().expect("a table");
                row_picker(ui, "advanced-search-row", &choices(game, &table), &mut d.row, None);
                ui.end_row();
                if d.kind == Kind::Skill {
                    crate::widgets::field_label(ui, "At least");
                    ui.add(egui::DragValue::new(&mut d.min_rank).range(1..=127).suffix(" rank(s)"));
                    ui.end_row();
                }
            }
            Kind::Script => {
                crate::widgets::field_label(ui, "Script");
                find |= text(ui, &mut d.name, "e.g. x2_def_* (* for any characters)");
                ui.end_row();
            }
            Kind::Field => {
                crate::widgets::field_label(ui, "Label");
                find |= text(ui, &mut d.name, "any (* for any characters)");
                ui.end_row();
                crate::widgets::field_label(ui, "Value");
                ui.horizontal(|ui| {
                    find |= text(
                        ui,
                        &mut d.value,
                        if d.number { "e.g. 103" } else { "text it contains" },
                    );
                    ui.checkbox(&mut d.number, "Whole number")
                        .on_hover_text("Fields of every integer type holding exactly this number");
                });
                ui.end_row();
            }
        }
        crate::widgets::field_label(ui, "Look at");
        ui.horizontal(|ui| {
            let object =
                OBJECTS.iter().find(|(t, _)| Some(*t) == d.object).map_or("Everything", |o| o.1);
            egui::ComboBox::from_id_salt("advanced-search-object")
                .selected_text(object)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut d.object, None, "Everything");
                    for (t, label) in OBJECTS {
                        ui.selectable_value(&mut d.object, Some(t), label);
                    }
                })
                .response
                .on_hover_text("Items include those in inventories, equipment and stores");
            let placed = match d.placed {
                None => "blueprints and placed objects",
                Some(true) => "placed objects only",
                Some(false) => "blueprints only",
            };
            egui::ComboBox::from_id_salt("advanced-search-placed").selected_text(placed).show_ui(
                ui,
                |ui| {
                    ui.selectable_value(&mut d.placed, None, "blueprints and placed objects");
                    ui.selectable_value(&mut d.placed, Some(false), "blueprints only");
                    ui.selectable_value(&mut d.placed, Some(true), "placed objects only");
                },
            );
        });
        ui.end_row();
    });
    find
}

/// The Advanced Search tab.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    if app.ws.is_none() {
        ui.label("Open a module first.");
        return;
    }
    let mut draft = std::mem::take(&mut app.advanced_search);
    let (mut find, mut go) = (false, None);
    // The places, with areas named as the rest of the window names them
    // (Options › General: Show areas by name).
    let places: Vec<String> = draft
        .hits
        .iter()
        .map(|h| {
            let of_area = matches!(h.key.restype, ResType::ARE | ResType::GIT);
            if !of_area || !app.settings.area_names {
                return h.place.clone();
            }
            let name = crate::tabs::area_label(app, h.key.resref);
            h.place.replacen(&h.key.resref.to_lowercase().to_string(), &name, 1)
        })
        .collect();
    let game = app.game.clone();
    ui.weak(
        "The module's blueprints, placed objects (and the items they hold), areas and \
         conversations, by what they hold.",
    );
    if game.is_none() {
        ui.weak("The game's data isn't loaded: choose rows by number.");
    }
    find |= search_ui(ui, &mut draft, game.as_deref());
    let query = draft.query();
    ui.horizontal(|ui| {
        find |= ui.add_enabled(query.is_ok(), egui::Button::new("Find")).clicked();
        if let Err(why) = &query {
            ui.weak(why);
        }
    });
    let current = query.as_ref().ok() == draft.searched.as_ref();
    if draft.searched.is_some() && !current {
        ui.weak("Find again for what's asked now.");
    }
    if draft.searched.is_some() {
        ui.separator();
        if draft.hits.is_empty() {
            ui.weak("Nothing in the module holds it.");
        } else {
            ui.label(format!("{} found:", draft.hits.len()));
        }
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for (i, h) in draft.hits.iter().enumerate() {
                ui.horizontal(|ui| {
                    let hover = format!("{} {}", h.key, h.path);
                    if ui.link(&places[i]).on_hover_text(hover).clicked() {
                        go = Some(i);
                    }
                    ui.weak(&h.matched);
                });
            }
        });
    }
    if find && draft.query().is_ok() {
        app.advanced_search(&mut draft);
    }
    if let Some(i) = go {
        let usage = draft.hits[i].usage();
        app.go_to_usage(&usage);
    }
    app.advanced_search = draft;
}
