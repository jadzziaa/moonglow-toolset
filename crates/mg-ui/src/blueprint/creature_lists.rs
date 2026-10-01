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
use crate::widgets::commit_number;

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
    Edit::InsertItem { key, path, list: list.into(), index, item }
}

fn remove(key: ResKey, path: GffPath, list: &str, index: usize) -> Edit {
    Edit::RemoveItem { key, path, list: list.into(), index }
}

/// A text filter kept per page.
fn filter(ui: &mut Ui, id: egui::Id) -> String {
    let mut text: String = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    ui.add(egui::TextEdit::singleline(&mut text).hint_text("Find").desired_width(200.0));
    ui.data_mut(|d| d.insert_temp(id, text.clone()));
    text.to_lowercase()
}

pub(super) fn feats(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    let key = f.key;
    let all = f
        .app
        .game
        .as_ref()
        .and_then(|g| {
            g.choices("feat", ChoiceColumns { name: Some("FEAT"), label: Some("LABEL") }).ok()
        })
        .unwrap_or_default();
    let list: Vec<i64> =
        f.root.list("FeatList").unwrap_or(&[]).iter().filter_map(|s| s.integer("Feat")).collect();
    let id = egui::Id::new(("utc-feats", key));
    let mut assigned_only: bool = ui.data(|d| d.get_temp(id.with("only"))).unwrap_or(false);
    let needle = ui
        .horizontal(|ui| {
            let t = filter(ui, id);
            ui.checkbox(&mut assigned_only, "Assigned only");
            ui.label(format!("{} feats assigned", list.len()));
            t
        })
        .inner;
    ui.data_mut(|d| d.insert_temp(id.with("only"), assigned_only));
    let mut toggle = None;
    egui::ScrollArea::vertical().id_salt(("utc-feat-list", key)).show(ui, |ui| {
        for c in &all {
            let on = list.contains(&(c.row as i64));
            if (assigned_only && !on) || !c.text.to_lowercase().contains(&needle) {
                continue;
            }
            let mut v = on;
            if ui.checkbox(&mut v, &c.text).changed() {
                toggle = Some((c.row, v));
            }
        }
    });
    match toggle {
        Some((row, true)) => {
            let mut s = Struct::new(FEAT_ID);
            s.set("Feat", Value::Word(row as u16));
            apply(f, "Add feat", vec![insert(key, base.clone(), "FeatList", list.len(), s)]);
        }
        Some((row, false)) => {
            if let Some(i) = list.iter().position(|&r| r == row as i64) {
                apply(f, "Remove feat", vec![remove(key, base.clone(), "FeatList", i)]);
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
        .unwrap_or_default();
    let Ok(spells) = game.table("spells") else { return Vec::new() };
    names
        .into_iter()
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

pub(super) fn spells(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    let key = f.key;
    let Some(game) = f.app.game.as_ref() else { return };
    let Ok(classes) = game.table("classes") else { return };
    let class_list: Vec<Struct> = f.root.list("ClassList").unwrap_or(&[]).to_vec();
    // The creature's spellcasting classes: (index in ClassList, class row,
    // name, whether it prepares its spells).
    let casters: Vec<(usize, usize, String, bool)> = class_list
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let row = c.integer("Class")?.max(0) as usize;
            (classes.get_int(row, "SpellCaster") == Some(1)).then(|| {
                let name = classes
                    .get_int(row, "Name")
                    .and_then(|s| game.string(mg_core::StrRef(s as u32)))
                    .unwrap_or_else(|| classes.get(row, "Label").unwrap_or_default().to_string());
                (i, row, name, classes.get_int(row, "MemorizesSpells") == Some(1))
            })
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
    ui.horizontal(|ui| {
        for (n, (_, _, name, _)) in casters.iter().enumerate() {
            ui.radio_value(&mut which, n, name);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Spell Level");
        ui.selectable_value(&mut level, None, "All");
        for l in 0..=9 {
            ui.selectable_value(&mut level, Some(l), l.to_string());
        }
    });
    ui.data_mut(|d| d.insert_temp(id, (which, level)));
    let (mut clear, mut save, mut load) = (false, false, false);
    ui.horizontal(|ui| {
        clear = ui.button("Clear Class Spell List").clicked();
        save = ui.button("Save Class Spell List").clicked();
        load = ui.button("Load Class Spell List").clicked();
    });
    let (index, class, _, prepares) = casters[which].clone();
    let spells = class_spells(game, class);
    let innate = game.table("spells").ok();
    let prefix = if prepares { "MemorizedList" } else { "KnownList" };
    let entries = |l: usize| -> Vec<i64> {
        class_list[index]
            .list(&format!("{prefix}{l}"))
            .unwrap_or(&[])
            .iter()
            .filter_map(|s| s.integer("Spell"))
            .collect()
    };
    let lists: Vec<Vec<i64>> = (0..=9).map(entries).collect();
    let summary: Vec<String> = (0..=9)
        .filter(|&l| !lists[l].is_empty())
        .map(|l| format!("{l}: {}", lists[l].len()))
        .collect();
    ui.label(format!(
        "{} by level: {}",
        if prepares { "Prepared" } else { "Known" },
        if summary.is_empty() { "none".to_string() } else { summary.join(", ") }
    ));
    let needle = filter(ui, id.with("find"));
    let path = base.clone().item("ClassList", index);
    let mut change: Option<(usize, usize, i64)> = None; // (spell, level, delta)
    egui::ScrollArea::vertical().id_salt(("utc-spell-list", key)).show(ui, |ui| {
        egui::Grid::new(("utc-spell-grid", key, which)).num_columns(3).striped(true).show(
            ui,
            |ui| {
                if prepares {
                    ui.strong("Prepared");
                }
                ui.strong(if prepares { "Spell" } else { "Known Spell" });
                ui.strong("Level");
                ui.end_row();
                for (row, name, l) in &spells {
                    if level.is_some_and(|x| x != *l) || !name.to_lowercase().contains(&needle) {
                        continue;
                    }
                    let count = lists[*l].iter().filter(|&&s| s == *row as i64).count() as i64;
                    if prepares {
                        if let Some(v) = commit_number(ui, count, 0..=20) {
                            change = Some((*row, *l, v - count));
                        }
                    } else {
                        let mut known = count > 0;
                        if ui.checkbox(&mut known, name).changed() {
                            change = Some((*row, *l, if known { 1 } else { -count }));
                        }
                    }
                    if prepares {
                        ui.label(name);
                    }
                    ui.label(l.to_string());
                    ui.end_row();
                }
            },
        );
    });
    // Clear, Save and Load Class Spell List: the class's lists as a whole.
    let held: Vec<(usize, ListedSpell)> = (0..=9)
        .flat_map(|l| {
            class_list[index].list(&format!("{prefix}{l}")).unwrap_or(&[]).iter().map(move |s| {
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
                let list = format!("{prefix}{l}");
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
            edits.push(insert(key, path.clone(), &format!("{prefix}{l}"), counts[l], item));
            counts[l] += 1;
        }
        apply(f, "Load class spell list", edits);
        return;
    }
    if let Some((spell, l, delta)) = change {
        let list = format!("{prefix}{l}");
        let current = &lists[l];
        let mut edits = Vec::new();
        if delta > 0 {
            for n in 0..delta as usize {
                let mut s = Struct::new(SPELL_ID);
                s.set("Spell", Value::Word(spell as u16));
                s.set("SpellFlags", Value::Byte(1));
                s.set("SpellMetaMagic", Value::Byte(0));
                edits.push(insert(key, path.clone(), &list, current.len() + n, s));
            }
        } else {
            // Remove from the end, so earlier indexes stay valid.
            let at: Vec<usize> = current
                .iter()
                .enumerate()
                .filter(|(_, s)| **s == spell as i64)
                .map(|(i, _)| i)
                .rev()
                .take((-delta) as usize)
                .collect();
            edits.extend(at.into_iter().map(|i| remove(key, path.clone(), &list, i)));
        }
        apply(f, "Spells", edits);
    }
}

pub(super) fn special_abilities(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    let key = f.key;
    let names = f
        .app
        .game
        .as_ref()
        .and_then(|g| {
            g.choices("spells", ChoiceColumns { name: Some("Name"), label: Some("Label") }).ok()
        })
        .unwrap_or_default();
    let name_of = |row: i64| {
        names
            .iter()
            .find(|c| c.row as i64 == row)
            .map_or_else(|| format!("({row})"), |c| c.text.clone())
    };
    let list: Vec<Struct> = f.root.list("SpecAbilityList").unwrap_or(&[]).to_vec();
    let level: i64 = f
        .root
        .list("ClassList")
        .unwrap_or(&[])
        .iter()
        .filter_map(|c| c.integer("ClassLevel"))
        .sum::<i64>()
        .max(1);
    let mut edits: Vec<(&str, Edit)> = Vec::new();
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        ui.strong("Spells");
        let needle = filter(ui, egui::Id::new(("utc-special-find", key)));
        egui::ScrollArea::vertical().id_salt(("utc-special-all", key)).max_height(400.0).show(
            ui,
            |ui| {
                for c in names.iter().filter(|c| c.text.to_lowercase().contains(&needle)) {
                    if ui.selectable_label(false, &c.text).on_hover_text("Add").clicked() {
                        let mut s = Struct::new(SPECIAL_ID);
                        s.set("Spell", Value::Word(c.row as u16));
                        s.set("SpellCasterLevel", Value::Byte(level.min(255) as u8));
                        s.set("SpellFlags", Value::Byte(1));
                        edits.push((
                            "Add special ability",
                            insert(key, base.clone(), "SpecAbilityList", list.len(), s),
                        ));
                    }
                }
            },
        );
        let ui = &mut cols[1];
        ui.strong("Special Abilities");
        egui::Grid::new(("utc-special", key)).num_columns(3).striped(true).show(ui, |ui| {
            ui.strong("Ability");
            ui.strong("Caster Level");
            ui.label("");
            ui.end_row();
            for (i, s) in list.iter().enumerate() {
                ui.label(name_of(s.integer("Spell").unwrap_or(0)));
                let lvl = s.integer("SpellCasterLevel").unwrap_or(1);
                if let Some(v) = commit_number(ui, lvl, 1..=60) {
                    edits.push((
                        "Caster level",
                        Edit::SetField {
                            key,
                            path: base.clone().item("SpecAbilityList", i),
                            label: "SpellCasterLevel".into(),
                            value: Some(Value::Byte(v as u8)),
                        },
                    ));
                }
                if ui.small_button("Remove").clicked() {
                    edits.push((
                        "Remove special ability",
                        remove(key, base.clone(), "SpecAbilityList", i),
                    ));
                }
                ui.end_row();
            }
        });
    });
    for (what, e) in edits {
        apply(f, what, vec![e]);
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

/// A creature's items' flags (Aurora's Selected Item options).
const ITEM_FLAGS: [&str; 2] = ["Dropable", "Pickpocketable"];

/// The slots an item can go in (its base item's `EquipableSlots`).
fn item_slots(f: &mut Form<'_>, resref: ResRef) -> u32 {
    let base = f.blueprint(BlueprintKind::Item, resref).and_then(|u| u.integer("BaseItem"));
    let table = f.app.game.as_ref().and_then(|g| g.table("baseitems").ok());
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

/// The edits that equip `r` in slot `bit`, replacing what is there.
fn equip_edits(f: &mut Form<'_>, equipped: &[Struct], bit: u32, r: ResRef) -> Vec<Edit> {
    let (key, base) = (f.key, f.path.clone());
    // A placed creature holds the whole item; a blueprint its resref.
    let s = if f.is_instance()
        && let Some(whole) = f.held_item(r, bit)
    {
        whole
    } else {
        let mut s = Struct::new(bit);
        s.set("EquippedRes", Value::resref(r));
        s
    };
    let mut e = Vec::new();
    // One item per slot; the list is kept in slot order.
    if let Some(i) = equipped.iter().position(|s| s.id == bit) {
        e.push(remove(key, base.clone(), "Equip_ItemList", i));
    }
    let at = equipped.iter().filter(|s| s.id < bit).count();
    e.push(insert(key, base, "Equip_ItemList", at, s));
    e
}

pub(super) fn inventory(f: &mut Form<'_>, ui: &mut Ui) {
    let base = f.path.clone();
    let key = f.key;
    if !f.app.settings.no_inventory_warning {
        let notice = tlk(f.app.game.as_ref(), INVENTORY_NOTICE);
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
    let mut edits: Vec<(&str, Edit)> = Vec::new();
    let item_id = egui::Id::new(("utc-selected-item", key, base.clone()));
    let chosen_item: Option<(&'static str, usize)> = ui.data(|d| d.get_temp(item_id));
    let mut pick_item = None;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(240.0);
            add = f.palette_picker(ui, BlueprintKind::Item, "Add to Backpack");
            chosen = f.palette_chosen(ui, BlueprintKind::Item);
        });
        ui.separator();
        super::side_panel(ui, egui::Id::new(("utc-inventory", key)), |ui| {
            {
                // Aurora's Options: the selected item's Dropable and Pickpocketable.
                ui.strong("Options");
                let entry = chosen_item.and_then(|(list, i)| {
                    let items = if list == "ItemList" { &backpack } else { &equipped };
                    items.get(i).map(|e| (list, i, e))
                });
                match entry {
                    Some((list, i, e)) => {
                        ui.label(format!("Selected Item: {}", f.entry_name(e, &names)));
                        ui.horizontal(|ui| {
                            for flag in ITEM_FLAGS {
                                let path = base.clone().item(list, i);
                                if let Some(edit) = inventory::flag_box(ui, key, path, e, flag) {
                                    edits.push(edit);
                                }
                            }
                        });
                    }
                    None => {
                        ui.weak("Selected Item: none (click an item)");
                    }
                }
                ui.separator();
                ui.strong("Equipment");
                let icons: Vec<_> = equipped.iter().map(|e| f.entry_icon(ui.ctx(), e)).collect();
                egui::Grid::new(("utc-equip", key)).num_columns(5).striped(true).show(ui, |ui| {
                    for (bit, slot) in SLOTS {
                        ui.label(slot);
                        let at = equipped.iter().position(|s| s.id == bit);
                        let shown = at.map_or_else(
                            || "—".to_string(),
                            |i| f.entry_name(&equipped[i], &names),
                        );
                        let icon = at.map_or(&[][..], |i| &icons[i][..]);
                        crate::images::icon_row(ui, icon, &shown);
                        let on = at.is_some() && at.map(|i| ("Equip_ItemList", i)) == chosen_item;
                        if ui.selectable_label(on, shown).clicked()
                            && let Some(i) = at
                        {
                            pick_item = Some(("Equip_ItemList", i));
                        }
                        if ui
                            .add_enabled(chosen.is_some(), egui::Button::new("Equip").small())
                            .on_hover_text("Equip the item chosen in the palette")
                            .clicked()
                        {
                            equip = Some(bit);
                        }
                        if ui
                            .add_enabled(at.is_some(), egui::Button::new("Remove").small())
                            .clicked()
                            && let Some(i) = at
                        {
                            edits.push(("Unequip", remove(key, base.clone(), "Equip_ItemList", i)));
                        }
                        ui.end_row();
                    }
                });
                ui.separator();
                ui.strong("Backpack");
                let path = base.clone();
                let icons: Vec<_> = backpack.iter().map(|it| f.entry_icon(ui.ctx(), it)).collect();
                let name = |it: &Struct| f.entry_name(it, &names);
                let selected = chosen_item.filter(|c| c.0 == "ItemList").map(|c| c.1);
                let look =
                    inventory::ItemLook { icons: &icons, name: &name, infinite: false, selected };
                let (more, clicked) = inventory::item_list(ui, key, &path, &backpack, look);
                edits.extend(more);
                if let Some(i) = clicked {
                    pick_item = Some(("ItemList", i));
                }
            }
        });
    });
    if let Some(p) = pick_item {
        ui.data_mut(|d| d.insert_temp(item_id, p));
    }
    // An item that needs a feat the creature lacks: Aurora asks whether to
    // add the first (and does not equip it otherwise).
    let ask = egui::Id::new(("utc-equip-feat", key, base.clone()));
    if let (Some(bit), Some(r)) = (equip, chosen) {
        if item_slots(f, r) & bit == 0 {
            f.app.log.error(format!("{} does not go in that slot", name_of(r)));
        } else {
            let feats: Vec<u16> = f
                .root
                .list("FeatList")
                .unwrap_or(&[])
                .iter()
                .filter_map(|s| s.integer("Feat"))
                .map(|v| v as u16)
                .collect();
            let item = f.blueprint(BlueprintKind::Item, r);
            let missing = match (f.app.game.as_ref(), item) {
                (Some(g), Some(item)) => g.missing_feats(&item, &feats),
                _ => Vec::new(),
            };
            if missing.is_empty() {
                let e = equip_edits(f, &equipped, bit, r);
                apply(f, "Equip", e);
            } else {
                ui.data_mut(|d| d.insert_temp(ask, (bit, r, missing)));
            }
        }
    }
    if let Some((bit, r, missing)) = ui.data(|d| d.get_temp::<(u32, ResRef, Vec<u16>)>(ask)) {
        let game = f.app.game.as_ref();
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
            ui.label(text);
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
            e.extend(equip_edits(f, &equipped, bit, r));
            apply(f, "Equip", e);
        }
        if yes || no || modal.should_close() {
            ui.data_mut(|d| d.remove::<(u32, ResRef, Vec<u16>)>(ask));
        }
    }
    if let Some(r) = add {
        let item = f.inventory_item(&backpack, r);
        apply(f, "Add item", vec![insert(key, base.clone(), "ItemList", backpack.len(), item)]);
    }
    for (what, e) in edits {
        apply(f, what, vec![e]);
    }
}
