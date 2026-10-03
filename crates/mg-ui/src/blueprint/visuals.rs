//! The Visuals page: what EE's scripting can change about how an object
//! looks, saved on the object and read back by the game when it loads the
//! object (`mg-corpus-tests/tests/engine_ee_fields.rs`). Aurora has no
//! fields for them.
//!
//! - Texture and animation replacements (`ReplaceObjectTexture`,
//!   `ReplaceObjectAnimation`: `TextureReplace`, `AnimationReplace`).
//! - Shader parameters (`SetMaterialShaderUniformInt` and `…Vec4`:
//!   `Material`).
//! - `MiscVisuals`: the highlight color, the mouse cursor, the text bubble,
//!   what shows on mouse-over and Tab, and how far away the object is seen.
//!
//! The game reads them on placeables, doors, creatures and items; on
//! triggers, only `MiscVisuals`.

use egui::Ui;
use mg_core::ResRef;
use mg_gff::{Struct, Value};

use super::Form;
use crate::text::{decode, encode};
use crate::widgets::{commit_number, commit_text};

/// The struct ids the game writes.
const TEXTURE_REPLACE_ID: u32 = 9;
const ANIMATION_REPLACE_ID: u32 = 11;
const MATERIAL_ID: u32 = 7;
const MISC_VISUALS_ID: u32 = 8;

/// `MiscVisuals`' defaults, as the game writes them for an object nothing
/// changed.
const DEFAULT_DISTANCE: f32 = 45.0;
const DEFAULT_DISCOVERY: i64 = 15;

/// `OBJECT_UI_DISCOVERY_*`.
const DISCOVERY: [(i64, &str); 4] = [
    (1, "Highlight on mouse-over"),
    (2, "Highlight with Tab"),
    (4, "Name on mouse-over"),
    (8, "Name with Tab"),
];

/// `OBJECT_UI_TEXT_BUBBLE_OVERRIDE_*`.
const BUBBLE_MODES: [&str; 4] = ["Name", "Replace the name", "Before the name", "After the name"];

/// `MOUSECURSOR_*` (each has a pressed form one higher, not listed).
const CURSORS: [(i64, &str); 32] = [
    (1, "Default"),
    (3, "Walk"),
    (5, "No walk"),
    (7, "Attack"),
    (9, "No attack"),
    (11, "Talk"),
    (13, "No talk"),
    (15, "Follow"),
    (17, "Examine"),
    (19, "No examine"),
    (21, "Transition"),
    (23, "Door"),
    (25, "Use"),
    (27, "No use"),
    (29, "Magic"),
    (31, "No magic"),
    (33, "Disarm"),
    (35, "No disarm"),
    (37, "Action"),
    (39, "No action"),
    (41, "Lock"),
    (43, "No lock"),
    (45, "Pushpin"),
    (47, "Create"),
    (49, "No create"),
    (51, "Kill"),
    (53, "No kill"),
    (55, "Heal"),
    (57, "No heal"),
    (59, "Run arrow"),
    (75, "Walk arrow"),
    (91, "Pick up"),
];

/// A cursor's name: a listed one, its pressed form, or a custom cursor
/// (`MOUSECURSOR_CUSTOM_00` … `_99`: `gui_mp_customNNu`).
fn cursor_name(v: i64) -> String {
    if v < 0 {
        return "Default for the object".into();
    }
    if let Some((_, n)) = CURSORS.iter().find(|(c, _)| *c == v) {
        return (*n).into();
    }
    if let Some((_, n)) = CURSORS.iter().find(|(c, _)| *c + 1 == v) {
        return format!("{n} (pressed)");
    }
    if (93..=292).contains(&v) {
        let n = (v - 93) / 2;
        return if (v - 93) % 2 == 0 {
            format!("Custom {n:02}")
        } else {
            format!("Custom {n:02} (pressed)")
        };
    }
    format!("({v})")
}

