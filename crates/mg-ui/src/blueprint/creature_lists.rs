//! The creature editor's list pages: Feats, Spells (per class, known or
//! prepared by level), Special Abilities, and the inventory (Aurora's
//! Inventory… dialog: equipment slots and the backpack).

use egui::Ui;
use mg_core::ResRef;
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_module::script_set::ListedSpell;
use mg_resman::ResKey;
use mg_rules::{ChoiceColumns, GameData};

use super::{Form, inventory};
use crate::Action;
use crate::dialogs::FileKind;
use crate::widgets::{commit_number, spin_number};

/// Struct ids of the creature's list entries (as the toolset writes them).
const FEAT_ID: u32 = 1;
const SPELL_ID: u32 = 3;
const SPECIAL_ID: u32 = 4;

fn apply(f: &mut Form<'_>, what: &str, edits: Vec<Edit>) {
    if !edits.is_empty() {
        f.app.actions.push(Action::Apply(Command::new(what, edits)));
    }
}

fn insert(key: ResKey, path: GffPath, list: &str, index: usize, item: Struct) -> Edit {
    Edit::insert_item(key, path, list, index, item)
}

fn remove(key: ResKey, path: GffPath, list: &str, index: usize) -> Edit {
    Edit::remove_item(key, path, list, index)
}

/// A text filter kept per page.
fn filter(ui: &mut Ui, id: egui::Id) -> mg_core::text::Needle {
    let mut text: String = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    ui.add(egui::TextEdit::singleline(&mut text).hint_text("Find").desired_width(200.0));
    ui.data_mut(|d| d.insert_temp(id, text.clone()));
    mg_core::text::Needle::new(&text)
}

/// Whether a feat's or spell's row is one to offer: it has a name in the
/// talk table, and isn't one the game's tables mark as gone by its label
/// (`DELETED`, `DELETED_PRO_Cold`, `Padding`). The game's feat.2da and
/// spells.2da keep such rows so the rows after them keep their numbers; a
/// creature that has one still shows it.
pub(crate) fn offered(table: &mg_2da::TwoDa, row: usize, name: &str, label: &str) -> bool {
    let gone = table.get(row, label).is_some_and(|l| {
        let l = l.to_ascii_lowercase();
        l.starts_with("deleted") || l.starts_with("padding")
    });
    !gone && table.get_int(row, name).is_some()
}

/// feat.2da `TOOLSCATEGORIES`: the toolset's categories of feats.
const FEAT_CATEGORIES: [(i32, &str); 6] = [
    (1, "Combat"),
    (2, "Active Combat"),
    (3, "Defensive"),
    (4, "Magical"),
    (5, "Class / Racial"),
    (6, "Other"),
];

/// A talent category's name as a list shows it: categories.2da's
/// `Harmful_AOE_Discriminant` and `TALENT_CATEGORY_BENEFICIAL_SUMMON` are
/// "Harmful AOE Discriminant" and "Beneficial Summon".
pub(crate) fn category_name(label: &str) -> String {
    let label = label.trim();
    let label =
        label.strip_prefix("TALENT_CATEGORY_").or(label.strip_prefix("TALENT_")).unwrap_or(label);
    label
        .split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            // (Acronyms stay as they are: AOE.)
            if w.len() <= 3 && w.chars().all(|c| c.is_ascii_uppercase()) {
                return w.to_string();
            }
            let mut c = w.chars();
            let first = c.next().map(|f| f.to_ascii_uppercase()).unwrap_or_default();
            format!("{first}{}", c.as_str().to_ascii_lowercase())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The talent categories spells have (categories.2da), by name.
fn spell_categories(game: &GameData) -> Vec<(i32, String)> {
    let Ok(t) = game.table("categories") else { return Vec::new() };
    let mut out: Vec<(i32, String)> = (0..t.len())
        .filter_map(|r| {
            let label =
                t.get(r, "Category").map(str::trim).filter(|l| !l.is_empty() && *l != "****")?;
            Some((r as i32, category_name(label)))
        })
        .collect();
    out.sort_by_key(|(_, name)| name.to_lowercase());
    out
}

/// A list's category filter, kept per page: any, or one of `categories`.
fn category_filter(ui: &mut Ui, id: egui::Id, categories: &[(i32, String)]) -> Option<i32> {
    let mut chosen: Option<i32> = ui.data(|d| d.get_temp(id)).unwrap_or(None);
    let name = |c: Option<i32>| match c {
        None => "Any category".to_string(),
        Some(c) => categories
            .iter()
            .find(|(n, _)| *n == c)
            .map_or_else(|| c.to_string(), |(_, name)| name.clone()),
    };
    let options: Vec<Option<i32>> =
        std::iter::once(None).chain(categories.iter().map(|(c, _)| Some(*c))).collect();
    crate::widgets::dropdown(id).selected_text(name(chosen)).width(200.0).show(
        ui,
        &mut chosen,
        &options,
        |c| name(*c),
    );
    ui.data_mut(|d| d.insert_temp(id, chosen));
    chosen
}

/// What the game says of row `row` of `table` (the talk-table string its
/// `column` names: a feat's, a spell's or a skill's description), shown
/// when `r` is hovered, as Aurora shows it with F1.
pub(super) fn described(
    r: egui::Response,
    game: Option<&GameData>,
    table: &str,
    column: &str,
    row: usize,
) -> egui::Response {
    r.on_hover_ui(|ui| {
        let text = game.and_then(|g| {
            let strref = g.table(table).ok()?.get_int(row, column)?;
            g.string(mg_core::StrRef(u32::try_from(strref).ok()?))
        });
        ui.set_max_width(440.0);
        // Which row it is: several feats and spells share a name.
        if let Some(t) = game.and_then(|g| g.table(table).ok()) {
            ui.weak(row_line(&t, table, row));
        }
        match text.filter(|t| !t.trim().is_empty()) {
            Some(text) => ui.label(text),
            None => ui.weak("The game has no description of it."),
        };
    })
}

/// A row's number and label, and a spell's innate level:
/// "spells.2da row 52: Elemental_Shield, innate level 4".
fn row_line(t: &mg_2da::TwoDa, table: &str, row: usize) -> String {
    let label = t.get(row, "Label").or_else(|| t.get(row, "LABEL"));
    let mut line = format!("{table}.2da row {row}");
    if let Some(l) = label {
        line += &format!(": {l}");
    }
    if let Some(level) = t.get_int(row, "Innate").filter(|_| table == "spells") {
        line += &format!(", innate level {level}");
    }
    line
}

pub(super) fn feats(f: &mut Form<'_>, ui: &mut Ui) {
    let described_by = f.app.game.clone();
    let base = f.path.clone();
    let key = f.key;
    let all = mg_rules::by_name(f.choices("feat", Some("FEAT"), Some("LABEL")).to_vec());
    let list: Vec<i64> =
        f.root.list("FeatList").unwrap_or(&[]).iter().filter_map(|s| s.integer("Feat")).collect();
    let id = egui::Id::new(("utc-feats", key));
    let mut assigned_only: bool = ui.data(|d| d.get_temp(id.with("only"))).unwrap_or(false);
    let feat_table = f.app.game.as_ref().and_then(|g| g.table("feat").ok());
    let categories: Vec<(i32, String)> =
        FEAT_CATEGORIES.iter().map(|(c, n)| (*c, n.to_string())).collect();
    let (needle, category) = ui
        .horizontal_wrapped(|ui| {
            let t = filter(ui, id);
            let c = category_filter(ui, id.with("category"), &categories);
            ui.checkbox(&mut assigned_only, "Assigned only");
            ui.label(format!("{} feats assigned", list.len()));
            (t, c)
        })
        .inner;
    let in_category = |row: usize| {
        category.is_none_or(|c| {
            feat_table.as_ref().and_then(|t| t.get_int(row, "TOOLSCATEGORIES")) == Some(c)
        })
    };
    ui.data_mut(|d| d.insert_temp(id.with("only"), assigned_only));
    let mut toggle = None;
    // The feats to choose from, and beside them those the creature has
    // (as the Special Abilities page lists its own): a click on one there
    // takes it away.
    ui.columns(2, |columns| {
        let ui = &mut columns[0];
        egui::ScrollArea::vertical().id_salt(("utc-feat-list", key)).auto_shrink(false).show(
            ui,
            |ui| {
                for c in &all {
                    let on = list.contains(&(c.row as i64));
                    let offer =
                        feat_table.as_ref().is_none_or(|t| offered(t, c.row, "FEAT", "LABEL"));
                    if (assigned_only && !on)
                        || (!on && !offer)
                        || !needle.finds(&c.text)
                        || !in_category(c.row)
                    {
                        continue;
                    }
                    let mut v = on;
                    let r = ui.checkbox(&mut v, &c.text);
                    if described(r, described_by.as_deref(), "feat", "DESCRIPTION", c.row).changed()
                    {
                        toggle = Some((c.row, v));
                    }
                }
            },
        );
        let ui = &mut columns[1];
        ui.strong("Assigned");
        egui::ScrollArea::vertical().id_salt(("utc-feat-assigned", key)).auto_shrink(false).show(
            ui,
            |ui| {
                // By name, as the list beside it; one the table has no
                // name for by its row.
                let mut assigned: Vec<(String, usize)> = list
                    .iter()
                    .filter_map(|&row| usize::try_from(row).ok())
                    .map(|row| {
                        let name = all
                            .iter()
                            .find(|c| c.row == row)
                            .map_or_else(|| format!("Feat {row}"), |c| c.text.clone());
                        (name, row)
                    })
                    .collect();
                assigned.sort_by_key(|(name, _)| name.to_lowercase());
                if assigned.is_empty() {
                    ui.weak("None: tick a feat to give it to the creature.");
                }
                for (name, row) in assigned {
                    ui.horizontal(|ui| {
                        if ui.small_button("×").on_hover_text("Take this feat away").clicked() {
                            toggle = Some((row, false));
                        }
                        let r = ui.label(name);
                        described(r, described_by.as_deref(), "feat", "DESCRIPTION", row);
                    });
                }
            },
        );
    });
    match toggle {
        Some((row, true)) => {
            let mut s = Struct::new(FEAT_ID);
            s.set("Feat", Value::Word(row as u16));
            apply(f, "Add feat", vec![insert(key, base, "FeatList", list.len(), s)]);
        }
        Some((row, false)) => {
            if let Some(i) = list.iter().position(|&r| r == row as i64) {
                apply(f, "Remove feat", vec![remove(key, base, "FeatList", i)]);
            }
        }
        None => {}
    }
}

/// A class's spells: (spell row, name, level), from its spells.2da column.
fn class_spells(game: &GameData, class: usize) -> Vec<(usize, String, usize)> {
    let Ok(classes) = game.table("classes") else { return Vec::new() };
    let Some(column) = classes.get(class, "SpellTableColumn").map(str::to_string) else {
        return Vec::new();
    };
    let names = game
        .choices("spells", ChoiceColumns { name: Some("Name"), label: Some("Label") })
        .map(mg_rules::by_name)
        .unwrap_or_default();
    let Ok(spells) = game.table("spells") else { return Vec::new() };
    names
        .into_iter()
        .filter(|c| offered(&spells, c.row, "Name", "Label"))
        .filter_map(|c| {
            let level = spells.get_int(c.row, &column)?;
            Some((c.row, c.text, level.clamp(0, 9) as usize))
        })
        .collect()
}

/// A warning without Aurora's closing question ("Do you wish to
/// proceed?"): Moonglow's pages state it instead.
fn statement(text: &str) -> &str {
    let t = text.trim_end();
    match t.strip_suffix('?').and_then(|q| q.rfind(['.', '!'])) {
        Some(i) => &t[..=i],
        None => t,
    }
}

/// `SpellMetaMagic`'s bits and what a spell so cast is called.
/// `SpellMetaMagic`'s bits, the feat (feat.2da) that gives each and its
/// English name.
const METAMAGIC: [(u8, usize, &str); 6] = [
    (1, 11, "Empower Spell"),
    (2, 12, "Extend Spell"),
    (4, 25, "Maximize Spell"),
    (8, 29, "Quicken Spell"),
    (16, 33, "Silent Spell"),
    (32, 37, "Still Spell"),
];

/// A metamagic's name, as Aurora names it: its feat's.
fn metamagic_name(game: &GameData, bit: u8) -> String {
    let Some(&(_, feat, english)) = METAMAGIC.iter().find(|(b, _, _)| *b == bit) else {
        return format!("Metamagic {bit}");
    };
    let name = game.table("feat").ok().and_then(|t| t.get_int(feat, "FEAT"));
    name.and_then(|s| game.string(mg_core::StrRef(u32::try_from(s).ok()?)))
        .unwrap_or_else(|| english.to_string())
}

/// A spell's name with its metamagic, as Aurora's summary has it: "Bless
/// (Empower Spell)".
fn with_metamagic(game: &GameData, name: &str, metamagic: u8) -> String {
    let how: Vec<String> = METAMAGIC
        .iter()
        .filter(|(bit, _, _)| metamagic & bit != 0)
        .map(|(bit, _, _)| metamagic_name(game, *bit))
        .collect();
    if how.is_empty() { name.to_string() } else { format!("{name} ({})", how.join(", ")) }
}

/// The entries of a class's spell list grouped as the page shows them:
/// those of one spell and metamagic are one row, so many prepared (or
/// known). Each row has its entries' places in the list, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellGroup {
    pub spell: i64,
    pub metamagic: u8,
    pub entries: Vec<usize>,
}

