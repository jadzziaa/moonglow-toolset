//! The blueprint palettes, as Aurora's right-hand pane: a blueprint type,
//! Standard (the game's) or Custom (the module's), and the categories with
//! their blueprints. Edit opens a custom blueprint; Edit Copy copies any
//! blueprint into the module as a new custom one; Delete removes a custom
//! one; Preview shows it in the 3D viewer.

use std::collections::HashMap;

use mg_core::ResRef;
use mg_edit::{Command, Edit};
use mg_gff::{Gff, Value};
use mg_module::palette::{
    BlueprintKind, Palette, PaletteBlueprint, PaletteNode, rebuild_custom_palette,
};
use mg_resman::ResKey;

use crate::{Action, Moonglow, Tab};

/// The palette pane's state.
#[derive(Debug)]
pub struct PaletteView {
    pub kind: BlueprintKind,
    /// Custom (module) blueprints rather than the game's.
    pub custom: bool,
    /// Shows only blueprints whose name or resref contains this.
    pub filter: String,
    standard: HashMap<BlueprintKind, Palette>,
    /// The custom palette built for a workspace revision.
    custom_cache: Option<(u64, BlueprintKind, Palette)>,
    pub selected: Option<ResKey>,
}

impl Default for PaletteView {
    fn default() -> PaletteView {
        PaletteView {
            kind: BlueprintKind::Creature,
            custom: false,
            filter: String::new(),
            standard: HashMap::new(),
            custom_cache: None,
            selected: None,
        }
    }
}

/// A resref for a copy of `from`: the resref without its trailing digits
/// and the first free three-digit number, within 16 characters.
pub fn copy_resref(from: &str, taken: &dyn Fn(&str) -> bool) -> Option<ResRef> {
    let stem = from.trim_end_matches(|c: char| c.is_ascii_digit());
    let stem = &stem[..stem.len().min(13)];
    (1..1000)
        .map(|n| format!("{stem}{n:03}"))
        .find(|name| !taken(name))
        .and_then(|name| ResRef::from_str(&name).ok())
}

/// Copies a blueprint into the module under a new resref (its
/// `TemplateResRef` too); the new key.
fn edit_copy(app: &mut Moonglow, key: ResKey) -> Option<ResKey> {
    let game = app.game.as_ref()?;
    let ws = app.ws.as_mut()?;
    let data = ws
        .module
        .get(&key)
        .map(<[u8]>::to_vec)
        .or_else(|| game.resman.get(&key).ok().map(|d| d.into_owned()))?;
    let mut gff = Gff::read(&data).ok()?;
    let taken = |name: &str| {
        let k = ResKey::parse(name, key.restype);
        k.is_some_and(|k| ws.module.contains(&k) || game.resman.get(&k).is_ok())
    };
    let resref = copy_resref(&key.resref.to_string(), &taken)?;
    gff.root.set("TemplateResRef", Value::resref(resref));
    let new = ResKey::new(resref, key.restype);
    let cmd = Command::new(
        format!("Edit copy of {key}"),
        vec![Edit::SetResource { key: new, data: Some(gff.to_bytes().ok()?) }],
    );
    match ws.apply(cmd) {
        Ok(()) => Some(new),
        Err(e) => {
            app.log.error(e.to_string());
            None
        }
    }
}

enum Pick {
    Edit(ResKey),
    EditCopy(ResKey),
    Delete(ResKey),
    Preview(ResKey),
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui) {
    let Some(game) = app.game.as_ref() else {
        ui.label("No game data.");
        return;
    };
    let mut view = std::mem::take(&mut app.palette);
    ui.horizontal_wrapped(|ui| {
        for kind in BlueprintKind::ALL {
            ui.selectable_value(&mut view.kind, kind, kind.label());
        }
    });
    ui.horizontal(|ui| {
        ui.selectable_value(&mut view.custom, false, "Standard");
        ui.add_enabled_ui(app.ws.is_some(), |ui| {
            ui.selectable_value(&mut view.custom, true, "Custom");
        });
        ui.separator();
        ui.add(egui::TextEdit::singleline(&mut view.filter).hint_text("Find").desired_width(140.0));
    });
    ui.separator();

    // The palette to show.
    let kind = view.kind;
    let palette: Option<&Palette> = if view.custom {
        match &mut app.ws {
            Some(ws) => {
                let rev = ws.revision();
                if view.custom_cache.as_ref().is_none_or(|(r, k, _)| *r != rev || *k != kind) {
                    let built = ws
                        .flush()
                        .ok()
                        .and_then(|()| rebuild_custom_palette(&ws.module, game, kind).ok())
                        .map(|g| Palette::read(&g))
                        .unwrap_or_default();
                    view.custom_cache = Some((rev, kind, built));
                }
                view.custom_cache.as_ref().map(|(_, _, p)| p)
            }
            None => None,
        }
    } else {
        Some(&*view.standard.entry(kind).or_insert_with(|| {
            game.resman
                .get_named(&format!("{}palstd", kind.name()), mg_core::ResType::ITP)
                .ok()
                .and_then(|d| Gff::read(&d).ok())
                .map(|g| Palette::read(&g))
                .unwrap_or_default()
        }))
    };
    let Some(palette) = palette else {
        ui.label("No module open.");
        app.palette = view;
        return;
    };

    let filter = view.filter.to_lowercase();
    let custom = view.custom;
    let mut picks = Vec::new();
    let mut selected = view.selected;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for (i, node) in palette.nodes.iter().enumerate() {
            show_node(ui, game, node, &filter, kind, custom, &mut selected, &mut picks, &[i]);
        }
    });
    view.selected = selected;
    app.palette = view;

    for pick in picks {
        match pick {
            Pick::Edit(key) => {
                if let Some(t) = Tab::for_resource(key) {
                    app.actions.push(Action::OpenTab(t));
                }
            }
            Pick::EditCopy(key) => {
                if let Some(new) = edit_copy(app, key) {
                    app.palette.custom = true;
                    app.palette.selected = Some(new);
                    if let Some(t) = Tab::for_resource(new) {
                        app.actions.push(Action::OpenTab(t));
                    }
                }
            }
            Pick::Delete(key) => {
                if let Some(ws) = &mut app.ws {
                    let cmd = Command::new(
                        format!("Delete {key}"),
                        vec![Edit::SetResource { key, data: None }],
                    );
                    if let Err(e) = ws.apply(cmd) {
                        app.log.error(e.to_string());
                    }
                }
            }
            Pick::Preview(key) => app.actions.push(Action::OpenTab(Tab::Model(key))),
        }
    }
}

