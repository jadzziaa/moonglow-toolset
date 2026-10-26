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
use crate::widgets::spin_number;
use crate::{Action, Moonglow};

pub(super) const PAGES: [&str; 6] =
    ["General", "Appearance", "Properties", "Visuals", "Description", "Comments"];

/// The width of the Appearance page's fields beside the model.
const APPEARANCE_SIDE: f32 = 460.0;

/// The width of a part's choice: a number, "Model 12", "7 (AC 5)".
const PART_WIDTH: f32 = 110.0;

/// The least height of the model shown beside the icon, in a window too
/// narrow to have it beside the page's fields.
const MODEL_SHORT: f32 = 220.0;

pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, page: &str) {
    match page {
        "General" => general(f, ui),
        "Appearance" => with_game(f, |f, game| {
            let fields =
                |f: &mut Form<'_>, ui: &mut Ui, beside: bool| appearance(f, ui, game, beside);
            super::situated::beside_model(f, ui, APPEARANCE_SIDE, false, fields);
        }),
        "Properties" => with_game(f, |f, game| properties(f, ui, game)),
        "Visuals" => super::visuals::page(f, ui, true),
        "Description" => description(f, ui),
        _ => f.memo(ui, "Comments", "Comment"),
    }
}

/// After a command: the `Cost` of each item it changed, recomputed, as part
/// of the same command.
pub(crate) fn refresh_costs(
    app: &mut Moonglow,
    objects: Vec<(mg_resman::ResKey, mg_edit::GffPath)>,
) {
    let (Some(ws), Some(game)) = (app.ws.as_mut(), app.game.as_deref()) else { return };
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

/// Runs `body` with the game data beside the form (the pages that use it
/// throughout change the item through the form, which needs the app). The
/// app keeps its own hold on it: the model shown in the page reads it.
fn with_game(f: &mut Form<'_>, body: impl FnOnce(&mut Form<'_>, &GameData)) {
    let Some(game) = f.app.game.clone() else { return };
    body(f, &game);
}

fn base_row(f: &Form<'_>) -> usize {
    f.int("BaseItem").max(0) as usize
}

/// Whether an item of base item `row` looks different on a woman: armor,
/// cloaks and the items the game has an icon of for each.
fn worn_by_either(game: &GameData, row: usize) -> bool {
    let Ok(t) = game.table("baseitems") else { return false };
    t.get_int(row, "ModelType") == Some(3)
        || t.get(row, "ItemClass").is_some_and(|c| c.eq_ignore_ascii_case("cloak"))
        || t.get_int(row, "GenderSpecific") == Some(1)
}

/// Whether the item is shown on a woman (Options › General, or the
/// Appearance page's Shown on): where it looks different on one.
fn on_woman(f: &Form<'_>) -> bool {
    f.app.settings.armor_on_woman
        && f.app.game.as_deref().is_some_and(|g| worn_by_either(g, base_row(f)))
}

fn strref(game: &GameData, v: Option<i32>) -> Option<String> {
    v.and_then(|s| game.string(StrRef(s as u32))).filter(|s| !s.is_empty())
}