pub(crate) fn spell_groups(list: &[Struct]) -> Vec<SpellGroup> {
    let mut groups: Vec<SpellGroup> = Vec::new();
    for (i, s) in list.iter().enumerate() {
        let spell = s.integer("Spell").unwrap_or(0);
        let metamagic = s.integer("SpellMetaMagic").unwrap_or(0).clamp(0, 255) as u8;
        match groups.iter_mut().find(|g| (g.spell, g.metamagic) == (spell, metamagic)) {
            Some(g) => g.entries.push(i),
            None => groups.push(SpellGroup { spell, metamagic, entries: vec![i] }),
        }
    }
    groups
}

/// A new entry of a class's spell list, as the toolset writes one.
fn spell_entry(spell: usize, metamagic: u8) -> Struct {
    let mut s = Struct::new(SPELL_ID);
    s.set("Spell", Value::Word(spell as u16));
    s.set("SpellFlags", Value::Byte(1));
    s.set("SpellMetaMagic", Value::Byte(metamagic));
    s
}

/// The Spells page, laid out as the Feats page: the spells of the chosen
/// class to choose from (by level, name and category) beside those it
/// knows or has prepared, by level, with how many it may have of each.
pub(super) fn spells(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    let key = f.key;
    let Some(game) = f.app.game.clone() else { return };
    let game = game.as_ref();
    let Ok(classes) = game.table("classes") else { return };
    let class_list: Vec<Struct> = f.root.list("ClassList").unwrap_or(&[]).to_vec();
    // The creature's spellcasting classes: (index in ClassList, class row,
    // name, what it may have).
    let casters: Vec<(usize, usize, String, mg_rules::SpellSlots)> = class_list
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let row = c.integer("Class")?.max(0) as usize;
            let slots = game.spell_slots(&f.root, c)?;
            let name = classes
                .get_int(row, "Name")
                .and_then(|s| game.string(mg_core::StrRef(s as u32)))
                .unwrap_or_else(|| classes.get(row, "Label").unwrap_or_default().to_string());
            Some((i, row, name, slots))
        })
        .collect();
    if casters.is_empty() {
        ui.weak("None of the creature's classes casts spells.");
        return;
    }
    // Aurora's warnings on closing the creature's properties (Options ›
    // General), here while the spells are edited.
    if !f.app.settings.no_spell_warning {
        let warnings = game.spell_warnings(&f.root);
        if !warnings.is_empty() {
            let color = ui.visuals().warn_fg_color;
            for w in &warnings {
                ui.colored_label(color, statement(&game.spell_warning_text(w)));
            }
            if ui
                .small_button("Never warn again")
                .on_hover_text("Options › General: Show invalid creature spell assignment warning")
                .clicked()
            {
                f.app.settings.no_spell_warning = true;
            }
        }
    }
    let id = egui::Id::new(("utc-spells", key));
    let (mut which, mut level): (usize, Option<usize>) =
        ui.data(|d| d.get_temp(id)).unwrap_or_default();
    which = which.min(casters.len() - 1);
    ui.horizontal_wrapped(|ui| {
        crate::widgets::field_label(ui, "Class");
        for (n, (_, _, name, _)) in casters.iter().enumerate() {
            ui.radio_value(&mut which, n, name);
        }
    });
    ui.horizontal_wrapped(|ui| {
        crate::widgets::field_label(ui, "Spell Level");
        ui.selectable_value(&mut level, None, "All");
        for l in 0..=9 {
            ui.selectable_value(&mut level, Some(l), l.to_string());
        }
    });
    ui.data_mut(|d| d.insert_temp(id, (which, level)));
    // Aurora's Meta-Magic, for a class that prepares its spells: none, or
    // one of the metamagic feats the creature has. The spells to choose
    // from are then prepared with it, at their level raised by it (their
    // casting level), and a count is of those so prepared.
    let feats: Vec<i64> =
        f.root.list("FeatList").unwrap_or(&[]).iter().filter_map(|s| s.integer("Feat")).collect();
    let metamagics: Vec<u8> = std::iter::once(0)
        .chain(METAMAGIC.iter().filter(|m| feats.contains(&(m.1 as i64))).map(|m| m.0))
        .collect();
    let mut metamagic: u8 = ui.data(|d| d.get_temp(id.with("metamagic"))).unwrap_or(0);
    if !metamagics.contains(&metamagic) {
        metamagic = 0;
    }
    let meta_name = |m: &u8| if *m == 0 { "None".to_string() } else { metamagic_name(game, *m) };
    let prepares_spells = casters[which].3.prepares;
    ui.add_enabled_ui(prepares_spells, |ui| {
        ui.horizontal(|ui| {
            crate::widgets::field_label(ui, "Meta-Magic");
            crate::widgets::dropdown(id.with("metamagic-list"))
                .selected_text(meta_name(&metamagic))
                .width(160.0)
                .show(ui, &mut metamagic, &metamagics, meta_name)
                .on_hover_text("The metamagic feats the creature has: spells are prepared with it");
        });
    });
    if !prepares_spells {
        metamagic = 0;
    }
    ui.data_mut(|d| d.insert_temp(id.with("metamagic"), metamagic));
    let raised = mg_module::script_set::metamagic_levels(metamagic) as usize;
    let (mut clear, mut save, mut load) = (false, false, false);
    ui.horizontal_wrapped(|ui| {
        clear = ui.button("Clear Class Spell List").clicked();
        save = ui.button("Save Class Spell List").clicked();
        load = ui.button("Load Class Spell List").clicked();
    });
    let (index, class, _, slots) = casters[which].clone();
    let prepares = slots.prepares;
    let spells = class_spells(game, class);
    let innate = game.table("spells").ok();
    let names = game
        .choices("spells", ChoiceColumns { name: Some("Name"), label: Some("Label") })
        .map(mg_rules::by_name)
        .unwrap_or_default();
    let lists: Vec<Vec<Struct>> =
        (0..=9).map(|l| class_list[index].list(&slots.list(l)).unwrap_or(&[]).to_vec()).collect();
    let groups: Vec<Vec<SpellGroup>> = lists.iter().map(|l| spell_groups(l)).collect();
    let categories = spell_categories(game);
    let (needle, category) = ui
        .horizontal_wrapped(|ui| {
            let t = filter(ui, id.with("find"));
            (t, category_filter(ui, id.with("category"), &categories))
        })
        .inner;
    let in_category = |row: usize| {
        category.is_none_or(|c| innate.as_ref().and_then(|t| t.get_int(row, "Category")) == Some(c))
    };
    let path = base.item("ClassList", index);
    // What the page changes: (what, edits), one command.
    let mut change: Option<(&str, Vec<Edit>)> = None;
    // A spell's entries in list `l` (of the metamagic chosen, for a class
    // that prepares its spells).
    let had = |l: usize, spell: usize| -> Vec<usize> {
        let Some(list) = lists.get(l) else { return Vec::new() };
        let of = |s: &Struct| {
            s.integer("Spell") == Some(spell as i64)
                && (!prepares || s.integer("SpellMetaMagic").unwrap_or(0) == i64::from(metamagic))
        };
        list.iter().enumerate().filter(|(_, s)| of(s)).map(|(i, _)| i).collect()
    };
    ui.columns(2, |columns| {
        // The class's spells, to choose from: a click on one gives it to the
        // creature (prepares one more of it).
        let ui = &mut columns[0];
        egui::ScrollArea::vertical()
            .id_salt(("utc-spell-list", key, which))
            .auto_shrink(false)
            .show(ui, |ui| {
                // (The casting level, with a metamagic chosen: the spell level else.)
                let columns = if raised > 0 { 3 } else { 2 };
                egui::Grid::new(("utc-spell-grid", key, which))
                    .num_columns(columns)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong(if prepares { "Spell" } else { "Known Spell" });
                        ui.strong("Level");
                        if raised > 0 {
                            ui.strong("Casting").on_hover_text(
                                "Casting Level: the spell level raised by the metamagic",
                            );
                        }
                        ui.end_row();
                        for (row, name, l) in &spells {
                            if level.is_some_and(|x| x != *l)
                                || !needle.finds(name)
                                || !in_category(*row)
                            {
                                continue;
                            }
                            // (Prepared with metamagic: at its casting level.)
                            let cast = if prepares { l + raised } else { *l };
                            let present = had(cast, *row);
                            let r = if prepares {
                                let text = match present.len() {
                                    0 => name.clone(),
                                    n => format!("{name}  ×{n}"),
                                };
                                // (Aurora lists one cast above level 9 too, as a
                                // level no list keeps.)
                                let r = ui
                                    .add_enabled(
                                        cast <= 9,
                                        egui::Button::selectable(!present.is_empty(), text),
                                    )
                                    .on_hover_text("Prepare one more")
                                    .on_disabled_hover_text("Cast above level 9");
                                if r.clicked() {
                                    let item = spell_entry(*row, metamagic);
                                    let at = lists[cast].len();
                                    let edit =
                                        insert(key, path.clone(), &slots.list(cast), at, item);
                                    change = Some(("Prepare spell", vec![edit]));
                                }
                                r
                            } else {
                                let mut known = !present.is_empty();
                                let r = ui.checkbox(&mut known, name);
                                if r.changed() {
                                    let list = slots.list(*l);
                                    change = Some(if known {
                                        let item = spell_entry(*row, 0);
                                        let at = lists[*l].len();
                                        (
                                            "Add known spell",
                                            vec![insert(key, path.clone(), &list, at, item)],
                                        )
                                    } else {
                                        let edits = present.iter().rev();
                                        let edits =
                                            edits.map(|&i| remove(key, path.clone(), &list, i));
                                        ("Remove known spell", edits.collect())
                                    });
                                }
                                r
                            };
                            described(r, Some(game), "spells", "SpellDesc", *row);
                            ui.label(l.to_string());
                            if raised > 0 {
                                ui.label(cast.to_string());
                            }
                            ui.end_row();
                        }
                    });
            });
        // Those the class has, by level: how many it may have of each
        // (as Aurora's warnings count them), and a row for each spell (a
        // count of a prepared one), × to take it away.
        let ui = &mut columns[1];
        ui.strong(if prepares { "Prepared" } else { "Known" });
        egui::ScrollArea::vertical()
            .id_salt(("utc-spell-assigned", key, which))
            .auto_shrink(false)
            .show(ui, |ui| {
                let warn = ui.visuals().warn_fg_color;
                for l in 0..=9 {
                    let n = lists[l].len();
                    if n == 0 && l >= slots.levels {
                        continue;
                    }
                    let text = if l < slots.levels {
                        format!("Level {l}: {n} of {}", slots.max[l])
                    } else {
                        format!("Level {l}: {n}, a level the class doesn't cast yet")
                    };
                    let over = l >= slots.levels || n > slots.max[l];
                    ui.add_space(4.0);
                    if over {
                        ui.colored_label(warn, egui::RichText::new(text).strong());
                    } else {
                        ui.label(egui::RichText::new(text).strong());
                    }
                    let list = slots.list(l);
                    // By name, as the list beside it.
                    let mut rows: Vec<(String, &SpellGroup)> = groups[l]
                        .iter()
                        .map(|g| {
                            let name = mg_rules::choice_text(&names, g.spell);
                            (with_metamagic(game, &name, g.metamagic), g)
                        })
                        .collect();
                    rows.sort_by_key(|(name, _)| name.to_lowercase());
                    for (name, g) in rows {
                        ui.horizontal(|ui| {
                            let tip = if prepares { "Prepare none of it" } else { "Forget it" };
                            if ui.small_button("×").on_hover_text(tip).clicked() {
                                let edits = g.entries.iter().rev();
                                let edits = edits.map(|&i| remove(key, path.clone(), &list, i));
                                change = Some(("Remove spell", edits.collect()));
                            }
                            if prepares {
                                let count = g.entries.len() as i64;
                                if let Some(v) = spin_number(ui, count, 1..=99) {
                                    let first = &lists[l][g.entries[0]];
                                    let edits =
                                        set_count(key, &path, &list, first, &g.entries, v as usize);
                                    change = Some(("Prepared spells", edits));
                                }
                            }
                            let row = usize::try_from(g.spell).unwrap_or(usize::MAX);
                            let r = ui.label(name);
                            described(r, Some(game), "spells", "SpellDesc", row);
                        });
                    }
                }
            });
    });
    // Clear, Save and Load Class Spell List: the class's lists as a whole.
    let held: Vec<(usize, ListedSpell)> = (0..=9)
        .flat_map(|l| {
            class_list[index].list(&slots.list(l)).unwrap_or(&[]).iter().map(move |s| {
                let n = |label: &str| s.integer(label).unwrap_or(0).max(0);
                (
                    l,
                    ListedSpell {
                        spell: n("Spell").min(i64::from(u16::MAX)) as u16,
                        flags: n("SpellFlags").min(255) as u8,
                        metamagic: n("SpellMetaMagic").min(255) as u8,
                    },
                )
            })
        })
        .collect();
    let cleared = || -> Vec<Edit> {
        (0..=9)
            .flat_map(|l| {
                let list = slots.list(l);
                let n = class_list[index].list(&list).map_or(0, <[_]>::len);
                let path = path.clone();
                (0..n).rev().map(move |i| remove(key, path.clone(), &list, i))
            })
            .collect()
    };
    if clear {
        apply(f, "Clear class spell list", cleared());
        return;
    }
    if save
        && let Some(file) =
            f.app.dialogs.save_file(FileKind::SpellList, Some(std::path::Path::new("spells.ini")))
    {
        let list: Vec<ListedSpell> = held.iter().map(|(_, s)| *s).collect();
        let text = mg_module::script_set::write_spells(class as u32, &list);
        if let Err(e) = std::fs::write(&file, text) {
            f.app.log.error(format!("{}: {e}", file.display()));
        }
    }
    if load {
        let start = f.app.install.as_ref().map(|i| i.root.join("data").join("scr"));
        let Some(file) = f.app.dialogs.open_file(FileKind::SpellList, start.as_deref()) else {
            return;
        };
        let text = match std::fs::read(&file) {
            Ok(b) => crate::text::decode(&b),
            Err(e) => {
                f.app.log.error(format!("{}: {e}", file.display()));
                return;
            }
        };
        let Some(listed) = mg_module::script_set::read_spells(&text, class as u32) else {
            f.app.log.warn(format!("{}: no spell list for this class", file.display()));
            return;
        };
        // Each spell at its level for the class (a prepared one raised by
        // its metamagic), replacing the class's lists.
        let mut edits = cleared();
        let mut counts = [0usize; 10];
        for s in listed {
            let base = spells.iter().find(|(row, _, _)| *row == usize::from(s.spell)).map(|x| x.2);
            let base = base.or_else(|| {
                let t = innate.as_ref()?;
                t.get_int(usize::from(s.spell), "Innate").map(|l| l.clamp(0, 9) as usize)
            });
            let Some(base) = base else {
                f.app.log.warn(format!("{}: spell {} skipped", file.display(), s.spell));
                continue;
            };
            let extra =
                if prepares { mg_module::script_set::metamagic_levels(s.metamagic) } else { 0 };
            let l = (base + extra as usize).min(9);
            let mut item = Struct::new(SPELL_ID);
            item.set("Spell", Value::Word(s.spell));
            item.set("SpellFlags", Value::Byte(s.flags));
            item.set("SpellMetaMagic", Value::Byte(s.metamagic));
            edits.push(insert(key, path.clone(), &slots.list(l), counts[l], item));
            counts[l] += 1;
        }
        apply(f, "Load class spell list", edits);
        return;
    }
    if let Some((what, edits)) = change {
        apply(f, what, edits);
    }
}

