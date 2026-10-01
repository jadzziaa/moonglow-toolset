//! Item Properties (`TdlgItemEdit`): General (name, costs, the base item's
//! statistics), Appearance (by the base item's model type: one model,
//! three weapon parts, or armor parts and colours), Properties (the item
//! properties its base item offers, with their parameters: Aurora's
//! `TdlgPropEdit` inline), Description, Comments.
//!
//! Every change also stores the item's recomputed `Cost` (in the same
//! undoable command), as Aurora does on OK.

use std::sync::Arc;

use egui::Ui;
use mg_core::{ResType, StrRef};
use mg_edit::{Command, Edit};
use mg_gff::{FieldType, Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_rules::items::{ItemProperty, ItemValue, PropertyType, part_number, wide_label};
use mg_rules::{Choice, GameData};

use super::Form;
use crate::widgets::commit_number;
use crate::{Action, Moonglow};

pub(super) const PAGES: [&str; 5] =
    ["General", "Appearance", "Properties", "Description", "Comments"];

pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, page: &str) {
    match page {
        "General" => general(f, ui),
        "Appearance" => with_game(f, |f, game| appearance(f, ui, game)),
        "Properties" => with_game(f, |f, game| properties(f, ui, game)),
        "Description" => description(f, ui),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

/// After a command: the `Cost` of each item it changed, recomputed, as part
/// of the same command.
pub(crate) fn refresh_costs(app: &mut Moonglow, cmd: &Command) {
    let objects = super::changed_objects(cmd, ResType::UTI, &["Cost"]);
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_ref()) else { return };
    let mut edits = Vec::new();
    for (key, path) in objects {
        let Ok(g) = ws.doc(&key) else { continue };
        let Some(item) = path.get(&g.root) else { continue };
        let cost = game.item_cost(&ItemValue::from_gff(item));
        if item.integer("Cost") != Some(i64::from(cost)) {
            edits.push(Edit::SetField {
                key,
                path,
                label: "Cost".into(),
                value: Some(super::integer(item.get("Cost"), cost.into(), FieldType::Dword)),
            });
        }
    }
    if !edits.is_empty()
        && let Err(e) = ws.amend(edits)
    {
        app.log.error(e.to_string());
    }
}

/// Runs `body` with the game data lent out of the app (the pages that use
/// it throughout change the item through the form, which needs the app).
fn with_game(f: &mut Form<'_>, body: impl FnOnce(&mut Form<'_>, &GameData)) {
    let Some(game) = f.app.game.take() else { return };
    body(f, &game);
    f.app.game = Some(game);
}

fn base_row(f: &Form<'_>) -> usize {
    f.int("BaseItem").max(0) as usize
}

fn strref(game: &GameData, v: Option<i32>) -> Option<String> {
    v.and_then(|s| game.string(StrRef(s as u32))).filter(|s| !s.is_empty())
}

