//! The Tint section of a placeable's Visuals page: The Domains of Dread's
//! colorways ([`mg_rules::tint`]), the color its scripts give a
//! placeable's fabric as the module loads, set as the placeable's locals
//! (`tdod_tint` for the whole object, `tdod_tint_<mtr>` for one
//! material) with a color picker each. Shown for a placeable whose model
//! has a material that takes a tint (an MTR declaring `tdodTint`), or
//! that has a tint's local.

use std::sync::Arc;

use egui::Ui;
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;
use mg_rules::tint;

use super::Form;
use crate::Action;

/// The section.
pub(super) fn section(f: &mut Form<'_>, ui: &mut Ui) {
    let Some(game) = f.app.game.clone() else { return };
    let materials = tintable(ui, &game, &f.root);
    let tints = tint::of_object(&f.root);
    if materials.is_empty() && tints.is_empty() {
        return;
    }
    crate::widgets::section_heading(ui, "Tint");
    ui.weak(
        "The color The Domains of Dread's scripts give the placeable's fabric as the module \
         loads (its tdod_tint variables): the whole object's, and a material's own over it.",
    );
    if !tint::takes_tint(&f.root) {
        let text = "Static: the game draws a static placeable as made, without its tint. Untick \
                    Static on the Basic page for the tint to show.";
        ui.colored_label(ui.visuals().warn_fg_color, text);
    }
    if materials.is_empty() {
        ui.weak("No material of its model takes a tint (an MTR declaring tdodTint).");
    }
    // The whole object's, then each material's: the model's, and any other
    // a local names.
    let mut rows: Vec<(String, String, Option<[f32; 3]>)> =
        vec![("Whole object".into(), tint::TINT.into(), tints.all)];
    let named = tints.by_material.iter().map(|(m, _)| m.clone());
    let mut shown: Vec<String> = materials.iter().cloned().collect();
    shown.extend(named.filter(|m| !materials.contains(m)));
    for m in shown {
        let own = tints.by_material.iter().find(|(n, _)| *n == m).map(|(_, c)| *c);
        rows.push((m.clone(), format!("{}{m}", tint::TINT_PREFIX), own));
    }
    let mut set: Option<(String, Option<String>)> = None;
    egui::Grid::new(f.id("Tint")).num_columns(4).spacing([12.0, 4.0]).show(ui, |ui| {
        for (i, (label, local, own)) in rows.iter().enumerate() {
            crate::widgets::field_label(ui, label).on_hover_text(format!("The variable {local}"));
            // (A material without its own shows the whole object's.)
            let shown = own.or(if i == 0 { None } else { tints.all });
            if let Some(rgb) = picker(f, ui, local, shown) {
                set = Some((local.clone(), Some(tint::format(rgb))));
            }
            match (own, i) {
                (Some(c), _) => ui.monospace(tint::format(c.map(|v| (v * 255.0).round() as u8))),
                (None, 0) => ui.weak("none"),
                (None, _) if tints.all.is_some() => ui.weak("as the whole object"),
                (None, _) => ui.weak("none"),
            };
            if own.is_some() && ui.small_button("Clear").clicked() {
                set = Some((local.clone(), None));
            }
            ui.end_row();
        }
    });
    ui.add_space(crate::widgets::SECTION_GAP);
    if let Some((local, value)) = set {
        let what = if value.is_some() { "Tint" } else { "Clear tint" };
        set_local(f, what, &local, value.as_deref());
    }
}