/// Special abilities as the game has them: a spell used so many times at
/// a caster level. The file keeps an entry for each use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AbilityGroup {
    pub spell: i64,
    pub caster_level: i64,
    pub flags: i64,
    /// The entries of `SpecAbilityList` that are its uses.
    pub entries: Vec<usize>,
}

/// The list's entries grouped into abilities, in the order each first
/// appears: entries of one spell and caster level that are ready (or
/// spent) alike are uses of one ability. (The game reads the flags as
/// yes or no: entries flagged 1 and 5 are the same to it. The group has
/// its first entry's flags.)
pub(crate) fn ability_groups(list: &[Struct]) -> Vec<AbilityGroup> {
    let mut groups: Vec<AbilityGroup> = Vec::new();
    for (i, s) in list.iter().enumerate() {
        let n = |label: &str, default: i64| s.integer(label).unwrap_or(default);
        let (spell, caster_level, flags) =
            (n("Spell", 0), n("SpellCasterLevel", 1), n("SpellFlags", 1));
        match groups
            .iter_mut()
            .find(|g| (g.spell, g.caster_level, g.flags != 0) == (spell, caster_level, flags != 0))
        {
            Some(g) => g.entries.push(i),
            None => groups.push(AbilityGroup { spell, caster_level, flags, entries: vec![i] }),
        }
    }
    groups
}