/// The base item's statistics as Aurora shows them: (label, value).
fn statistics(f: &Form<'_>) -> Vec<(&'static str, String)> {
    let Some(game) = f.app.game.as_ref() else { return Vec::new() };
    let Ok(t) = game.table("baseitems") else { return Vec::new() };
    let row = base_row(f);
    let value = ItemValue::from_gff(&f.root);
    let cost = game.item_cost(&value);
    let mut out = vec![("Total Cost", cost.to_string())];
    let armor = game.is_armor(value.base_item);
    let ac = armor.then(|| game.armor_class(value.torso)).flatten();
    let armor_table = game.table("armor").ok();
    let armor_cell = |col: &str| {
        let t = armor_table.as_ref()?;
        t.get_int(ac? as usize, col)
    };
    let tenths = if armor { armor_cell("WEIGHT") } else { t.get_int(row, "TenthLBS") };
    if let Some(w) = tenths {
        out.push(("Base Weight", format!("{:.1} lbs", w as f32 / 10.0)));
    }
    if let (Some(n), Some(d)) = (t.get_int(row, "NumDice"), t.get_int(row, "DieToRoll"))
        && n > 0
    {
        out.push(("Damage", format!("{n}d{d}")));
        let threat = t.get_int(row, "CritThreat").unwrap_or(1).max(1);
        let range = if threat == 1 { "20".to_string() } else { format!("{}-20", 21 - threat) };
        let mult = t.get_int(row, "CritHitMult").unwrap_or(2);
        out.push(("Critical", format!("{range} / x{mult}")));
        let kind = match t.get_int(row, "WeaponType") {
            Some(1) => "Piercing",
            Some(2) => "Bludgeoning",
            Some(3) => "Slashing",
            Some(4) => "Piercing and Slashing",
            Some(5) => "Bludgeoning and Piercing",
            _ => "",
        };
        if !kind.is_empty() {
            out.push(("Damage Type", kind.to_string()));
        }
    }
    if let Some(ac) = ac {
        out.push(("Armor Class", ac.to_string()));
        if let Some(v) = armor_cell("ACCHECK") {
            out.push(("Armor Check Penalty", v.to_string()));
        }
        if let Some(v) = armor_cell("ARCANEFAILURE%") {
            out.push(("Arcane Spell Failure", format!("{v}%")));
        }
        if let Some(v) = armor_cell("DEXBONUS") {
            out.push(("Max Dex Bonus", if ac == 0 { "-".to_string() } else { v.to_string() }));
        }
    } else if let Some(ac) = t.get_int(row, "BaseAC").filter(|&ac| ac > 0) {
        out.push(("Armor Class", ac.to_string()));
    }
    // The level (itemvalue.2da) and Lore (skillvsitemcost.2da) its value asks
    // for: the first row whose limit it fits.
    let first_fit = |table: &str, col: &str| {
        let t = game.table(table).ok()?;
        (0..t.len())
            .find(|&r| t.get_int(r, col).is_some_and(|max| i64::from(max) >= i64::from(cost)))
    };
    if let Some(r) = first_fit("itemvalue", "MAXSINGLEITEMVALUE") {
        out.push(("Required Level", (r + 1).to_string()));
    }
    if let Some(r) = first_fit("skillvsitemcost", "DeviceCostMax") {
        out.push(("Required Lore", r.to_string()));
    }
    out
}

fn general(f: &mut Form<'_>, ui: &mut Ui) {
    let (base_name, stacking) = f
        .app
        .game
        .as_ref()
        .and_then(|g| {
            let t = g.table("baseitems").ok()?;
            let row = base_row(f);
            let name = strref(g, t.get_int(row, "Name"))
                .or_else(|| t.get(row, "label").map(str::to_string))?;
            Some((name, t.get_int(row, "Stacking").unwrap_or(1).max(1)))
        })
        .unwrap_or_else(|| (format!("({})", f.int("BaseItem")), 1));
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        egui::Grid::new(("uti-general", f.key)).num_columns(2).spacing([12.0, 6.0]).show(
            ui,
            |ui| {
                ui.label("Item Name");
                f.locstring(ui, "Name", "LocalizedName");
                ui.end_row();
                ui.label("Tag");
                f.text(ui, "Tag", "Tag", 32);
                ui.end_row();
                ui.label("Blueprint ResRef");
                f.blueprint_resref(ui);
                ui.end_row();
                ui.label("Base Type Name");
                ui.label(base_name);
                ui.end_row();
                ui.label("Category");
                f.category(ui, BlueprintKind::Item);
                ui.end_row();
                ui.label("Stack Size");
                f.number(ui, "Stack size", "StackSize", 1..=i64::from(stacking));
                ui.end_row();
                ui.label("Charges");
                f.number(ui, "Charges", "Charges", 0..=250);
                ui.end_row();
                ui.label("Additional Cost");
                f.number(ui, "Additional cost", "AddCost", 0..=999_999_999);
                ui.end_row();
                ui.label("");
                ui.horizontal(|ui| {
                    f.check(ui, "Plot Item", "Plot");
                    f.check(ui, "Stolen", "Stolen");
                });
                ui.end_row();
                ui.label("");
                super::situated::preview_button(f, ui);
                ui.end_row();
            },
        );
        let ui = &mut cols[1];
        egui::Grid::new(("uti-stats", f.key)).num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            for (label, value) in statistics(f) {
                ui.label(label);
                ui.strong(value);
                ui.end_row();
            }
        });
    });
}