/// The base item's statistics as Aurora shows them: (label, value).
pub(super) fn statistics(game: &GameData, item: &Struct) -> Vec<(&'static str, String)> {
    let Ok(t) = game.table("baseitems") else { return Vec::new() };
    let row = item.integer("BaseItem").unwrap_or(0).max(0) as usize;
    let value = ItemValue::from_gff(item);
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
    crate::widgets::two_columns(ui, 340.0, |ui, col| {
        if col == 0 {
            egui::Grid::new(("uti-general", f.key)).num_columns(2).spacing([12.0, 6.0]).show(
                ui,
                |ui| {
                    crate::widgets::field_label(ui, "Item Name");
                    f.locstring(ui, "Name", "LocalizedName");
                    ui.end_row();
                    crate::widgets::field_label(ui, "Tag");
                    f.text(ui, "Tag", "Tag", 32);
                    ui.end_row();
                    crate::widgets::field_label(ui, "Blueprint ResRef");
                    f.blueprint_resref(ui);
                    ui.end_row();
                    crate::widgets::field_label(ui, "Base Type Name");
                    ui.label(&base_name);
                    ui.end_row();
                    f.model_warning(ui, mg_rules::ObjectKind::Item);
                    crate::widgets::field_label(ui, "Category");
                    f.category(ui, BlueprintKind::Item);
                    ui.end_row();
                    // Past the base item's stack size, as Aurora doesn't
                    // allow: the game keeps the stack whole
                    // (engine_ee_fields.rs).
                    crate::widgets::field_label(ui, "Stack Size");
                    ui.horizontal(|ui| {
                        f.spin(ui, "Stack size", "StackSize", 1..=65_535);
                        if f.int("StackSize") > i64::from(stacking) {
                            ui.weak(format!("more than the base item's {stacking}"));
                        }
                    });
                    ui.end_row();
                    // The game reads at most 250 (engine_ee_fields.rs).
                    crate::widgets::field_label(ui, "Charges");
                    f.spin(ui, "Charges", "Charges", 0..=250);
                    ui.end_row();
                    crate::widgets::field_label(ui, "Additional Cost");
                    f.spin(ui, "Additional cost", "AddCost", 0..=999_999_999);
                    ui.end_row();
                    crate::widgets::field_label(ui, "");
                    ui.horizontal(|ui| {
                        f.check(ui, "Plot Item", "Plot");
                        f.check(ui, "Stolen", "Stolen");
                    });
                    ui.end_row();
                    crate::widgets::field_label(ui, "");
                    super::situated::preview_button(f, ui);
                    ui.end_row();
                },
            );
        } else {
            egui::Grid::new(("uti-stats", f.key)).num_columns(2).spacing([12.0, 4.0]).show(
                ui,
                |ui| {
                    for (label, value) in
                        f.app.game.as_deref().map(|g| statistics(g, &f.root)).unwrap_or_default()
                    {
                        crate::widgets::field_label(ui, label);
                        ui.strong(value);
                        ui.end_row();
                    }
                },
            );
        }
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

/// The numbers a composite item's part can have: those it has a model
/// for, and those it has an icon for (a potion has one model whatever its
/// parts, and its icons alone tell them apart: with models only, its
/// lists were empty).
fn part_numbers(ui: &Ui, game: &GameData, prefix: &str) -> Arc<Vec<u16>> {
    let models = model_numbers(ui, game, prefix);
    let icons = icon_numbers(ui, game, prefix);
    if icons.is_empty() {
        return models;
    }
    let mut all: Vec<u16> = models.iter().chain(icons.iter()).copied().collect();
    all.sort_unstable();
    all.dedup();
    Arc::new(all)
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
    let choices: Vec<Choice> =
        numbers.iter().map(|&n| Choice { row: n as usize, text: text(n) }).collect();
    pick_part(f, ui, what, labels, &choices);
}

/// A part number chosen among `choices`, set in `labels` (and their EE
/// twins); beside the row the pointer rests on, the item with that part.
/// The mouse wheel over the list, closed, steps through them too.
fn pick_part(f: &mut Form<'_>, ui: &mut Ui, what: &str, labels: &[&str], choices: &[Choice]) {
    let current = part_number(&f.root, labels[0]).unwrap_or(0);
    if let Some(v) = pictured(f, ui, (what, choices, current), (labels, &|row| row), true) {
        set_part(f, what, labels, v);
    }
}

/// A list (named `name`, of `choices`, at `current`) of an item's model or
/// part, set in `labels`, in its numbers' order, the mouse wheel stepping
/// through it where `wheel`; the item's picture as it would be with the
/// part `part(row)` beside the row the pointer rests on (on a woman where
/// the page shows it on one), made as the palette's are and kept. The row
/// chosen.
fn pictured(
    f: &mut Form<'_>,
    ui: &mut Ui,
    (name, choices, current): (&str, &[Choice], i64),
    (labels, part): (&[&str], &dyn Fn(i64) -> i64),
    wheel: bool,
) -> Option<i64> {
    let female = on_woman(f);
    let key = f.key;
    let (app, root) = (&mut *f.app, &f.root);
    let mut shown = move |ui: &mut Ui, row: usize| {
        let n = part(row as i64).clamp(0, i64::from(u16::MAX)) as u16;
        let item = labels.iter().fold(root.clone(), |item, label| with_model(&item, label, n));
        super::item_picture(app, ui, &item, female);
    };
    let list = super::situated::pick_list(key, name, choices, current, PART_WIDTH).ordered();
    let list = if wheel { list.wheel() } else { list };
    list.about(&mut shown).show(ui)
}

/// A composite item's parts: (name, field, the letter in its models' names).
const COMPOSITE_PARTS: [(&str, &str, &str); 3] =
    [("Top", "ModelPart3", "t"), ("Middle", "ModelPart2", "m"), ("Bottom", "ModelPart1", "b")];

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

fn colors(f: &mut Form<'_>, ui: &mut Ui, game: &GameData) {
    egui::Grid::new(("uti-colors", f.key)).num_columns(4).spacing([12.0, 6.0]).show(ui, |ui| {
        // The colors of layered items and armor, and their palettes.
        for (i, channel) in mg_rules::items::ArmorChannel::ALL.into_iter().enumerate() {
            let (text, label, palette) = (channel.name(), channel.field(), channel.palette());
            crate::widgets::field_label(ui, text);
            f.palette_color(ui, Some(game), text, label, palette);
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
}

/// An armor's parts' own colors (EE; Aurora has no such page): a part to
/// choose, and for each channel its color or the armor's.
fn part_colors(f: &mut Form<'_>, ui: &mut Ui, game: &GameData) {
    use mg_rules::items::{
        ARMOR_PARTS as PARTS, ArmorChannel, armor_part_color, armor_part_color_label,
    };
    let own = |f: &Form<'_>, part: usize| {
        ArmorChannel::ALL.iter().any(|&c| armor_part_color(&f.root, part, c).is_some())
    };
    let id = egui::Id::new(("uti-part-colors", f.key));
    let mut part: usize = ui.data(|d| d.get_temp(id)).unwrap_or(7);
    ui.horizontal(|ui| {
        crate::widgets::field_label(ui, "Part Colors");
        // (A part with colors of its own is marked.)
        let name = |f: &Form<'_>, p: usize| {
            if own(f, p) { format!("{} *", PARTS[p].1) } else { PARTS[p].1.to_string() }
        };
        let parts: Vec<usize> = (0..PARTS.len()).collect();
        crate::widgets::dropdown(id).selected_text(name(f, part)).show(
            ui,
            &mut part,
            &parts,
            |p| name(f, *p),
        );
    })
    .response
    .on_hover_text("A part takes the armor's color where it has none of its own (*: it has).");
    ui.data_mut(|d| d.insert_temp(id, part));
    egui::Grid::new(("uti-part-colors", f.key)).num_columns(2).spacing([12.0, 6.0]).show(
        ui,
        |ui| {
            for channel in ArmorChannel::ALL {
                let palette = channel.palette();
                let what = format!("{} {}", PARTS[part].1, channel.name());
                let label = armor_part_color_label(part, channel);
                let color = armor_part_color(&f.root, part, channel);
                ui.label(channel.name());
                ui.horizontal(|ui| {
                    // Without one of its own: the armor's, as it shows.
                    let shown = color.unwrap_or(f.int(channel.field()).clamp(0, 175) as u8);
                    let text = color.is_none().then_some("Default");
                    if let Some(v) =
                        f.palette_pick(ui, Some(game), &what, &label, palette, shown, text)
                        && Some(v) != color
                    {
                        f.set(&what, &label, Value::Byte(v));
                    }
                    let reset = ui
                        .add_enabled(color.is_some(), egui::Button::new("Reset").small())
                        .on_hover_text("Use the armor's color");
                    reset.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            true,
                            format!("Reset {what}"),
                        )
                    });
                    if reset.clicked() {
                        f.set_opt(&what, &label, None);
                    }
                });
                ui.end_row();
            }
        },
    );
}

/// The item with another model number (and its EE twin, where it has one
/// or the number needs one).
fn with_model(item: &Struct, label: &str, n: u16) -> Struct {
    let mut s = item.clone();
    s.set(label, Value::Byte(n.min(255) as u8));
    if n > 255 || s.get(&wide_label(label)).is_some() {
        s.set(&wide_label(label), Value::Word(n));
    }
    s
}

/// An item's icon layers (the game lent to the page).
fn icon(f: &mut Form<'_>, ui: &Ui, game: &GameData, item: &Struct) -> Vec<crate::images::Picture> {
    // (As a woman's inventory shows it, where the page shows it on one.)
    let female = f.app.settings.armor_on_woman;
    let mut loader = crate::images::Loader {
        pictures: &mut f.app.pictures,
        palettes: &mut f.app.palettes,
        ws: f.app.ws.as_ref(),
        game,
    };
    loader.item_icon_on(ui.ctx(), item, female)
}

/// One part of a composite item's icon alone (see
/// [`crate::images::Loader::item_icon_part`]).
fn part_icon(
    f: &mut Form<'_>,
    ui: &Ui,
    game: &GameData,
    item: &Struct,
    part: (&str, &str),
) -> Option<crate::images::Picture> {
    let mut loader = crate::images::Loader {
        pictures: &mut f.app.pictures,
        palettes: &mut f.app.palettes,
        ws: f.app.ws.as_ref(),
        game,
    };
    loader.item_icon_part(ui.ctx(), item, part)
}

/// Aurora's icon grid: the icon of each model number, a click chooses it.
fn icon_grid(f: &mut Form<'_>, ui: &mut Ui, game: &GameData, numbers: &[u16]) {
    part_grid(f, ui, game, numbers, ("Appearance", "ModelPart1"), None, 220.0);
}

/// The largest a composite item's part is drawn among its choices: a
/// great sword's blade is 190 points in its icon, and three lists of those
/// are more than a window.
const PART_TALLEST: f32 = 96.0;

/// The tallest the page's icon is drawn (half as large again as it is, a
/// great sword's 285 points), and a model's among its choices: a hak's
/// icon may be any size.
const ICON_TALLEST: f32 = 300.0;
const CHOICE_TALLEST: f32 = 128.0;

/// The item's icon with each of `numbers` as its part `label` (named
/// `what`), in a grid at most `height` tall: a click chooses the part.
/// For a composite item's part (`letter` in its icons' names), that part
/// alone and no more room than it fills: the icon is the whole item's
/// size, most of it empty.
fn part_grid(
    f: &mut Form<'_>,
    ui: &mut Ui,
    game: &GameData,
    numbers: &[u16],
    (what, label): (&str, &str),
    letter: Option<&str>,
    height: f32,
) {
    let current = part_number(&f.root, label).unwrap_or(0);
    let mut chosen = None;
    let id = ("uti-icons", f.key, label);
    // As tall as whole rows of icons (their height as last drawn), so the
    // tips of the next row don't show under the last one in view.
    let row_id = egui::Id::new(id).with("row");
    let gap = ui.spacing().item_spacing.y;
    let height = match ui.data(|d| d.get_temp::<f32>(row_id)).filter(|h| *h > 0.0) {
        Some(row) => {
            let rows = ((height + gap) / (row + gap)).floor().max(1.0);
            rows * (row + gap) - gap
        }
        None => height,
    };
    let mut tallest: f32 = 0.0;
    egui::ScrollArea::vertical().max_height(height).id_salt(id).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for &n in numbers {
                let item = with_model(&f.root, label, n);
                let alt = format!("{what} {n}");
                let alone = letter.and_then(|l| part_icon(f, ui, game, &item, (label, l)));
                let r = match alone {
                    Some(p) => crate::images::cropped(ui, &p, PART_TALLEST, &alt),
                    // A part with a model and no icon: its number.
                    None if letter.is_some() => {
                        let text = egui::RichText::new(n.to_string()).weak();
                        let r = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                        r.widget_info(|| {
                            egui::WidgetInfo::labeled(egui::WidgetType::Image, true, &alt)
                        });
                        r
                    }
                    None => {
                        let layers = icon(f, ui, game, &item);
                        crate::images::stacked_drawn(ui, &layers, (1.0, CHOICE_TALLEST), &alt)
                    }
                }
                .on_hover_text(alt);
                tallest = tallest.max(r.rect.height());
                if i64::from(n) == current {
                    let stroke = ui.visuals().selection.stroke;
                    // (Within the icon's own room: outside it, the grid's
                    // edge cut the top of the first row's off.)
                    ui.painter().rect_stroke(r.rect, 2.0, stroke, egui::StrokeKind::Inside);
                }
                if r.clicked() {
                    chosen = Some(n);
                }
            }
        });
    });
    if ui.data(|d| d.get_temp::<f32>(row_id)) != Some(tallest) {
        ui.data_mut(|d| d.insert_temp(row_id, tallest));
        ui.ctx().request_repaint();
    }
    if let Some(n) = chosen.filter(|&n| i64::from(n) != current) {
        set_part(f, what, &[label], i64::from(n));
    }
}