/// The page: everything (`full`), or only `MiscVisuals` (triggers).
pub(super) fn page(f: &mut Form<'_>, ui: &mut Ui, full: bool) {
    if full {
        crate::widgets::section_heading(ui, "Texture replacements");
        ui.weak("A texture of the object's model drawn with another (not PLT textures).");
        pairs(f, ui, Pairs::TEXTURES);
        ui.separator();
        crate::widgets::section_heading(ui, "Animation replacements");
        ui.weak("An animation of the object's model played as another.");
        pairs(f, ui, Pairs::ANIMATIONS);
        ui.separator();
        crate::widgets::section_heading(ui, "Shader parameters");
        ui.weak("Values a material's shader reads (uniforms).");
        shader_params(f, ui);
        ui.separator();
    }
    misc(f, ui);
}

/// A replacement list's labels.
struct Pairs {
    what: &'static str,
    field: &'static str,
    id: u32,
    list: &'static str,
    old: &'static str,
    new: &'static str,
    titles: (&'static str, &'static str),
}

impl Pairs {
    const TEXTURES: Pairs = Pairs {
        what: "Texture replacement",
        field: "TextureReplace",
        id: TEXTURE_REPLACE_ID,
        list: "TextureReplaceLi",
        old: "OldTexture",
        new: "NewTexture",
        titles: ("Texture", "Drawn as"),
    };
    const ANIMATIONS: Pairs = Pairs {
        what: "Animation replacement",
        field: "AnimationReplace",
        id: ANIMATION_REPLACE_ID,
        list: "AnimationReplace",
        old: "OldAnimation",
        new: "NewAnimation",
        titles: ("Animation", "Played as"),
    };
}

impl Form<'_> {
    /// The items of a list inside a struct field (`TextureReplace` ›
    /// `TextureReplaceLi`).
    fn nested_list(&self, field: &str, list: &str) -> Vec<Struct> {
        self.root
            .child(field)
            .and_then(|s| s.list(list))
            .map(<[Struct]>::to_vec)
            .unwrap_or_default()
    }

    /// Sets the list inside a struct field, keeping the struct's other
    /// fields; an empty list removes the field.
    fn set_nested_list(
        &mut self,
        what: &str,
        field: &str,
        id: u32,
        list: &str,
        items: Vec<Struct>,
    ) {
        let value = (!items.is_empty()).then(|| {
            let mut s = self.root.child(field).cloned().unwrap_or_else(|| Struct::new(id));
            s.set(list, Value::List(items));
            Value::Struct(s)
        });
        self.set_opt(what, field, value);
    }

    /// Sets a field of `MiscVisuals`, keeping the others.
    fn set_misc(&mut self, what: &str, fields: &[(&str, Value)]) {
        let mut s =
            self.root.child("MiscVisuals").cloned().unwrap_or_else(|| Struct::new(MISC_VISUALS_ID));
        for (label, v) in fields {
            s.set(label, v.clone());
        }
        self.set(what, "MiscVisuals", Value::Struct(s));
    }
}

fn resref_text(s: &Struct, label: &str) -> String {
    s.resref(label).map(|r| r.to_string()).unwrap_or_default()
}