/// The numbers `nnn` for which a model `{prefix}{nnn:03}` exists, cached.
fn model_numbers(ui: &Ui, game: &GameData, prefix: &str) -> Arc<Vec<u16>> {
    let id = egui::Id::new(("model-numbers", prefix));
    if let Some(v) = ui.data(|d| d.get_temp::<Arc<Vec<u16>>>(id)) {
        return v;
    }
    let mut v: Vec<u16> = game
        .resman
        .list(ResType::MDL)
        .iter()
        .filter_map(|r| {
            let name = r.to_string();
            let rest = name.strip_prefix(prefix)?;
            (rest.len() == 3).then(|| rest.parse().ok()).flatten()
        })
        .collect();
    v.sort_unstable();
    v.dedup();
    let v = Arc::new(v);
    ui.data_mut(|d| d.insert_temp(id, v.clone()));
    v
}

/// The numbers `nnn` for which an inventory icon `i{prefix}{nnn:03}` exists
/// (TGA, DDS or PLT), cached: what Aurora offers simple and layered items
/// ("the icon determines availability", nwn.wiki's baseitems.2da).
fn icon_numbers(ui: &Ui, game: &GameData, prefix: &str) -> Arc<Vec<u16>> {
    let id = egui::Id::new(("icon-numbers", prefix));
    if let Some(v) = ui.data(|d| d.get_temp::<Arc<Vec<u16>>>(id)) {
        return v;
    }
    let icon = format!("i{prefix}");
    let mut v: Vec<u16> = [ResType::TGA, ResType::DDS, ResType::PLT]
        .into_iter()
        .flat_map(|t| game.resman.list(t))
        .filter_map(|r| {
            let name = r.to_string();
            let rest = name.strip_prefix(&icon)?;
            (rest.len() == 3).then(|| rest.parse().ok()).flatten()
        })
        .collect();
    v.sort_unstable();
    v.dedup();
    let v = Arc::new(v);
    ui.data_mut(|d| d.insert_temp(id, v.clone()));
    v
}

/// Sets a model or body part number: the BYTE field and its EE twin.
fn set_part(f: &mut Form<'_>, what: &str, labels: &[&str], v: i64) {
    let mut fields = Vec::new();
    for label in labels {
        fields.push((label.to_string(), v.min(255), FieldType::Byte));
        fields.push((wide_label(label), v, FieldType::Word));
    }
    let fields: Vec<(&str, i64, FieldType)> =
        fields.iter().map(|(l, v, t)| (l.as_str(), *v, *t)).collect();
    f.set_many(what, &fields);
}

/// A part number chosen among `numbers` (shown as `text(n)`).
fn part_choice(
    f: &mut Form<'_>,
    ui: &mut Ui,
    what: &str,
    labels: &[&str],
    numbers: &[u16],
    text: impl Fn(u16) -> String,
) {
    let current = part_number(&f.root, labels[0]).unwrap_or(0);
    let choices: Vec<Choice> =
        numbers.iter().map(|&n| Choice { row: n as usize, text: text(n) }).collect();
    if let Some(v) = super::situated::pick(ui, f.key, what, &choices, current) {
        set_part(f, what, labels, v);
    }
}

/// The armor's parts: (label, fields, parts table).
const ARMOR_PARTS: [(&str, &[&str], &str); 13] = [
    ("Neck", &["ArmorPart_Neck"], "parts_neck"),
    ("Torso", &["ArmorPart_Torso"], "parts_chest"),
    ("Belt", &["ArmorPart_Belt"], "parts_belt"),
    ("Pelvis", &["ArmorPart_Pelvis"], "parts_pelvis"),
    ("Right Shoulder", &["ArmorPart_RShoul"], "parts_shoulder"),
    ("Left Shoulder", &["ArmorPart_LShoul"], "parts_shoulder"),
    ("Right Bicep", &["ArmorPart_RBicep"], "parts_bicep"),
    ("Left Bicep", &["ArmorPart_LBicep"], "parts_bicep"),
    ("Right Forearm", &["ArmorPart_RFArm"], "parts_forearm"),
    ("Left Forearm", &["ArmorPart_LFArm"], "parts_forearm"),
    ("Right Hand", &["ArmorPart_RHand"], "parts_hand"),
    ("Left Hand", &["ArmorPart_LHand"], "parts_hand"),
    ("Thighs", &["ArmorPart_LThigh", "ArmorPart_RThigh"], "parts_legs"),
];

/// Parts both legs share below the knee, and the robe.
const LOWER_PARTS: [(&str, &[&str], &str); 3] = [
    ("Shins", &["ArmorPart_LShin", "ArmorPart_RShin"], "parts_shin"),
    ("Feet", &["ArmorPart_LFoot", "ArmorPart_RFoot"], "parts_foot"),
    ("Robe", &["ArmorPart_Robe"], "parts_robe"),
];

