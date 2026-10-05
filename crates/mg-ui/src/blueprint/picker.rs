//! Choosing blueprints from a palette (the encounter's creatures, a store's
//! items), and what the editors need to know about the chosen ones.

use std::collections::HashMap;

use egui::Ui;
use mg_core::ResRef;
use mg_gff::{Gff, Struct};
use mg_module::palette::{BlueprintKind, PaletteNode};
use mg_resman::ResKey;

use super::Form;
use crate::palette_view;

impl Form<'_> {
    /// A blueprint's fields: the module's, else the game's.
    pub(crate) fn blueprint(&mut self, kind: BlueprintKind, resref: ResRef) -> Option<Struct> {
        blueprint(self.app, kind, resref)
    }

    /// The names of a type's blueprints, from the standard and the custom
    /// palette.
    pub(crate) fn blueprint_names(&mut self, kind: BlueprintKind) -> HashMap<ResRef, String> {
        fn walk(n: &PaletteNode, game: &mg_rules::GameData, out: &mut HashMap<ResRef, String>) {
            for b in &n.blueprints {
                out.insert(b.resref, b.name.text(game));
            }
            for c in &n.children {
                walk(c, game, out);
            }
        }
        let mut out = HashMap::new();
        let palettes = [false, true].map(|custom| palette_view::palette(self.app, kind, custom));
        let Some(game) = self.app.game.as_deref() else { return out };
        for p in palettes.iter().flatten() {
            for n in &p.nodes {
                walk(n, game, &mut out);
            }
        }
        out
    }

    /// The blueprint chosen (clicked) in a type's palette picker.
    pub(crate) fn palette_chosen(&self, ui: &Ui, kind: BlueprintKind) -> Option<ResRef> {
        let state_id = self.id(&format!("picker-{}", kind.name()));
        ui.data(|d| d.get_temp::<(bool, String, Option<ResRef>)>(state_id)).and_then(|s| s.2)
    }

    /// A palette to choose blueprints from: Standard or Custom, a filter and
    /// the tree; a click chooses, a double-click or `add` takes it. The
    /// blueprint taken.
    pub(crate) fn palette_picker(
        &mut self,
        ui: &mut Ui,
        kind: BlueprintKind,
        add: &str,
    ) -> Option<ResRef> {
        let state_id = self.id(&format!("picker-{}", kind.name()));
        palette_picker(self.app, ui, kind, add, state_id, None)
    }

    /// [`palette_picker`](Self::palette_picker) with `below` drawn under
    /// it and kept in sight: the tree ends above it, however much of it is
    /// opened out (scrolling inside), rather than push it out of the pane.
    pub(crate) fn palette_picker_over(
        &mut self,
        ui: &mut Ui,
        kind: BlueprintKind,
        add: &str,
        below: impl FnOnce(&mut Self, &mut Ui),
    ) -> Option<ResRef> {
        let state_id = self.id(&format!("picker-{}", kind.name()));
        // (As tall as it was drawn last.)
        let room_id = state_id.with("below");
        let room: f32 = ui.data(|d| d.get_temp(room_id)).unwrap_or(0.0);
        let button = ui.spacing().interact_size.y + 2.0 * ui.spacing().item_spacing.y;
        // (Under Standard and Custom, and Find.)
        let top = ui.cursor().top() + 2.0 * button;
        let height = (ui.clip_rect().bottom() - top - button - room).max(120.0);
        let taken = palette_picker(self.app, ui, kind, add, state_id, Some(height));
        let drawn = ui.scope(|ui| below(self, ui)).response.rect.height();
        let room = if drawn > 0.0 { drawn + ui.spacing().item_spacing.y } else { 0.0 };
        ui.data_mut(|d| d.insert_temp(room_id, room));
        taken
    }
}