/// The edits that make an ability of `entries` (its uses now) have `uses`:
/// more are copies of `first` after its last, fewer drop its last ones (0
/// removes it).
fn set_uses(
    key: ResKey,
    base: &GffPath,
    first: &Struct,
    entries: &[usize],
    uses: usize,
) -> Vec<Edit> {
    set_count(key, base, "SpecAbilityList", first, entries, uses)
}

/// [`set_uses`] for any list: the edits that make `entries` of `list`
/// (copies of one thing) `count`.
fn set_count(
    key: ResKey,
    base: &GffPath,
    list: &str,
    first: &Struct,
    entries: &[usize],
    count: usize,
) -> Vec<Edit> {
    if count >= entries.len() {
        let after = entries.last().map_or(0, |i| i + 1);
        (0..count - entries.len())
            .map(|k| insert(key, base.clone(), list, after + k, first.clone()))
            .collect()
    } else {
        // From the last, so the entries before keep their places.
        entries[count..].iter().rev().map(|&i| remove(key, base.clone(), list, i)).collect()
    }
}

/// A special ability's `SpellFlags` for a use the creature has: BioWare's
/// creature format names the byte's bits Ready (1), Spontaneous (2) and
/// Unlimited (4), but the game reads it as yes or no (any bit: a use it
/// has; none: a use spent; `engine_special_abilities.rs`), so the editor
/// has the one switch, and writes Ready.
const ABILITY_READY: i64 = 0x1;

pub(super) fn special_abilities(f: &mut Form<'_>, ui: &mut Ui) {
    let described_by = f.app.game.clone();
    let base = f.path.clone();
    let key = f.key;
    let names = mg_rules::by_name(f.choices("spells", Some("Name"), Some("Label")).to_vec());
    let name_of = |row: i64| mg_rules::choice_text(&names, row);
    let list: Vec<Struct> = f.root.list("SpecAbilityList").unwrap_or(&[]).to_vec();
    let level: i64 = f
        .root
        .list("ClassList")
        .unwrap_or(&[])
        .iter()
        .filter_map(|c| c.integer("ClassLevel"))
        .sum::<i64>()
        .max(1);
    let categories = f.app.game.as_deref().map(spell_categories).unwrap_or_default();
    let spell_table = f.app.game.as_ref().and_then(|g| g.table("spells").ok());
    let groups = ability_groups(&list);
    let mut edits: Vec<(&str, Vec<Edit>)> = Vec::new();
    crate::widgets::two_columns(ui, 300.0, |ui, col| {
        if col == 0 {
            ui.strong("Spells");
            let id = egui::Id::new(("utc-special-find", key));
            let needle = filter(ui, id);
            let category = category_filter(ui, id.with("category"), &categories);
            let in_category = |row: usize| {
                category.is_none_or(|c| {
                    spell_table.as_ref().and_then(|t| t.get_int(row, "Category")) == Some(c)
                })
            };
            egui::ScrollArea::vertical().id_salt(("utc-special-all", key)).max_height(400.0).show(
                ui,
                |ui| {
                    let offer = |row: usize| {
                        spell_table.as_ref().is_none_or(|t| offered(t, row, "Name", "Label"))
                    };
                    for c in names
                        .iter()
                        .filter(|c| needle.finds(&c.text) && in_category(c.row) && offer(c.row))
                    {
                        let r =
                            ui.selectable_label(false, &c.text).on_hover_text("Add a use of it");
                        let game = described_by.as_deref();
                        if described(r, game, "spells", "SpellDesc", c.row).clicked() {
                            let mut s = Struct::new(SPECIAL_ID);
                            s.set("Spell", Value::Word(c.row as u16));
                            s.set("SpellCasterLevel", Value::Byte(level.min(255) as u8));
                            s.set("SpellFlags", Value::Byte(1));
                            edits.push((
                                "Add special ability",
                                vec![insert(key, base.clone(), "SpecAbilityList", list.len(), s)],
                            ));
                        }
                    }
                },
            );
        } else {
            // As the game has them: so many uses of a spell at a caster
            // level (the file keeps an entry for each use).
            ui.strong("Special Abilities");
            egui::Grid::new(("utc-special", key)).num_columns(5).striped(true).show(ui, |ui| {
                ui.strong("Ability");
                ui.strong("Uses");
                ui.strong("Caster Level");
                ui.strong("Ready").on_hover_text(
                    "Whether the creature has these uses, or starts with them spent. (The \
                     file keeps a byte of flags for each use, which the game reads as yes \
                     or no: nothing makes an ability's uses unlimited.)",
                );
                ui.label("");
                ui.end_row();
                for g in &groups {
                    let first = &list[g.entries[0]];
                    let r = ui.label(name_of(g.spell));
                    let row = usize::try_from(g.spell).unwrap_or(usize::MAX);
                    described(r, described_by.as_deref(), "spells", "SpellDesc", row);
                    let uses = g.entries.len() as i64;
                    if let Some(v) = commit_number(ui, uses, 1..=99) {
                        edits.push(("Uses", set_uses(key, &base, first, &g.entries, v as usize)));
                    }
                    if let Some(v) = commit_number(ui, g.caster_level, 1..=60) {
                        let set = |&i: &usize| Edit::SetField {
                            key,
                            path: base.clone().item("SpecAbilityList", i),
                            label: "SpellCasterLevel".into(),
                            value: Some(Value::Byte(v as u8)),
                        };
                        edits.push(("Caster level", g.entries.iter().map(set).collect()));
                    }
                    // (Flags a file has besides stay as they are until
                    // the switch is turned.)
                    let mut on = g.flags != 0;
                    if ui.checkbox(&mut on, "").changed() {
                        let flags = if on { ABILITY_READY } else { 0 };
                        let set = |&i: &usize| Edit::SetField {
                            key,
                            path: base.clone().item("SpecAbilityList", i),
                            label: "SpellFlags".into(),
                            value: Some(Value::Byte(flags as u8)),
                        };
                        edits.push(("Ready", g.entries.iter().map(set).collect()));
                    }
                    if ui.small_button("Remove").clicked() {
                        edits.push((
                            "Remove special ability",
                            set_uses(key, &base, first, &g.entries, 0),
                        ));
                    }
                    ui.end_row();
                }
            });
        }
    });
    for (what, e) in edits {
        apply(f, what, e);
    }
}