/// The colours of layered items and armor, and their palettes.
const COLORS: [(&str, &str, &str); 6] = [
    ("Cloth 1", "Cloth1Color", "pal_cloth01"),
    ("Cloth 2", "Cloth2Color", "pal_cloth01"),
    ("Leather 1", "Leather1Color", "pal_leath01"),
    ("Leather 2", "Leather2Color", "pal_leath01"),
    ("Metal 1", "Metal1Color", "pal_armor01"),
    ("Metal 2", "Metal2Color", "pal_armor02"),
];

fn colors(f: &mut Form<'_>, ui: &mut Ui, game: &GameData) {
    egui::Grid::new(("uti-colors", f.key)).num_columns(4).spacing([12.0, 6.0]).show(ui, |ui| {
        for (i, (text, label, palette)) in COLORS.iter().enumerate() {
            ui.label(*text);
            f.palette_color(ui, Some(game), text, label, palette);
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
}

/// The item with another model number (and its EE twin).
fn with_model(item: &Struct, label: &str, n: u16) -> Struct {
    let mut s = item.clone();
    s.set(label, Value::Byte(n.min(255) as u8));
    if s.get(&wide_label(label)).is_some() {
        s.set(&wide_label(label), Value::Word(n));
    }
    s
}

/// An item's icon layers (the game lent to the page).
fn icon(f: &mut Form<'_>, ui: &Ui, game: &GameData, item: &Struct) -> Vec<crate::images::Picture> {
    let mut loader = crate::images::Loader {
        pictures: &mut f.app.pictures,
        palettes: &mut f.app.palettes,
        ws: f.app.ws.as_ref(),
        game,
    };
    loader.item_icon(ui.ctx(), item)
}

/// Aurora's icon grid: the icon of each model number, a click chooses it.
fn icon_grid(f: &mut Form<'_>, ui: &mut Ui, game: &GameData, numbers: &[u16]) {
    let current = part_number(&f.root, "ModelPart1").unwrap_or(0);
    let mut chosen = None;
    egui::ScrollArea::vertical().max_height(220.0).id_salt(("uti-icons", f.key)).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for &n in numbers {
                let item = with_model(&f.root, "ModelPart1", n);
                let layers = icon(f, ui, game, &item);
                let r = crate::images::stacked(ui, &layers, 1.0, &format!("Appearance {n}"))
                    .on_hover_text(format!("Appearance {n}"));
                if i64::from(n) == current {
                    let stroke = ui.visuals().selection.stroke;
                    ui.painter().rect_stroke(
                        r.rect.expand(1.0),
                        2.0,
                        stroke,
                        egui::StrokeKind::Outside,
                    );
                }
                if r.clicked() {
                    chosen = Some(n);
                }
            }
        });
    });
    if let Some(n) = chosen.filter(|&n| i64::from(n) != current) {
        set_part(f, "Appearance", &["ModelPart1"], i64::from(n));
    }
}