/// Old → new pairs, each with Remove, and a row to add one.
fn pairs(f: &mut Form<'_>, ui: &mut Ui, p: Pairs) {
    let items = f.nested_list(p.field, p.list);
    let mut changed: Option<Vec<Struct>> = None;
    egui::Grid::new(f.id(p.field)).num_columns(3).min_col_width(140.0).spacing([12.0, 4.0]).show(
        ui,
        |ui| {
            crate::widgets::field_label(ui, p.titles.0);
            ui.label(p.titles.1);
            ui.end_row();
            for (i, e) in items.iter().enumerate() {
                for label in [p.old, p.new] {
                    let id = f.id(&format!("{}#{i}#{label}", p.field));
                    let current = resref_text(e, label);
                    if let Some(v) = commit_text(f.app, ui, id, &current, false, 140.0)
                        && let Ok(r) = ResRef::from_str(v.trim())
                    {
                        let mut all = items.clone();
                        all[i].set(label, Value::resref(r));
                        changed = Some(all);
                    }
                }
                if ui.small_button("Remove").clicked() {
                    let mut all = items.clone();
                    all.remove(i);
                    changed = Some(all);
                }
                ui.end_row();
            }
            // A new pair.
            let ids = [p.old, p.new].map(|l| f.id(&format!("{}#new#{l}", p.field)));
            for (id, hint) in ids.into_iter().zip([p.titles.0, p.titles.1]) {
                let buf = f.app.buffers.entry(id).or_default();
                ui.add(
                    egui::TextEdit::singleline(buf)
                        .char_limit(16)
                        .desired_width(140.0)
                        .hint_text(hint),
                );
            }
            let text = |f: &Form<'_>, id| {
                f.app.buffers.get(&id).map(|s| s.trim().to_string()).unwrap_or_default()
            };
            let (old, new) = (text(f, ids[0]), text(f, ids[1]));
            let valid = ResRef::from_str(&old).is_ok_and(|r| !r.is_empty())
                && ResRef::from_str(&new).is_ok();
            if ui.add_enabled(valid, egui::Button::new("Add").small()).clicked() {
                let mut e = Struct::new(0);
                e.set(p.old, Value::resref(ResRef::from_str(&old).expect("checked")));
                e.set(p.new, Value::resref(ResRef::from_str(&new).expect("checked")));
                let mut all = items.clone();
                all.push(e);
                changed = Some(all);
                for id in ids {
                    f.app.buffers.remove(&id);
                }
            }
            ui.end_row();
        },
    );
    if let Some(all) = changed {
        f.set_nested_list(p.what, p.field, p.id, p.list, all);
    }
}

/// Shader parameters: material, parameter, and an integer or four floats.
fn shader_params(f: &mut Form<'_>, ui: &mut Ui) {
    let items = f.nested_list("Material", "ShaderParams");
    let mut changed: Option<Vec<Struct>> = None;
    let text = |s: &Struct, l: &str| decode(s.string(l).unwrap_or_default());
    egui::Grid::new(f.id("ShaderParams")).num_columns(5).spacing([12.0, 4.0]).show(ui, |ui| {
        if !items.is_empty() {
            for l in ["Material", "Parameter", "Type", "Value"] {
                ui.strong(l);
            }
            ui.end_row();
        }
        for (i, e) in items.iter().enumerate() {
            for label in ["Material", "Param"] {
                let id = f.id(&format!("ShaderParams#{i}#{label}"));
                if let Some(v) = commit_text(f.app, ui, id, &text(e, label), false, 120.0) {
                    let mut all = items.clone();
                    all[i].set(label, Value::String(encode(v.trim())));
                    changed = Some(all);
                }
            }
            let kind = e.integer("Type").unwrap_or(0);
            let mut pick = kind;
            egui::ComboBox::from_id_salt(f.id(&format!("ShaderParams#{i}#Type")))
                .selected_text(match kind {
                    1 => "Integer".to_string(),
                    2 => "Vector".to_string(),
                    k => format!("({k})"),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut pick, 1, "Integer");
                    ui.selectable_value(&mut pick, 2, "Vector");
                });
            if pick != kind {
                let mut all = items.clone();
                all[i].set("Type", Value::Byte(pick as u8));
                changed = Some(all);
            }
            ui.horizontal(|ui| match kind {
                1 => {
                    let v = e.integer("Int").unwrap_or(0);
                    if let Some(n) = commit_number(ui, v, i64::from(i32::MIN)..=i64::from(i32::MAX))
                    {
                        let mut all = items.clone();
                        all[i].set("Int", Value::Int(n as i32));
                        changed = Some(all);
                    }
                }
                2 => {
                    for k in 1..=4 {
                        let label = format!("Float{k}");
                        let v = e.float(&label).unwrap_or(0.0);
                        let mut n = v;
                        let r = ui.add(egui::DragValue::new(&mut n).speed(0.01).max_decimals(3));
                        if (r.drag_stopped() || (r.changed() && !r.dragged())) && n != v {
                            let mut all = items.clone();
                            all[i].set(&label, Value::Float(n));
                            changed = Some(all);
                        }
                    }
                }
                _ => {}
            });
            if ui.small_button("Remove").clicked() {
                let mut all = items.clone();
                all.remove(i);
                changed = Some(all);
            }
            ui.end_row();
        }
    });
    let ids = ["Material", "Param"].map(|l| f.id(&format!("ShaderParams#new#{l}")));
    ui.horizontal(|ui| {
        for (id, hint) in ids.iter().zip(["material", "parameter"]) {
            let buf = f.app.buffers.entry(*id).or_default();
            ui.add(egui::TextEdit::singleline(buf).desired_width(120.0).hint_text(hint));
        }
        let text = |id| f.app.buffers.get(&id).map(|s| s.trim().to_string()).unwrap_or_default();
        let (material, param) = (text(ids[0]), text(ids[1]));
        let valid = !material.is_empty() && !param.is_empty();
        if ui.add_enabled(valid, egui::Button::new("Add").small()).clicked() {
            let mut e = Struct::new(0);
            e.set("Material", Value::String(encode(&material)));
            e.set("Param", Value::String(encode(&param)));
            e.set("Type", Value::Byte(1));
            e.set("Int", Value::Int(0));
            let mut all = items.clone();
            all.push(e);
            changed = Some(all);
        }
    });
    if changed.as_ref().is_some_and(|all| all.len() > items.len()) {
        for id in ids {
            f.app.buffers.remove(&id);
        }
    }
    if let Some(all) = changed {
        f.set_nested_list("Shader parameter", "Material", MATERIAL_ID, "ShaderParams", all);
    }
}