/// Whether a blueprint passes the filter.
fn shown(b: &PaletteBlueprint, name: &str, filter: &str) -> bool {
    filter.is_empty()
        || name.to_lowercase().contains(filter)
        || b.resref.to_string().contains(filter)
}

/// Whether a node or anything under it passes the filter.
fn any_shown(game: &mg_rules::GameData, node: &PaletteNode, filter: &str) -> bool {
    filter.is_empty()
        || node.blueprints.iter().any(|b| shown(b, &b.name.text(game), filter))
        || node.children.iter().any(|c| any_shown(game, c, filter))
}

#[allow(clippy::too_many_arguments)]
fn show_node(
    ui: &mut egui::Ui,
    game: &mg_rules::GameData,
    node: &PaletteNode,
    filter: &str,
    kind: BlueprintKind,
    custom: bool,
    selected: &mut Option<ResKey>,
    picks: &mut Vec<Pick>,
    path: &[usize],
) {
    if !any_shown(game, node, filter) {
        return;
    }
    let count = node.blueprints.len();
    let title = if count > 0 {
        format!("{} ({count})", node.name.text(game))
    } else {
        node.name.text(game)
    };
    egui::CollapsingHeader::new(title)
        .id_salt(("palette", kind, custom, path))
        .default_open(!filter.is_empty())
        .show(ui, |ui| {
            for (i, child) in node.children.iter().enumerate() {
                let mut p = path.to_vec();
                p.push(i);
                show_node(ui, game, child, filter, kind, custom, selected, picks, &p);
            }
            for b in &node.blueprints {
                let name = b.name.text(game);
                if !shown(b, &name, filter) {
                    continue;
                }
                let key = ResKey::new(b.resref, kind.restype());
                let label = match b.cr {
                    Some(cr) => format!("{name}  (CR {cr})"),
                    None => name,
                };
                let r = ui
                    .selectable_label(*selected == Some(key), label)
                    .on_hover_text(b.resref.to_string());
                if r.clicked() {
                    *selected = Some(key);
                }
                if r.double_clicked() {
                    picks.push(if custom { Pick::Edit(key) } else { Pick::Preview(key) });
                }
                r.context_menu(|ui| {
                    if custom && ui.button("Edit").clicked() {
                        picks.push(Pick::Edit(key));
                        ui.close();
                    }
                    if ui
                        .button("Edit Copy")
                        .on_hover_text("Copy into the module's custom palette")
                        .clicked()
                    {
                        picks.push(Pick::EditCopy(key));
                        ui.close();
                    }
                    if crate::model_view::previewable(key.restype) && ui.button("Preview").clicked()
                    {
                        picks.push(Pick::Preview(key));
                        ui.close();
                    }
                    if custom && ui.button("Delete").clicked() {
                        picks.push(Pick::Delete(key));
                        ui.close();
                    }
                });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_take_the_next_free_number() {
        let taken = |n: &str| ["nw_chicken001", "nw_chicken002"].contains(&n);
        assert_eq!(copy_resref("nw_chicken", &taken).unwrap().to_string(), "nw_chicken003");
        assert_eq!(copy_resref("nw_chicken001", &taken).unwrap().to_string(), "nw_chicken003");
        // At most 16 characters.
        let long = copy_resref("abcdefghijklmnop", &|_| false).unwrap().to_string();
        assert_eq!(long, "abcdefghijklm001");
    }
}