fn appearance(f: &mut Form<'_>, ui: &mut Ui, game: &GameData) {
    let Ok(t) = game.table("baseitems") else { return };
    let row = base_row(f);
    let class = t.get(row, "ItemClass").unwrap_or_default().to_ascii_lowercase();
    // The item's inventory icon, as it is now.
    let root = f.root.clone();
    let layers = icon(f, ui, game, &root);
    crate::images::stacked(ui, &layers, 1.5, "Icon");
    match t.get_int(row, "ModelType").unwrap_or(0) {
        // Composite: bottom, middle, top, each a shape and a colour (the
        // number's tens and units).
        2 => {
            egui::Grid::new(("uti-parts", f.key)).num_columns(3).spacing([12.0, 6.0]).show(
                ui,
                |ui| {
                    for (text, label, part) in [
                        ("Top", "ModelPart3", "t"),
                        ("Middle", "ModelPart2", "m"),
                        ("Bottom", "ModelPart1", "b"),
                    ] {
                        let numbers = model_numbers(ui, game, &format!("{class}_{part}_"));
                        let current = part_number(&f.root, label).unwrap_or(0);
                        let (shape, color) = (current / 10, current % 10);
                        let mut shapes: Vec<u16> = numbers.iter().map(|n| n / 10).collect();
                        shapes.dedup();
                        let shape_numbers: Vec<u16> = shapes.iter().map(|s| s * 10).collect();
                        ui.label(text);
                        let shape_choices: Vec<Choice> = shape_numbers
                            .iter()
                            .map(|&n| Choice {
                                row: (n / 10) as usize,
                                text: format!("Model {}", n / 10),
                            })
                            .collect();
                        if let Some(s) = super::situated::pick(
                            ui,
                            f.key,
                            &format!("{label}-shape"),
                            &shape_choices,
                            shape,
                        ) {
                            // Keep the colour where the new shape has it.
                            let v = s * 10 + color;
                            let v = if numbers.contains(&(v as u16)) {
                                v
                            } else {
                                numbers
                                    .iter()
                                    .find(|&&n| i64::from(n / 10) == s)
                                    .map_or(v, |&n| n.into())
                            };
                            set_part(f, text, &[label], v);
                        }
                        let color_choices: Vec<Choice> = numbers
                            .iter()
                            .filter(|&&n| i64::from(n / 10) == shape)
                            .map(|&n| Choice {
                                row: (n % 10) as usize,
                                text: format!("Color {}", n % 10),
                            })
                            .collect();
                        if let Some(c) = super::situated::pick(
                            ui,
                            f.key,
                            &format!("{label}-color"),
                            &color_choices,
                            color,
                        ) {
                            set_part(f, text, &[label], shape * 10 + c);
                        }
                        ui.end_row();
                    }
                },
            );
        }
        // Armor: a part per body part (parts_*.2da), the robe and colours.
        3 => {
            ui.columns(2, |cols| {
                for (col, parts) in [(0, &ARMOR_PARTS[..]), (1, &LOWER_PARTS[..])] {
                    let ui = &mut cols[col];
                    egui::Grid::new(("uti-armor", f.key, col))
                        .num_columns(2)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            for (text, labels, table) in parts {
                                let numbers: Vec<u16> = game
                                    .table(table)
                                    .map(|t| {
                                        (0..t.len())
                                            .filter(|&r| t.get(r, "ACBONUS").is_some())
                                            .map(|r| r as u16)
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                let ac_of = |n: u16| {
                                    game.table(table)
                                        .ok()
                                        .and_then(|t| t.get_float(n as usize, "ACBONUS"))
                                        .unwrap_or(0.0)
                                };
                                ui.label(*text);
                                if *table == "parts_chest" {
                                    part_choice(f, ui, text, labels, &numbers, |n| {
                                        format!("{n} (AC {})", ac_of(n).round())
                                    });
                                } else {
                                    part_choice(f, ui, text, labels, &numbers, |n| n.to_string());
                                }
                                ui.end_row();
                            }
                        });
                }
                colors(f, &mut cols[1], game);
            });
        }
        // Simple and layered: one model (layered ones take colours).
        kind => {
            let prefix = if class == "cloak" { String::new() } else { format!("{class}_") };
            ui.horizontal(|ui| {
                ui.label("Appearance");
                if prefix.is_empty() {
                    // Cloaks: cloakmodel.2da rows.
                    let rows: Vec<u16> = game
                        .table("cloakmodel")
                        .map(|t| {
                            (0..t.len())
                                .filter(|&r| t.get(r, "LABEL").is_some())
                                .map(|r| r as u16)
                                .collect()
                        })
                        .unwrap_or_default();
                    part_choice(f, ui, "Appearance", &["ModelPart1"], &rows, |n| n.to_string());
                } else {
                    let numbers = icon_numbers(ui, game, &prefix);
                    part_choice(f, ui, "Appearance", &["ModelPart1"], &numbers, |n| n.to_string());
                }
            });
            let numbers: Vec<u16> = if prefix.is_empty() {
                game.table("cloakmodel")
                    .map(|t| {
                        (0..t.len())
                            .filter(|&r| t.get(r, "LABEL").is_some())
                            .map(|r| r as u16)
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                icon_numbers(ui, game, &prefix).to_vec()
            };
            icon_grid(f, ui, game, &numbers);
            if kind == 1 {
                ui.separator();
                colors(f, ui, game);
            }
        }
    }
    ui.separator();
    super::situated::preview_button(f, ui);
}

fn properties(f: &mut Form<'_>, ui: &mut Ui, game: &GameData) {
    let here = f.path.clone();
    let key = f.key;
    let base = f.int("BaseItem").max(0) as u32;
    let assigned: Vec<ItemProperty> =
        f.root.list("PropertiesList").unwrap_or(&[]).iter().map(ItemProperty::from_gff).collect();
    let state_id = egui::Id::new(("uti-props", key));
    // (available type chosen, its subtype chosen, assigned index chosen)
    let (mut chosen, mut chosen_sub, mut selected): (Option<u16>, Option<u16>, Option<usize>) =
        ui.data(|d| d.get_temp(state_id)).unwrap_or_default();
    selected = selected.filter(|&i| i < assigned.len());
    let available = game.available_properties(base);
    let mut add: Option<(PropertyType, Option<u16>)> = None;
    let mut remove = None;
    ui.columns(2, |cols| {
        let ui = &mut cols[0];
        ui.strong("Available Properties");
        egui::ScrollArea::vertical().id_salt(("uti-available", key)).max_height(380.0).show(
            ui,
            |ui| {
                for t in &available {
                    let subtypes = game.property_subtypes(t, base);
                    if subtypes.is_empty() {
                        let r = ui.selectable_label(
                            chosen == Some(t.row) && chosen_sub.is_none(),
                            &t.name,
                        );
                        if r.clicked() {
                            (chosen, chosen_sub) = (Some(t.row), None);
                        }
                        if r.double_clicked() {
                            add = Some((t.clone(), None));
                        }
                    } else {
                        egui::CollapsingHeader::new(&t.name)
                            .id_salt(("uti-type", key, t.row))
                            .show(ui, |ui| {
                                for s in &subtypes {
                                    let sub = s.row as u16;
                                    let on = chosen == Some(t.row) && chosen_sub == Some(sub);
                                    let r = ui.selectable_label(on, &s.text);
                                    if r.clicked() {
                                        (chosen, chosen_sub) = (Some(t.row), Some(sub));
                                    }
                                    if r.double_clicked() {
                                        add = Some((t.clone(), Some(sub)));
                                    }
                                }
                            });
                    }
                }
            },
        );
        let ui = &mut cols[1];
        ui.strong("Assigned Properties");
        egui::ScrollArea::vertical().id_salt(("uti-assigned", key)).max_height(200.0).show(
            ui,
            |ui| {
                for (i, p) in assigned.iter().enumerate() {
                    if ui.selectable_label(selected == Some(i), game.property_text(p)).clicked() {
                        selected = Some(i);
                    }
                }
            },
        );
        ui.horizontal(|ui| {
            let can_add = chosen.is_some();
            if ui.add_enabled(can_add, egui::Button::new("Add")).clicked()
                && let Some(t) = chosen.and_then(|c| available.iter().find(|t| t.row == c))
            {
                add = Some((t.clone(), chosen_sub));
            }
            if ui.add_enabled(selected.is_some(), egui::Button::new("Remove")).clicked() {
                remove = selected;
            }
        });
        if let Some(i) = selected {
            ui.separator();
            property_editor(f, ui, game, base, i, &assigned[i]);
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        f.check(ui, "Identified", "Identified");
        f.check(ui, "Undroppable", "Cursed");
        let cost = game.item_cost(&ItemValue::from_gff(&f.root));
        ui.label(format!("Total Cost: {cost}"));
    });
    if let Some((t, sub)) = add {
        let mut p = game.new_property(&t, base);
        if let Some(sub) = sub {
            p.subtype = sub;
            p.param1 = game.property_param_table(&t, sub).unwrap_or(255);
            p.param1_value = match p.param1 {
                255 => 0,
                table => game.property_params(table).first().map_or(0, |c| c.row as u8),
            };
        }
        f.app.actions.push(Action::Apply(Command::new(
            "Add property",
            vec![Edit::InsertItem {
                key,
                path: here.clone(),
                list: "PropertiesList".into(),
                index: assigned.len(),
                item: property_struct(&p),
            }],
        )));
        selected = Some(assigned.len());
    }
    if let Some(i) = remove {
        f.app.actions.push(Action::Apply(Command::new(
            "Remove property",
            vec![Edit::RemoveItem {
                key,
                path: here.clone(),
                list: "PropertiesList".into(),
                index: i,
            }],
        )));
        selected = None;
    }
    ui.data_mut(|d| d.insert_temp(state_id, (chosen, chosen_sub, selected)));
}

/// A `PropertiesList` entry.
fn property_struct(p: &ItemProperty) -> Struct {
    let mut s = Struct::new(0);
    s.set("PropertyName", Value::Word(p.property));
    s.set("Subtype", Value::Word(p.subtype));
    s.set("CostTable", Value::Byte(p.cost_table));
    s.set("CostValue", Value::Word(p.cost_value));
    s.set("Param1", Value::Byte(p.param1));
    s.set("Param1Value", Value::Byte(p.param1_value));
    s.set("ChanceAppear", Value::Byte(p.chance));
    s
}

/// An assigned property's parameters (Aurora's Select Property
/// Parameters): subtype, value, parameter, chance of appearing.
fn property_editor(
    f: &mut Form<'_>,
    ui: &mut Ui,
    game: &GameData,
    base: u32,
    index: usize,
    p: &ItemProperty,
) {
    let here = f.path.clone();
    let Some(t) = game.property_type(p.property) else { return };
    let path = here.clone().item("PropertiesList", index);
    let mut changes: Vec<(&str, Value)> = Vec::new();
    egui::Grid::new(("uti-prop", f.key, index)).num_columns(2).spacing([12.0, 6.0]).show(
        ui,
        |ui| {
            ui.label("Item Property");
            ui.strong(&t.name);
            ui.end_row();
            let subtypes = game.property_subtypes(&t, base);
            if !subtypes.is_empty() {
                ui.label("Sub-Property");
                if let Some(v) =
                    super::situated::pick(ui, f.key, "prop-sub", &subtypes, i64::from(p.subtype))
                {
                    changes.push(("Subtype", Value::Word(v as u16)));
                    // The parameter follows the subtype's table.
                    let table = game.property_param_table(&t, v as u16);
                    if table.unwrap_or(255) != p.param1 {
                        let first = table
                            .and_then(|tb| game.property_params(tb).first().map(|c| c.row as u8));
                        changes.push(("Param1", Value::Byte(table.unwrap_or(255))));
                        changes.push(("Param1Value", Value::Byte(first.unwrap_or(0))));
                    }
                }
                ui.end_row();
            }
            let costs = game.property_costs(p.cost_table, base);
            if !costs.is_empty() {
                ui.label("Cost Parameter");
                if let Some(v) =
                    super::situated::pick(ui, f.key, "prop-cost", &costs, i64::from(p.cost_value))
                {
                    changes.push(("CostValue", Value::Word(v as u16)));
                }
                ui.end_row();
            }
            if p.param1 != 255 {
                let params = game.property_params(p.param1);
                ui.label("Parameter 1");
                if let Some(v) = super::situated::pick(
                    ui,
                    f.key,
                    "prop-param",
                    &params,
                    i64::from(p.param1_value),
                ) {
                    changes.push(("Param1Value", Value::Byte(v as u8)));
                }
                ui.end_row();
            }
            ui.label("Chance of Appearing (%)");
            if let Some(v) = commit_number(ui, i64::from(p.chance), 0..=100) {
                changes.push(("ChanceAppear", Value::Byte(v as u8)));
            }
            ui.end_row();
        },
    );
    if !changes.is_empty() {
        let edits = changes
            .into_iter()
            .map(|(label, value)| Edit::SetField {
                key: f.key,
                path: path.clone(),
                label: label.into(),
                value: Some(value),
            })
            .collect();
        f.app.actions.push(Action::Apply(Command::new("Property parameters", edits)));
    }
}

fn description(f: &mut Form<'_>, ui: &mut Ui) {
    f.locstring_memo(ui, "Unidentified Description", "Description");
    ui.separator();
    f.locstring_memo(ui, "Identified Description", "DescIdentified");
    ui.separator();
    if let Some(game) = f.app.game.as_ref() {
        let row = base_row(f);
        let text = game
            .table("baseitems")
            .ok()
            .and_then(|t| strref(game, t.get_int(row, "Description")))
            .unwrap_or_default();
        ui.label("Item Type Description");
        ui.weak(text);
        ui.separator();
        ui.label("Item Statistics");
        for s in f.root.list("PropertiesList").unwrap_or(&[]) {
            ui.weak(game.property_text(&ItemProperty::from_gff(s)));
        }
    }
    ui.separator();
    f.variables(ui);
    f.update_instances(ui);
}