/// A color's swatch and picker: what is picked once the picker closes
/// (one command, as the area's colors are set).
fn picker(f: &Form<'_>, ui: &mut Ui, local: &str, shown: Option<[f32; 3]>) -> Option<[u8; 3]> {
    let id = f.id(&format!("Tint#{local}"));
    let pending_id = id.with("pending");
    let pending: Option<[u8; 3]> = ui.data(|d| d.get_temp(pending_id));
    let current = shown.map(|c| c.map(|v| (v * 255.0).round() as u8));
    let mut rgb = pending.or(current).unwrap_or([255; 3]);
    let (changed, open) = ui
        .push_id(id, |ui| {
            let popup = ui.auto_id_with("popup");
            let changed = crate::color_button::edit_srgb(ui, &mut rgb)
                .on_hover_text("Pick the color")
                .changed();
            (changed, egui::Popup::is_id_open(ui.ctx(), popup))
        })
        .inner;
    if changed {
        ui.data_mut(|d| d.insert_temp(pending_id, rgb));
    }
    if !open && let Some(v) = ui.data(|d| d.get_temp::<[u8; 3]>(pending_id)) {
        ui.data_mut(|d| d.remove::<[u8; 3]>(pending_id));
        return Some(v);
    }
    None
}

/// The model's tintable materials ([`mg_preview::tintable_materials`]),
/// worked out once a model (and game data) and kept.
fn tintable(ui: &Ui, game: &Arc<mg_rules::GameData>, utp: &Struct) -> Arc<Vec<String>> {
    let appearance = utp.integer("Appearance").unwrap_or(-1);
    let id = egui::Id::new(("tintable", Arc::as_ptr(game) as usize, appearance));
    if let Some(m) = ui.data(|d| d.get_temp::<Arc<Vec<String>>>(id)) {
        return m;
    }
    let m = Arc::new(mg_preview::tintable_materials(game, utp));
    ui.data_mut(|d| d.insert_temp(id, m.clone()));
    m
}

/// Sets a string local (or takes it out: `None`) on each object edited,
/// each keeping its other variables.
fn set_local(f: &mut Form<'_>, what: &str, name: &str, value: Option<&str>) {
    let targets: Vec<(ResKey, GffPath)> = f.targets().map(|(k, p)| (k, p.clone())).collect();
    let mut edits = Vec::new();
    for (key, path) in targets {
        let Some(Ok(doc)) = f.app.ws.as_mut().map(|ws| ws.doc(&key)) else { continue };
        let Some(s) = path.get(&doc.root) else { continue };
        let list = s.list("VarTable").unwrap_or(&[]);
        let changed = with_local(list, name, value);
        if changed != list {
            let value = Some(Value::List(changed));
            edits.push(Edit::SetField { key, path, label: "VarTable".into(), value });
        }
    }
    if !edits.is_empty() {
        f.app.actions.push(Action::Apply(Command::new(what, edits)));
    }
}

/// A VarTable with the string local `name` (any case) set to `value`, or
/// taken out; the others as they were.
fn with_local(list: &[Struct], name: &str, value: Option<&str>) -> Vec<Struct> {
    let mut out = list.to_vec();
    let named =
        |v: &Struct| v.string("Name").is_some_and(|n| n.eq_ignore_ascii_case(name.as_bytes()));
    match (out.iter().position(named), value) {
        (Some(i), Some(v)) => {
            out[i].set("Type", Value::Dword(3));
            out[i].set("Value", Value::String(v.as_bytes().to_vec()));
        }
        (None, Some(v)) => {
            let mut s = Struct::new(0);
            s.set("Name", Value::String(name.as_bytes().to_vec()));
            s.set("Type", Value::Dword(3));
            s.set("Value", Value::String(v.as_bytes().to_vec()));
            out.push(s);
        }
        (Some(i), None) => {
            out.remove(i);
        }
        (None, None) => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str, value: &str) -> Struct {
        with_local(&[], name, Some(value)).remove(0)
    }

    #[test]
    fn a_tint_s_local_is_set_replaced_and_taken_out_the_others_kept() {
        let other = var("x_note", "keep");
        let list = with_local(std::slice::from_ref(&other), "tdod_tint", Some("#2a4d8f"));
        assert_eq!(list, [other.clone(), var("tdod_tint", "#2a4d8f")]);
        let list = with_local(&list, "TDOD_TINT", Some("#ffffff"));
        assert_eq!(list, [other.clone(), var("tdod_tint", "#ffffff")], "replaced in place");
        assert_eq!(with_local(&list, "tdod_tint", None), std::slice::from_ref(&other));
        assert_eq!(with_local(&list, "tdod_tint_rug", None), list, "none to take out");
    }
}