/// Equipment slots: (bit, name). The last four are the creature's natural
/// equipment.
const SLOTS: [(u32, &str); 18] = [
    (0x1, "Helmet"),
    (0x2, "Armor"),
    (0x4, "Boots"),
    (0x8, "Gloves or Bracers"),
    (0x10, "Primary Weapon"),
    (0x20, "Secondary Weapon or Shield"),
    (0x40, "Cloak"),
    (0x80, "Left Ring"),
    (0x100, "Right Ring"),
    (0x200, "Amulet"),
    (0x400, "Belt"),
    (0x800, "Arrows"),
    (0x1000, "Bullets"),
    (0x2000, "Bolts"),
    (0x4000, "Creature Weapon (Left)"),
    (0x8000, "Creature Weapon (Right)"),
    (0x1_0000, "Creature Weapon (Bite)"),
    (0x2_0000, "Creature Hide"),
];

/// How the Equipment's slots are laid out in the width the page gives
/// them: a line each (the slot's name in a column, the item's icon and
/// name, Equip and Remove) where that has room, else (a window as
/// narrow as one opens beside the area) the slot's name over the item's
/// beside a smaller icon, Equip over Remove, so the item's name stays in
/// sight at any width.
struct SlotLayout {
    /// One line a slot, its names' column this wide; `None`: narrow.
    label: Option<f32>,
    /// The width of a row.
    width: f32,
    /// The width of Equip and Remove over each other.
    button: f32,
}

/// The icon beside a narrow slot's names.
const NARROW_ICON: egui::Vec2 = egui::vec2(40.0, 80.0);

impl SlotLayout {
    /// The least room for the item's name on a slot's line.
    const NAME: f32 = 120.0;

    fn new(ui: &Ui) -> SlotLayout {
        // (The side panel's margin keeps it clear of the scroll bar.)
        let width = ui.available_width();
        let measure = |text: &str, style: egui::TextStyle| {
            let font = style.resolve(ui.style());
            ui.painter().layout_no_wrap(text.into(), font, egui::Color32::WHITE).size().x
        };
        let body = egui::TextStyle::Body;
        let label = SLOTS.iter().map(|(_, s)| measure(s, body.clone())).fold(0.0, f32::max);
        let pad = 2.0 * ui.spacing().button_padding.x;
        let (equip, remove) = (measure("Equip", egui::TextStyle::Button) + pad, {
            measure("Remove", egui::TextStyle::Button) + pad
        });
        let gap = ui.spacing().item_spacing.x;
        let one_line = label + crate::images::ICON_MAX.x + Self::NAME + equip + remove + 4.0 * gap;
        SlotLayout { label: (width >= one_line).then_some(label), width, button: equip.max(remove) }
    }
}

/// What a slot's row reports: its item's name (clicked, dragged, its
/// menu), Equip clicked, Remove, and the row's place (a drop on it).
struct SlotRow {
    name: egui::Response,
    equip: bool,
    take: egui::Response,
    rect: egui::Rect,
}

/// What a slot's row shows: the slot, its item's icon, name (`shown`)
/// and tip, whether it is selected, and which buttons are on.
struct SlotLook<'a> {
    slot: &'a str,
    icon: &'a [crate::images::Picture],
    shown: &'a str,
    tip: Option<String>,
    selected: bool,
    striped: bool,
    can_equip: bool,
    can_take: bool,
}

/// A slot of the Equipment, laid out as `layout` says, across the whole
/// width.
fn slot_row(ui: &mut Ui, layout: &SlotLayout, look: SlotLook<'_>) -> SlotRow {
    let SlotLook { slot, icon, shown, tip, selected, striped, can_equip, can_take } = look;
    let gap = ui.spacing().item_spacing.y;
    let line = ui.spacing().interact_size.y;
    let (max, lines) = match layout.label {
        Some(_) => (crate::images::ICON_MAX, line),
        None => (NARROW_ICON, 2.0 * line + gap),
    };
    let height = (crate::images::icon_size(icon, max).y + 4.0).max(lines);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(layout.width, height), egui::Sense::hover());
    if striped {
        ui.painter().rect_filled(rect, 0.0, ui.visuals().faint_bg_color);
    }
    let across = egui::Layout::left_to_right(egui::Align::Center);
    let mut row = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(across));
    if let Some(w) = layout.label {
        // (A column: the icons and names after it in line.)
        let used = crate::widgets::field_label(&mut row, slot).rect.width();
        row.add_space((w - used).max(0.0));
    }
    crate::images::icon_box(&mut row, icon, max, shown);
    let back = egui::Layout::right_to_left(egui::Align::Center);
    let (name, equip, take) = row
        .with_layout(back, |ui| {
            let remove = || egui::Button::new("Remove").small();
            let equip = || egui::Button::new("Equip").small();
            let (equip, take) = if layout.label.is_some() {
                let take = ui.add_enabled(can_take, remove());
                (ui.add_enabled(can_equip, equip()), take)
            } else {
                let size = egui::vec2(layout.button, lines);
                let stacked = egui::Layout::top_down_justified(egui::Align::Center);
                ui.allocate_ui_with_layout(size, stacked, |ui| {
                    let equip = ui.add_enabled(can_equip, equip());
                    (equip, ui.add_enabled(can_take, remove()))
                })
                .inner
            };
            let equip = equip.on_hover_text("Equip the item chosen in the palette").clicked();
            let names = egui::vec2(ui.available_width(), lines);
            let down = egui::Layout::top_down(egui::Align::Min);
            let name = ui
                .allocate_ui_with_layout(names, down, |ui| {
                    if layout.label.is_none() {
                        // (A long one cut short: whole in its tip.)
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                        crate::widgets::field_label(ui, slot);
                    }
                    let width = ui.available_width();
                    let name = egui::Button::selectable(selected, (shown, egui::Atom::grow()))
                        .min_size(egui::vec2(width, 0.0))
                        .truncate()
                        .sense(egui::Sense::click_and_drag());
                    let r = ui.add(name);
                    match tip {
                        Some(tip) => r.on_hover_text(tip),
                        None => r,
                    }
                })
                .inner;
            (name, equip, take)
        })
        .inner;
    SlotRow { name, equip, take, rect }
}

/// A creature's items' flags (Aurora's Options, under its palette): each
/// field, labeled as Aurora labels it, and Aurora's tip with what the game
/// does with it.
const ITEM_FLAGS: [(&str, &str); 2] = [
    (
        "Dropable",
        "Specify if selected item can be dropped: the game leaves it on the creature's remains \
         when it is killed (GetDroppableFlag)",
    ),
    (
        "Pickpocketable",
        "Specify if selected item can be pickpocketed: a player can steal it with Pick Pocket \
         (GetPickpocketableFlag)",
    ),
];