/// `beside`: the model is shown beside these fields (a wide window). In a
/// narrower one it is shown here, to the right of the icon.
fn appearance(f: &mut Form<'_>, ui: &mut Ui, game: &GameData, beside: bool) {
    let Ok(t) = game.table("baseitems") else { return };
    let row = base_row(f);
    let class = t.get(row, "ItemClass").unwrap_or_default().to_ascii_lowercase();
    // The item's inventory icon, as it is now.
    let root = f.root.clone();
    let layers = icon(f, ui, game, &root);
    if beside {
        crate::images::stacked_drawn(ui, &layers, (1.5, ICON_TALLEST), "Icon");
    } else {
        // (The page's visible width: see `situated::beside_model`.)
        let seen = ui.available_width().min(ui.clip_rect().right() - ui.cursor().left());
        ui.horizontal_top(|ui| {
            let icon = crate::images::stacked_drawn(ui, &layers, (1.5, ICON_TALLEST), "Icon").rect;
            let gap = ui.spacing().item_spacing.x;
            let size = egui::vec2(seen - icon.width() - 2.0 * gap, icon.height().max(MODEL_SHORT));
            ui.vertical(|ui| super::situated::inline_preview(f, ui, size));
        });
    }
    // Who wears it in the model and the icon shown: armor, cloaks and the
    // items the game has an icon of for each have a man's and a woman's.
    // (Options › General has the choice too, kept: inventories and the
    // palette show the icons and models so.)
    let kind = t.get_int(row, "ModelType").unwrap_or(0);
    if worn_by_either(game, row) {
        ui.horizontal(|ui| {
            crate::widgets::field_label(ui, "Shown on");
            let on = &mut f.app.settings.armor_on_woman;
            ui.selectable_value(on, false, "Male");
            ui.selectable_value(on, true, "Female");
        })
        .response
        .on_hover_text(
            "The body the item's model is shown on, and whose inventory icon is shown, here, \
             in inventories and in the palette (Options › General); the item is the same",
        );
    }
    match kind {
        // Composite: bottom, middle, top, each a shape and a colour (the
        // number's tens and units).
        2 => {
            egui::Grid::new(("uti-parts", f.key)).num_columns(3).spacing([12.0, 6.0]).show(
                ui,
                |ui| {
                    for (text, label, part) in COMPOSITE_PARTS {
                        let numbers = part_numbers(ui, game, &format!("{class}_{part}_"));
                        let current = part_number(&f.root, label).unwrap_or(0);
                        let (shape, color) = (current / 10, current % 10);
                        let mut shapes: Vec<u16> = numbers.iter().map(|n| n / 10).collect();
                        shapes.dedup();
                        let shape_numbers: Vec<u16> = shapes.iter().map(|s| s * 10).collect();
                        crate::widgets::field_label(ui, text);
                        let shape_choices: Vec<Choice> = shape_numbers
                            .iter()
                            .map(|&n| Choice {
                                row: (n / 10) as usize,
                                text: format!("Model {}", n / 10),
                            })
                            .collect();
                        // Keep the colour where the new shape has it.
                        let with_shape = |s: i64| {
                            let v = s * 10 + color;
                            if numbers.contains(&(v as u16)) {
                                v
                            } else {
                                numbers
                                    .iter()
                                    .find(|&&n| i64::from(n / 10) == s)
                                    .map_or(v, |&n| n.into())
                            }
                        };
                        let list = (&*format!("{label}-shape"), &shape_choices[..], shape);
                        if let Some(s) = pictured(f, ui, list, (&[label], &with_shape), false) {
                            set_part(f, text, &[label], with_shape(s));
                        }
                        let color_choices: Vec<Choice> = numbers
                            .iter()
                            .filter(|&&n| i64::from(n / 10) == shape)
                            .map(|&n| Choice {
                                row: (n % 10) as usize,
                                text: format!("Color {}", n % 10),
                            })
                            .collect();
                        let list = (&*format!("{label}-color"), &color_choices[..], color);
                        let with_color = |c: i64| shape * 10 + c;
                        if let Some(c) = pictured(f, ui, list, (&[label], &with_color), false) {
                            set_part(f, text, &[label], with_color(c));
                        }
                        ui.end_row();
                    }
                },
            );
            // Each part's shapes and colors as pictures: the item as it
            // would be with each, a click chooses it.
            for (text, label, part) in COMPOSITE_PARTS {
                let numbers = part_numbers(ui, game, &format!("{class}_{part}_"));
                if numbers.len() < 2 {
                    continue;
                }
                ui.separator();
                crate::widgets::field_label(ui, text);
                part_grid(f, ui, game, &numbers, (text, label), Some(part), 150.0);
            }
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
                                crate::widgets::field_label(ui, *text);
                                // (0 as well, which most of the game's
                                // tables leave out: no part, so the
                                // wearer's own shows there, as in the
                                // game. A builder's report.)
                                let choices = if *table == "parts_chest" {
                                    super::situated::part_choices(&numbers, |n| {
                                        format!("{n} (AC {})", ac_of(n).round())
                                    })
                                } else {
                                    super::situated::part_choices(&numbers, |n| n.to_string())
                                };
                                pick_part(f, ui, text, labels, &choices);
                                ui.end_row();
                            }
                        });
                }
                colors(f, &mut cols[1], game);
                cols[1].separator();
                part_colors(f, &mut cols[1], game);
            });
        }
        // Simple and layered: one model (layered ones take colours).
        kind => {
            let prefix = if class == "cloak" { String::new() } else { format!("{class}_") };
            // The numbers to choose from: a cloak's are cloakmodel.2da's
            // rows, another item's those it has icons for.
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
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Appearance");
                part_choice(f, ui, "Appearance", &["ModelPart1"], &numbers, |n| n.to_string());
            });
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
    crate::widgets::two_columns(ui, 340.0, |ui, col| {
        if col == 0 {
            crate::widgets::section_heading(ui, "Available Properties");
            // Search: the properties and their choices (a spell of Cast
            // Spell, a skill of Skill Bonus) that have every word typed,
            // their kinds opened out.
            let search_id = egui::Id::new(("uti-search", key));
            let mut search: String = ui.data(|d| d.get_temp(search_id)).unwrap_or_default();
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut search)
                        .hint_text("🔍 Search properties")
                        .desired_width(ui.available_width() - 28.0),
                );
                if !search.is_empty() && ui.small_button("×").on_hover_text("Clear").clicked() {
                    search.clear();
                }
            });
            ui.data_mut(|d| d.insert_temp(search_id, search.clone()));
            let words: Vec<String> = search.split_whitespace().map(str::to_lowercase).collect();
            let has = |text: &str| {
                let text = text.to_lowercase();
                words.iter().all(|w| text.contains(w.as_str()))
            };
            egui::ScrollArea::vertical().id_salt(("uti-available", key)).max_height(380.0).show(
                ui,
                |ui| {
                    let mut found = 0;
                    for t in &available {
                        let mut subtypes = game.property_subtypes(t, base);
                        // (A kind found by its own name shows all of its
                        // choices; else those found.)
                        if !words.is_empty() && !has(&t.name) {
                            subtypes.retain(|s| has(&format!("{} {}", t.name, s.text)));
                            if subtypes.is_empty() {
                                continue;
                            }
                        }
                        found += 1;
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
                                .open((!words.is_empty()).then_some(true))
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
                    if found == 0 {
                        ui.weak("No property has that.");
                    }
                },
            );
        } else {
            crate::widgets::section_heading(ui, "Assigned Properties");
            egui::ScrollArea::vertical().id_salt(("uti-assigned", key)).max_height(200.0).show(
                ui,
                |ui| {
                    for (i, p) in assigned.iter().enumerate() {
                        if ui.selectable_label(selected == Some(i), game.property_text(p)).clicked()
                        {
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
            vec![Edit::insert_item(
                key,
                here.clone(),
                "PropertiesList",
                assigned.len(),
                p.to_gff(),
            )],
        )));
        selected = Some(assigned.len());
    }
    if let Some(i) = remove {
        f.app.actions.push(Action::Apply(Command::new(
            "Remove property",
            vec![Edit::remove_item(key, here, "PropertiesList", i)],
        )));
        selected = None;
    }
    ui.data_mut(|d| d.insert_temp(state_id, (chosen, chosen_sub, selected)));
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
    let path = here.item("PropertiesList", index);
    let mut changes: Vec<(&str, Value)> = Vec::new();
    egui::Grid::new(("uti-prop", f.key, index)).num_columns(2).spacing([12.0, 6.0]).show(
        ui,
        |ui| {
            crate::widgets::field_label(ui, "Item Property");
            ui.strong(&t.name);
            ui.end_row();
            let subtypes = game.property_subtypes(&t, base);
            if !subtypes.is_empty() {
                crate::widgets::field_label(ui, "Sub-Property");
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
                crate::widgets::field_label(ui, "Cost Parameter");
                if let Some(v) = super::situated::pick_ordered(
                    ui,
                    f.key,
                    "prop-cost",
                    &costs,
                    i64::from(p.cost_value),
                ) {
                    changes.push(("CostValue", Value::Word(v as u16)));
                }
                ui.end_row();
            }
            if p.param1 != 255 {
                let params = game.property_params(p.param1);
                crate::widgets::field_label(ui, "Parameter 1");
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
            crate::widgets::field_label(ui, "Chance of Appearing (%)");
            if let Some(v) = spin_number(ui, i64::from(p.chance), 0..=100) {
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
    if let Some(game) = f.app.game.as_deref() {
        let row = base_row(f);
        let text = game
            .table("baseitems")
            .ok()
            .and_then(|t| strref(game, t.get_int(row, "Description")))
            .unwrap_or_default();
        crate::widgets::field_label(ui, "Item Type Description");
        ui.weak(text);
        ui.separator();
        crate::widgets::field_label(ui, "Item Statistics");
        for s in f.root.list("PropertiesList").unwrap_or(&[]) {
            ui.weak(game.property_text(&ItemProperty::from_gff(s)));
        }
    }
    ui.separator();
    f.variables(ui);
    f.update_instances(ui);
}