/// `MiscVisuals`.
fn misc(f: &mut Form<'_>, ui: &mut Ui) {
    let s = f.root.child("MiscVisuals").cloned().unwrap_or_default();
    let int = |l: &str, d: i64| s.integer(l).unwrap_or(d);
    let rgb = ["HiliteColorR", "HiliteColorG", "HiliteColorB"].map(|l| s.float(l).unwrap_or(-1.0));
    egui::Grid::new(f.id("MiscVisuals")).num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        // The highlight color: -1 each for the game's.
        crate::widgets::field_label(ui, "Highlight Color");
        ui.horizontal(|ui| {
            let custom = rgb.iter().all(|c| *c >= 0.0);
            let mut on = custom;
            if ui.checkbox(&mut on, "Own color").changed() {
                let v = if on { [1.0, 1.0, 1.0] } else { [-1.0; 3] };
                set_highlight(f, v);
            }
            if custom {
                let pending_id = f.id("HiliteColor").with("pending");
                let pending: Option<[f32; 3]> = ui.data(|d| d.get_temp(pending_id));
                let mut c = pending.unwrap_or(rgb);
                let (picked, open) = ui
                    .push_id(f.id("HiliteColor"), |ui| {
                        let popup = ui.auto_id_with("popup");
                        let changed = ui.color_edit_button_rgb(&mut c).changed();
                        (changed, egui::Popup::is_id_open(ui.ctx(), popup))
                    })
                    .inner;
                if picked {
                    ui.data_mut(|d| d.insert_temp(pending_id, c));
                }
                if !open && let Some(v) = ui.data(|d| d.get_temp::<[f32; 3]>(pending_id)) {
                    ui.data_mut(|d| d.remove::<[f32; 3]>(pending_id));
                    if v != rgb {
                        set_highlight(f, v);
                    }
                }
            }
        });
        ui.end_row();

        crate::widgets::field_label(ui, "Mouse Cursor");
        let current = int("MouseCursor", -1);
        let mut pick = current;
        egui::ComboBox::from_id_salt(f.id("MouseCursor"))
            .selected_text(cursor_name(current))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut pick, -1, cursor_name(-1));
                for (v, n) in CURSORS {
                    ui.selectable_value(&mut pick, v, n);
                }
                for n in 0..100 {
                    ui.selectable_value(&mut pick, 93 + 2 * n, format!("Custom {n:02}"));
                }
            });
        if pick != current {
            f.set_misc("Mouse cursor", &[("MouseCursor", Value::Int(pick as i32))]);
        }
        ui.end_row();

        crate::widgets::field_label(ui, "Text Bubble");
        ui.horizontal(|ui| {
            let mode = int("TextBubbleType", 0);
            let mut pick = mode;
            egui::ComboBox::from_id_salt(f.id("TextBubbleType"))
                .selected_text(BUBBLE_MODES.get(mode as usize).copied().unwrap_or("?"))
                .show_ui(ui, |ui| {
                    for (i, n) in BUBBLE_MODES.iter().enumerate() {
                        ui.selectable_value(&mut pick, i as i64, *n);
                    }
                });
            if pick != mode {
                f.set_misc("Text bubble", &[("TextBubbleType", Value::Int(pick as i32))]);
            }
            if mode != 0 {
                let current = decode(s.string("TextBubbleText").unwrap_or_default());
                if let Some(v) =
                    commit_text(f.app, ui, f.id("TextBubbleText"), &current, false, 200.0)
                {
                    f.set_misc("Text bubble", &[("TextBubbleText", Value::String(encode(&v)))]);
                }
            }
        });
        ui.end_row();

        // What shows on mouse-over and Tab: -1 for the game's choice.
        crate::widgets::field_label(ui, "Shown");
        ui.vertical(|ui| {
            let mask = int("UiDiscoverMask", -1);
            let mut default = mask < 0;
            if ui.checkbox(&mut default, "As the game decides").changed() {
                let v = if default { -1 } else { DEFAULT_DISCOVERY };
                f.set_misc("Shown", &[("UiDiscoverMask", Value::Int(v as i32))]);
            }
            if !default {
                ui.horizontal_wrapped(|ui| {
                    for (bit, n) in DISCOVERY {
                        let mut on = mask & bit != 0;
                        if ui.checkbox(&mut on, n).changed() {
                            let v = if on { mask | bit } else { mask & !bit };
                            f.set_misc("Shown", &[("UiDiscoverMask", Value::Int(v as i32))]);
                        }
                    }
                });
            }
        });
        ui.end_row();

        crate::widgets::field_label(ui, "Visible Distance");
        ui.horizontal(|ui| {
            let v = s.float("VisibleDistance").unwrap_or(DEFAULT_DISTANCE);
            let mut n = v;
            let r = ui.add(
                egui::DragValue::new(&mut n)
                    .range(0.0..=1000.0)
                    .speed(0.5)
                    .max_decimals(1)
                    .suffix(" m"),
            );
            if (r.drag_stopped() || (r.changed() && !r.dragged())) && n != v {
                f.set_misc("Visible distance", &[("VisibleDistance", Value::Float(n))]);
            }
            ui.weak(format!("the game's default is {DEFAULT_DISTANCE} m"));
        });
        ui.end_row();
    });
}

fn set_highlight(f: &mut Form<'_>, rgb: [f32; 3]) {
    f.set_misc(
        "Highlight color",
        &[
            ("HiliteColorR", Value::Float(rgb[0])),
            ("HiliteColorG", Value::Float(rgb[1])),
            ("HiliteColorB", Value::Float(rgb[2])),
        ],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_names() {
        assert_eq!(cursor_name(-1), "Default for the object");
        assert_eq!(cursor_name(25), "Use");
        assert_eq!(cursor_name(26), "Use (pressed)");
        assert_eq!(cursor_name(93), "Custom 00");
        assert_eq!(cursor_name(292), "Custom 99 (pressed)");
        assert_eq!(cursor_name(500), "(500)");
    }
}