/// The creature's items selected on its Inventory page (each its list and
/// place in it), the one clicked last (for Shift's range), and how long
/// the two lists were then: when either changes, the places mean other
/// items, and the selection is let go.
#[derive(Clone, Default)]
struct Selected {
    items: std::collections::BTreeSet<(&'static str, usize)>,
    anchor: Option<(&'static str, usize)>,
    lens: (usize, usize),
}

/// What the Options under the palette asked for.
#[derive(Default)]
struct OptionsOut {
    /// The flag set or cleared on every item selected: its edits.
    flags: Option<(&'static str, Vec<Edit>)>,
    /// The one item selected: taken from the backpack and equipped
    /// (Equip), unequipped into the backpack (To Backpack), or its
    /// properties opened.
    wear: Option<(usize, ResRef)>,
    stow: Option<(usize, ResRef)>,
    open: Option<(&'static str, usize)>,
}

/// Aurora's Options for the items selected: Selected Item, Dropable and
/// Pickpocketable (always there; grayed while nothing is selected, as in
/// Aurora; mixed where the items differ), and for one item Equip or To
/// Backpack, and Properties.
fn item_options(
    f: &Form<'_>,
    ui: &mut Ui,
    picked: &[(&'static str, usize, &Struct)],
    names: &std::collections::HashMap<ResRef, String>,
) -> OptionsOut {
    let mut out = OptionsOut::default();
    let (key, base) = (f.key, &f.path);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        crate::widgets::section_heading(ui, "Options");
        let what = match picked {
            [] => "Selected Item: none (click an item; Ctrl+click adds one, Shift+click a range)"
                .to_string(),
            [(_, _, e)] => format!("Selected Item: {}", f.entry_name(e, names)),
            many => format!("Selected Items: {}", many.len()),
        };
        ui.add(egui::Label::new(what).wrap());
        ui.horizontal_wrapped(|ui| {
            for (flag, tip) in ITEM_FLAGS {
                let (mut on, mixed) = inventory::flag_state(picked.iter().map(|p| p.2), flag);
                let boxed = egui::Checkbox::new(&mut on, flag).indeterminate(mixed);
                let r = ui.add_enabled(!picked.is_empty(), boxed).on_hover_text(tip);
                if r.changed() {
                    let edits: Vec<Edit> = picked
                        .iter()
                        .filter_map(|&(list, i, e)| {
                            inventory::flag_edit(key, base.clone().item(list, i), e, flag, on)
                        })
                        .collect();
                    out.flags = Some((flag, edits));
                }
            }
        });
        let [(list, i, e)] = picked else { return };
        let (list, i) = (*list, *i);
        ui.horizontal_wrapped(|ui| {
            let r = inventory::entry_resref(e);
            if list == "ItemList" {
                let hint =
                    "Take it from the backpack and equip it in the first free slot it goes in";
                if ui.small_button("Equip").on_hover_text(hint).clicked() {
                    out.wear = Some((i, r));
                }
            } else if ui
                .small_button("To Backpack")
                .on_hover_text("Unequip it and put it in the backpack")
                .clicked()
            {
                out.stow = Some((i, r));
            }
            if ui
                .small_button("Properties")
                .on_hover_text(
                    "The item's properties: a placed creature's item is its own; a blueprint's \
                     is its blueprint (the module's to edit, the game's to look at)",
                )
                .clicked()
            {
                out.open = Some((list, i));
            }
        });
    });
    out
}

/// The slots an item can go in (its base item's `EquipableSlots`): a
/// whole item's own base item (`item`, one moved), else its blueprint's.
fn item_slots(f: &mut Form<'_>, resref: ResRef, item: Option<&Struct>) -> u32 {
    let base = match item.and_then(|s| s.integer("BaseItem")) {
        Some(b) => Some(b),
        None => f.blueprint(BlueprintKind::Item, resref).and_then(|u| u.integer("BaseItem")),
    };
    let table = f.app.game.as_deref().and_then(|g| g.table("baseitems").ok());
    match (table, base) {
        (Some(t), Some(b)) => t.get_int(b.max(0) as usize, "EquipableSlots").unwrap_or(0) as u32,
        _ => 0,
    }
}

/// Aurora's notice on opening a creature's inventory (dialog.tlk 67646).
const INVENTORY_NOTICE: (u32, &str) = (
    67646,
    "WARNING: The game may unequip items from this creature if they do not possess the \
     appropriate feats or are not of sufficient level. This only applies to the Standard \
     Equipment.",
);

/// Aurora's question when an item needs a feat the creature lacks
/// (dialog.tlk 9069; `%s`: the feats, a line each).
const MISSING_FEATS: (u32, &str) = (
    9069,
    "This creature cannot equip the selected item because the creature is missing one or more \
     of the following feats:\n%s\n\nDo you wish to add the first feat listed above to the \
     creature so that it can equip the item?",
);

/// A talk table string, else its English text.
fn tlk(game: Option<&GameData>, (strref, english): (u32, &str)) -> String {
    game.and_then(|g| g.string(mg_core::StrRef(strref))).unwrap_or_else(|| english.to_string())
}

/// An item equipped from where it was (the backpack, another slot,
/// another object's inventory): itself, and the edits that take it from
/// there (none for a copy), made with the equipping as one step.
#[derive(Debug, Clone, PartialEq)]
struct Moving {
    item: Struct,
    taken: Vec<Edit>,
}

/// The creature's `items` of its list `of` once `taken` has run: without
/// those it takes out of them (an item moved from one slot to another, or
/// from the backpack).
fn after_taken(f: &Form<'_>, items: &[Struct], of: &str, taken: &[Edit]) -> Vec<Struct> {
    let mut rest = items.to_vec();
    for e in taken {
        if let Edit::RemoveItem { key, path, list, index } = e
            && (*key, path, list.as_str()) == (f.key, &f.path, of)
            && *index < rest.len()
        {
            rest.remove(*index);
        }
    }
    rest
}

/// The edits that equip `r` in slot `bit`: `moving` taken from where it
/// was and put there as it is, else a new item of blueprint `r`. What
/// the slot held goes to the end of the backpack (`backpack`), as it is.
fn equip_edits(
    f: &mut Form<'_>,
    (equipped, backpack): (&[Struct], &[Struct]),
    bit: u32,
    r: ResRef,
    moving: Option<&Moving>,
) -> Vec<Edit> {
    let (key, base) = (f.key, f.path.clone());
    let mut e = Vec::new();
    let taken = moving.map_or(&[][..], |m| &m.taken[..]);
    e.extend(taken.iter().cloned());
    let equipped = after_taken(f, equipped, "Equip_ItemList", taken);
    let backpack = after_taken(f, backpack, "ItemList", taken);
    let s = match moving {
        Some(m) => mg_module::instances::equipped_item(&f.carried(&m.item, bit), bit),
        // A placed creature holds the whole item; a blueprint its resref.
        None if f.is_instance()
            && let Some(whole) = f.held_item(r, bit) =>
        {
            whole
        }
        None => {
            let mut s = Struct::new(bit);
            s.set("EquippedRes", Value::resref(r));
            s
        }
    };
    // One item per slot; the list is kept in slot order.
    if let Some(i) = equipped.iter().position(|s| s.id == bit) {
        e.push(remove(key, base.clone(), "Equip_ItemList", i));
        let held = f.inventory_item(&backpack, &inventory::Added::Item(equipped[i].clone()));
        e.push(insert(key, base.clone(), "ItemList", backpack.len(), held));
    }
    let at = equipped.iter().filter(|s| s.id < bit).count();
    e.push(insert(key, base, "Equip_ItemList", at, s));
    e
}

pub(super) fn inventory(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    let key = f.key;
    if !f.app.settings.no_inventory_warning {
        let notice = tlk(f.app.game.as_deref(), INVENTORY_NOTICE);
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(ui.visuals().warn_fg_color, notice);
            if ui
                .small_button("Never warn again")
                .on_hover_text("Options › General: Show creature inventory warning")
                .clicked()
            {
                f.app.settings.no_inventory_warning = true;
            }
        });
    }
    let equipped: Vec<Struct> = f.root.list("Equip_ItemList").unwrap_or(&[]).to_vec();
    let backpack: Vec<Struct> = f.root.list("ItemList").unwrap_or(&[]).to_vec();
    let names = f.blueprint_names(BlueprintKind::Item);
    let name_of = |r: ResRef| names.get(&r).cloned().unwrap_or_else(|| r.to_string());
    let (mut add, mut chosen, mut equip) = (None, None, None);
    // Equip the palette's item where it fits; a backpack item equipped; an
    // equipped one put back; an item's Properties opened.
    let (mut equip_chosen, mut wear, mut stow, mut open) = (false, None, None, None);
    // Dropped on a slot (the slot, and what was dropped); an equipped item
    // taken off by its menu.
    let mut slot_drop: Option<(u32, inventory::Dragged)> = None;
    let (mut adds, mut taken): (Vec<inventory::Added>, Vec<Edit>) = (Vec::new(), Vec::new());
    let (mut unequip, mut copy) = (None, None);
    let whose = inventory::owner(key, &base);
    // (The palette's rows offer Equip here, and a dragged one is equipped
    // where it is dropped.)
    ui.data_mut(|d| d.insert_temp(super::picker::can_equip_id(), true));
    let mut edits: Vec<(&str, Edit)> = Vec::new();
    // The items selected (Aurora selects one; here Ctrl and Shift add
    // more, to set their options together).
    let item_id = egui::Id::new(("utc-selected-items", key, base.clone()));
    let lens = (equipped.len(), backpack.len());
    let mut selected: Selected =
        ui.data(|d| d.get_temp(item_id)).filter(|s: &Selected| s.lens == lens).unwrap_or_default();
    selected.lens = lens;
    let picked: Vec<(&'static str, usize, &Struct)> = selected
        .items
        .iter()
        .filter_map(|&(list, i)| {
            let items = if list == "ItemList" { &backpack } else { &equipped };
            items.get(i).map(|e| (list, i, e))
        })
        .collect();
    let mut options = OptionsOut::default();
    let mut pick_item = None;
    ui.horizontal_top(|ui| {
        // (Narrower in a narrow window, leaving the slots room.)
        let palette = (ui.available_width() * 0.4).clamp(200.0, 240.0);
        ui.vertical(|ui| {
            ui.set_width(palette);
            // (The item chosen stays in sight under the tree, and Aurora's
            // Options under it, for the items selected on the right.)
            add = f.palette_picker_over(ui, BlueprintKind::Item, "Add to Backpack", |f, ui| {
                f.chosen_item(ui);
                let chosen = f.palette_chosen(ui, BlueprintKind::Item);
                equip_chosen |= ui
                    .add_enabled(chosen.is_some(), egui::Button::new("Equip"))
                    .on_hover_text(
                        "Equip the item chosen above in the first free slot it goes in (the \
                         Equip beside a slot puts it in that one)",
                    )
                    .clicked();
                options = item_options(f, ui, &picked, &names);
            });
            chosen = f.palette_chosen(ui, BlueprintKind::Item);
            ui.data_mut(|d| d.remove::<bool>(super::picker::can_equip_id()));
            // Equip, from a palette row's menu: where it fits.
            let asked = super::picker::equip_asked_id();
            if let Some(r) = ui.data_mut(|d| d.remove_temp::<ResRef>(asked)) {
                chosen = Some(r);
                equip_chosen = true;
            }
        });
        ui.separator();
        super::side_panel(ui, egui::Id::new(("utc-inventory", key)), |ui| {
            {
                crate::widgets::section_heading(ui, "Equipment");
                let icons: Vec<_> = equipped.iter().map(|e| f.entry_icon(ui.ctx(), e)).collect();
                let layout = SlotLayout::new(ui);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (n, (bit, slot)) in SLOTS.into_iter().enumerate() {
                        let at = equipped.iter().position(|s| s.id == bit);
                        let shown = at.map_or_else(
                            || "—".to_string(),
                            |i| f.entry_name(&equipped[i], &names),
                        );
                        let tip = at.map(|i| {
                            format!("{shown} ({})", inventory::entry_resref(&equipped[i]))
                        });
                        let look = SlotLook {
                            slot,
                            icon: at.map_or(&[][..], |i| &icons[i][..]),
                            shown: &shown,
                            tip,
                            selected: at
                                .is_some_and(|i| selected.items.contains(&("Equip_ItemList", i))),
                            striped: n % 2 == 1,
                            can_equip: chosen.is_some(),
                            can_take: at.is_some(),
                        };
                        let SlotRow { name: r, equip: equip_here, take, rect } =
                            slot_row(ui, &layout, look);
                        if r.clicked()
                            && let Some(i) = at
                        {
                            pick_item = Some(("Equip_ItemList", i));
                        }
                        // What is equipped is dragged: to the backpack, or
                        // to another slot it goes in.
                        if let Some(i) = at {
                            let resref = inventory::entry_resref(&equipped[i]);
                            if r.drag_started() {
                                r.dnd_set_drag_payload(inventory::Dragged::Equipped {
                                    owner: whose,
                                    source: (key, base.clone()),
                                    index: i,
                                    resref,
                                    item: equipped[i].clone(),
                                });
                            }
                            r.context_menu(|ui| {
                                if ui.button("To Backpack").clicked() {
                                    stow = Some((i, resref));
                                    ui.close();
                                }
                                if ui.button("Copy").clicked() {
                                    copy = Some(resref);
                                    ui.close();
                                }
                                if ui.button("Remove").clicked() {
                                    unequip = Some(i);
                                    ui.close();
                                }
                                // Properties last, as in Aurora's menus.
                                ui.separator();
                                if ui.button("Properties").clicked() {
                                    open = Some(("Equip_ItemList", i));
                                    ui.close();
                                }
                            });
                        }
                        if equip_here {
                            equip = Some(bit);
                        }
                        if take.clicked()
                            && let Some(i) = at
                        {
                            edits.push(("Unequip", remove(key, base.clone(), "Equip_ItemList", i)));
                        }
                        // The slot's row (from its name to its buttons)
                        // takes what is dropped on it.
                        if let Some(d) = inventory::dropped_on(ui, rect) {
                            slot_drop = Some((bit, d));
                        }
                    }
                });
                ui.add_space(crate::widgets::SECTION_GAP);
                crate::widgets::section_heading(ui, "Backpack");
                let path = base.clone();
                let in_pack: Vec<usize> =
                    selected.items.iter().filter(|c| c.0 == "ItemList").map(|c| c.1).collect();
                let copied = f.app.item_clip.len();
                let ctx = ui.ctx().clone();
                let mut row =
                    |_: usize, it: &Struct| (f.entry_icon(&ctx, it), f.entry_name(it, &names));
                let look = inventory::ItemLook {
                    row: &mut row,
                    infinite: false,
                    selected: &in_pack,
                    equip: true,
                    copied,
                };
                let mut out = inventory::item_list(ui, key, &path, &backpack, look);
                edits.extend(std::mem::take(&mut out.edits));
                if let Some(i) = out.clicked {
                    pick_item = Some(("ItemList", i));
                }
                // The creature's own: an equipped item dropped on the
                // backpack comes off into it; Equip from a row's menu.
                if let Some(inventory::Dragged::Equipped { owner, index, resref, .. }) =
                    &out.dropped
                    && *owner == whose
                {
                    stow = Some((*index, *resref));
                }
                if let Some((i, inventory::Ask::Equip)) = out.asked {
                    wear = backpack.get(i).map(|e| (i, inventory::entry_resref(e), None));
                }
                // And what every inventory does: pasted, dragged from the
                // palette or from another object's inventory, copied, moved.
                (adds, taken) = f.list_events(ui, &backpack, &mut out, whose, &path);
            }
        });
    });
    // A click selects the item alone; with Ctrl, adds it or takes it away;
    // with Shift, the items from the one clicked last to it (the slots in
    // their order, then the backpack).
    if let Some(p) = pick_item {
        let listed: Vec<(&'static str, usize)> = (0..equipped.len())
            .map(|i| ("Equip_ItemList", i))
            .chain((0..backpack.len()).map(|i| ("ItemList", i)))
            .collect();
        let m = ui.input(|i| i.modifiers);
        let s = &mut selected;
        crate::widgets::select_in_list(&mut s.items, &mut s.anchor, &listed, p, m.command, m.shift);
    }
    if let Some((flag, set)) = options.flags.take() {
        apply(f, flag, set);
    }
    if let Some((i, r)) = options.wear {
        wear = Some((i, r, None));
    }
    stow = stow.or(options.stow);
    open = open.or(options.open);
    ui.data_mut(|d| d.insert_temp(item_id, selected));
    // The first free slot an item goes in: for Equip without a slot named.
    let free_slot = |f: &mut Form<'_>, r: ResRef, item: Option<&Struct>| -> Result<u32, String> {
        let fits = item_slots(f, r, item);
        if fits == 0 {
            return Err(format!("{} can't be equipped", name_of(r)));
        }
        SLOTS
            .iter()
            .map(|(bit, _)| *bit)
            .filter(|bit| fits & bit != 0)
            .find(|bit| !equipped.iter().any(|s| s.id == *bit))
            .ok_or_else(|| format!("Every slot {} goes in is taken: remove one first", name_of(r)))
    };
    if let (true, Some(r)) = (equip_chosen, chosen) {
        match free_slot(f, r, None) {
            Ok(bit) => equip = Some(bit),
            Err(why) => f.app.log.warn(why),
        }
    }
    if let Some(r) = copy {
        f.copy_item(ui, r);
    }
    if let Some(i) = unequip {
        edits.push(("Unequip", remove(key, base.clone(), "Equip_ItemList", i)));
    }
    // An item equipped from where it was, as it is (not made again from
    // its blueprint), taken from there in the same step.
    let mut moving: Option<Moving> = None;
    // Dropped on a slot: the palette's item equipped there; a backpack item
    // taken out and equipped there; an equipped one moved to it; one from
    // another object's inventory equipped here, and taken from there unless
    // Ctrl is held, which copies it.
    match slot_drop {
        Some((bit, inventory::Dragged::Palette(r))) => (equip, chosen) = (Some(bit), Some(r)),
        Some((bit, inventory::Dragged::Listed { owner, index, resref, .. })) if owner == whose => {
            wear = Some((index, resref, Some(bit)));
        }
        Some((bit, inventory::Dragged::Equipped { owner, index, resref, item, .. }))
            if owner == whose =>
        {
            if equipped.get(index).is_some_and(|s| s.id != bit) {
                let taken = vec![remove(key, base.clone(), "Equip_ItemList", index)];
                moving = Some(Moving { item, taken });
                (equip, chosen) = (Some(bit), Some(resref));
            }
        }
        Some((bit, d)) => {
            if let inventory::Dragged::Listed { resref, item, .. }
            | inventory::Dragged::Equipped { resref, item, .. } = &d
            {
                moving = Some(Moving { item: item.clone(), taken: f.taking(ui, &d) });
                (equip, chosen) = (Some(bit), Some(*resref));
            }
        }
        None => {}
    }
    if let Some((i, r, slot)) = wear
        && let Some(item) = backpack.get(i)
    {
        let slot = match slot {
            Some(bit) if item_slots(f, r, Some(item)) & bit != 0 => Ok(bit),
            Some(_) => Err(format!("{} does not go in that slot", f.entry_name(item, &names))),
            None => free_slot(f, r, Some(item)),
        };
        match slot {
            Ok(bit) => {
                let taken = vec![remove(key, base.clone(), "ItemList", i)];
                moving = Some(Moving { item: item.clone(), taken });
                (equip, chosen) = (Some(bit), Some(r));
                ui.data_mut(|d| d.remove::<Selected>(item_id));
            }
            Err(why) => f.app.log.warn(why),
        }
    }
    // Put in the backpack as it is, at its end, in the same step.
    if let Some((i, _)) = stow
        && let Some(item) = equipped.get(i)
    {
        let item = f.inventory_item(&backpack, &inventory::Added::Item(item.clone()));
        let moved = vec![
            remove(key, base.clone(), "Equip_ItemList", i),
            insert(key, base.clone(), "ItemList", backpack.len(), item),
        ];
        apply(f, "Unequip to backpack", moved);
        ui.data_mut(|d| d.remove::<Selected>(item_id));
    }
    if let Some((list, i)) = open {
        let items = if list == "ItemList" { &backpack } else { &equipped };
        if let Some(entry) = items.get(i) {
            f.item_properties(&base, list, i, entry);
        }
    }
    // An item that needs a feat the creature lacks: Aurora asks whether to
    // add the first (and does not equip it otherwise; one moved stays where
    // it was).
    let ask = egui::Id::new(("utc-equip-feat", key, base.clone()));
    if let (Some(bit), Some(r)) = (equip, chosen) {
        let whole = moving.as_ref().map(|m| &m.item).filter(|s| inventory::is_whole(s));
        if item_slots(f, r, whole) & bit == 0 {
            let name = whole.map_or_else(|| name_of(r), |s| f.entry_name(s, &names));
            f.app.log.error(format!("{name} does not go in that slot"));
        } else {
            let feats: Vec<u16> = f
                .root
                .list("FeatList")
                .unwrap_or(&[])
                .iter()
                .filter_map(|s| s.integer("Feat"))
                .map(|v| v as u16)
                .collect();
            // (A moved whole item's own properties, else its blueprint's.)
            let item = match whole {
                Some(s) => Some(s.clone()),
                None => f.blueprint(BlueprintKind::Item, r),
            };
            let missing = match (f.app.game.as_deref(), item) {
                (Some(g), Some(item)) => g.missing_feats(&item, &feats),
                _ => Vec::new(),
            };
            if missing.is_empty() {
                let e = equip_edits(f, (&equipped, &backpack), bit, r, moving.as_ref());
                apply(f, "Equip", e);
            } else {
                ui.data_mut(|d| d.insert_temp(ask, (bit, r, missing, moving)));
            }
        }
    }
    type Asked = (u32, ResRef, Vec<u16>, Option<Moving>);
    if let Some((bit, r, missing, moving)) = ui.data(|d| d.get_temp::<Asked>(ask)) {
        let game = f.app.game.as_deref();
        let names: Vec<String> = missing
            .iter()
            .map(|&feat| {
                game.and_then(|g| {
                    let strref = g.table("feat").ok()?.get_int(usize::from(feat), "FEAT")?;
                    g.string(mg_core::StrRef(strref as u32))
                })
                .unwrap_or_else(|| format!("({feat})"))
            })
            .collect();
        let text = tlk(game, MISSING_FEATS).replacen("%s", &names.join("\n"), 1);
        let (mut yes, mut no) = (false, false);
        let modal = egui::Modal::new(ask.with("modal")).show(ui.ctx(), |ui| {
            ui.set_max_width(360.0);
            crate::widgets::field_label(ui, text);
            ui.horizontal(|ui| {
                yes = ui.button("Yes").clicked();
                no = ui.button("No").clicked();
            });
        });
        if yes {
            let count = f.root.list("FeatList").map_or(0, <[_]>::len);
            let mut feat = Struct::new(FEAT_ID);
            feat.set("Feat", Value::Word(missing[0]));
            let mut e = vec![insert(key, base.clone(), "FeatList", count, feat)];
            e.extend(equip_edits(f, (&equipped, &backpack), bit, r, moving.as_ref()));
            apply(f, "Equip", e);
        }
        if yes || no || modal.should_close() {
            ui.data_mut(|d| d.remove::<Asked>(ask));
        }
    }
    adds.extend(add.map(inventory::Added::Blueprint));
    f.apply_list(&base, &backpack, adds, taken, edits);
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::ResType;

    fn ability(spell: u16, level: u8) -> Struct {
        let mut s = Struct::new(SPECIAL_ID);
        s.set("Spell", Value::Word(spell));
        s.set("SpellCasterLevel", Value::Byte(level));
        s.set("SpellFlags", Value::Byte(1));
        s
    }

    #[test]
    fn deleted_and_unnamed_rows_are_not_offered() {
        let text =
            b"2DA V2.0\n\n  Label Name Innate\n0 Acid_Fog 2 6\n1 DELETED_PRO_Cold 5118 ****\n\
            2 Bless_Weapon **** ****\n3 Padding 7 ****\n4 **** **** ****\n";
        let t = mg_2da::TwoDa::parse(&text[..], mg_core::Codepage::default()).unwrap();
        let offer: Vec<bool> = (0..5).map(|r| offered(&t, r, "Name", "Label")).collect();
        assert_eq!(offer, [true, false, false, false, false]);
        assert_eq!(row_line(&t, "spells", 0), "spells.2da row 0: Acid_Fog, innate level 6");
        assert_eq!(row_line(&t, "feat", 4), "feat.2da row 4");
    }

    #[test]
    fn special_abilities_are_uses_of_a_spell_at_a_caster_level() {
        // Three entries of one spell at one level are three uses of it; the
        // same spell at another level is another ability.
        let list = [ability(10, 5), ability(20, 5), ability(10, 5), ability(10, 9), ability(10, 5)];
        let groups = ability_groups(&list);
        let shape: Vec<(i64, i64, &[usize])> =
            groups.iter().map(|g| (g.spell, g.caster_level, g.entries.as_slice())).collect();
        assert_eq!(shape, [(10, 5, &[0, 2, 4][..]), (20, 5, &[1]), (10, 9, &[3])]);

        let key = ResKey::parse("c", ResType::UTC).unwrap();
        let base = GffPath::root();
        let entries = &groups[0].entries;
        // More uses: copies after its last entry.
        let more = set_uses(key, &base, &list[0], entries, 5);
        let inserted: Vec<usize> = more
            .iter()
            .map(|e| match e {
                Edit::InsertItem { index, item, .. } if *item == list[0] => *index,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(inserted, [5, 6]);
        // Fewer: its last entries go, the last first.
        let fewer = set_uses(key, &base, &list[0], entries, 1);
        let removed: Vec<usize> = fewer
            .iter()
            .map(|e| match e {
                Edit::RemoveItem { index, .. } => *index,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(removed, [4, 2]);
        // None: all of them; as many: nothing.
        assert_eq!(set_uses(key, &base, &list[0], entries, 0).len(), 3);
        assert!(set_uses(key, &base, &list[0], entries, 3).is_empty());

        // Uses flagged otherwise but ready all the same are one ability (the
        // game reads the flags as yes or no); spent ones are another.
        let flagged = |flags: u8| {
            let mut s = ability(10, 5);
            s.set("SpellFlags", Value::Byte(flags));
            s
        };
        let groups = ability_groups(&[flagged(1), flagged(5), flagged(0), flagged(2)]);
        let shape: Vec<(bool, &[usize])> =
            groups.iter().map(|g| (g.flags != 0, g.entries.as_slice())).collect();
        assert_eq!(shape, [(true, &[0, 1, 3][..]), (false, &[2])]);
    }

    #[test]
    fn talent_categories_are_named_for_people() {
        assert_eq!(category_name("Harmful_AOE_Discriminant"), "Harmful AOE Discriminant");
        assert_eq!(category_name("TALENT_CATEGORY_BENEFICIAL_SUMMON"), "Beneficial Summon");
        assert_eq!(category_name("TALENT_CATEGORY_HARMFUL_MELEE "), "Harmful Melee");
        assert_eq!(category_name("TALENT_DISPEL"), "Dispel");
    }
}