/// A palette to choose blueprints from (see [`Form::palette_picker`]), its
/// state kept under `state_id`; its tree `height` points tall, or (`None`)
/// down to the bottom of what shows, its button below.
pub(crate) fn palette_picker(
    app: &mut crate::Moonglow,
    ui: &mut Ui,
    kind: BlueprintKind,
    add: &str,
    state_id: egui::Id,
    height: Option<f32>,
) -> Option<ResRef> {
    // (custom palette, filter, chosen blueprint)
    let (mut custom, mut filter, mut chosen): (bool, String, Option<ResRef>) =
        ui.data(|d| d.get_temp(state_id)).unwrap_or_default();
    let mut taken = None;
    ui.horizontal(|ui| {
        ui.selectable_value(&mut custom, false, "Standard Palette");
        ui.selectable_value(&mut custom, true, "Custom Palette");
    });
    ui.add(egui::TextEdit::singleline(&mut filter).hint_text("Find"));
    let palette = palette_view::palette(app, kind, custom);
    let game = app.game.as_deref();
    let button = ui.spacing().interact_size.y + 2.0 * ui.spacing().item_spacing.y;
    let height =
        height.unwrap_or_else(|| (ui.clip_rect().bottom() - ui.cursor().top() - button).max(200.0));
    let list = egui::ScrollArea::vertical().id_salt(state_id.with("tree")).max_height(height);
    list.auto_shrink([false, true]).show(ui, |ui| {
        if let (Some(p), Some(game)) = (palette, game) {
            let filter = filter.to_lowercase();
            for (i, node) in p.nodes.iter().enumerate() {
                tree(ui, game, node, &filter, &mut chosen, &mut taken, state_id, &[i]);
            }
        }
    });
    if ui.add_enabled(chosen.is_some(), egui::Button::new(add)).clicked() {
        taken = chosen;
    }
    ui.data_mut(|d| d.insert_temp(state_id, (custom, filter, chosen)));
    taken
}

/// A blueprint's fields: the module's, else the game's.
pub(crate) fn blueprint(
    app: &mut crate::Moonglow,
    kind: BlueprintKind,
    resref: ResRef,
) -> Option<Struct> {
    let key = ResKey::new(resref, kind.restype());
    if let Some(root) = app.ws.as_mut().and_then(|ws| ws.doc(&key).ok().map(|g| g.root.clone())) {
        return Some(root);
    }
    let data = app.game.as_deref()?.resman.get(&key).ok()?;
    Gff::read(&data).ok().map(|g| g.root)
}

/// A palette branch.
#[allow(clippy::too_many_arguments)]
fn tree(
    ui: &mut Ui,
    game: &mg_rules::GameData,
    node: &PaletteNode,
    filter: &str,
    chosen: &mut Option<ResRef>,
    taken: &mut Option<ResRef>,
    salt: egui::Id,
    path: &[usize],
) {
    fn any(n: &PaletteNode, game: &mg_rules::GameData, filter: &str) -> bool {
        n.blueprints.iter().any(|b| {
            b.name.text(game).to_lowercase().contains(filter)
                || b.resref.to_string().contains(filter)
        }) || n.children.iter().any(|c| any(c, game, filter))
    }
    if !filter.is_empty() && !any(node, game, filter) {
        return;
    }
    // While filtering, every branch with a match is open.
    egui::CollapsingHeader::new(node.name.text(game))
        .id_salt((salt, path))
        .open((!filter.is_empty()).then_some(true))
        .show(ui, |ui| {
            for (i, child) in node.children.iter().enumerate() {
                let mut p = path.to_vec();
                p.push(i);
                tree(ui, game, child, filter, chosen, taken, salt, &p);
            }
            for b in &node.blueprints {
                let name = b.name.text(game);
                if !filter.is_empty()
                    && !name.to_lowercase().contains(filter)
                    && !b.resref.to_string().contains(filter)
                {
                    continue;
                }
                let label = match b.cr {
                    Some(cr) => format!("{name}  (CR {cr})"),
                    None => name,
                };
                let r = ui
                    .selectable_label(*chosen == Some(b.resref), label)
                    .on_hover_text(b.resref.to_string());
                if r.clicked() {
                    *chosen = Some(b.resref);
                }
                if r.double_clicked() {
                    *taken = Some(b.resref);
                }
            }
        });
}
