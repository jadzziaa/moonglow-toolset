//! Choosing blueprints from a palette (the encounter's creatures, a store's
//! items), and what the editors need to know about the chosen ones.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use egui::Ui;
use mg_core::ResRef;
use mg_gff::{Gff, Struct};
use mg_module::palette::{BlueprintKind, Palette, PaletteNode};
use mg_resman::ResKey;

use super::Form;
use crate::palette_view;

impl Form<'_> {
    /// A blueprint's fields: the module's, else the game's.
    pub(crate) fn blueprint(&mut self, kind: BlueprintKind, resref: ResRef) -> Option<Struct> {
        blueprint(self.app, kind, resref)
    }

    /// A blueprint's fields, as [`blueprint`](Self::blueprint) reads
    /// them, kept until the module changes ([`Kept`]).
    pub(crate) fn kept_blueprint(
        &mut self,
        kind: BlueprintKind,
        resref: ResRef,
    ) -> Option<Arc<Struct>> {
        let key = ResKey::new(resref, kind.restype());
        let revision = self.app.ws.as_ref().map(mg_edit::Workspace::revision);
        let kept = &mut self.app.palette.kept;
        if kept.revision != revision {
            kept.revision = revision;
            kept.blueprints.clear();
        }
        if let Some(b) = kept.blueprints.get(&key) {
            return b.clone();
        }
        let b = blueprint(self.app, kind, resref).map(Arc::new);
        self.app.palette.kept.blueprints.insert(key, b.clone());
        b
    }

    /// The names of a type's blueprints, from the standard and the custom
    /// palette; kept while the palettes are the same.
    pub(crate) fn blueprint_names(&mut self, kind: BlueprintKind) -> Arc<HashMap<ResRef, String>> {
        fn walk(n: &PaletteNode, game: &mg_rules::GameData, out: &mut HashMap<ResRef, String>) {
            for b in &n.blueprints {
                out.insert(b.resref, b.name.text(game));
            }
            for c in &n.children {
                walk(c, game, out);
            }
        }
        let palettes = [false, true].map(|custom| palette_view::palette(self.app, kind, custom));
        let same = |a: &Option<Arc<Palette>>, b: &Option<Arc<Palette>>| match (a, b) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (a, b) => a.is_none() && b.is_none(),
        };
        if let Some((from, names)) = self.app.palette.kept.names.get(&kind)
            && same(&from[0], &palettes[0])
            && same(&from[1], &palettes[1])
        {
            return names.clone();
        }
        let mut out = HashMap::new();
        let Some(game) = self.app.game.as_deref() else { return Arc::new(out) };
        for p in palettes.iter().flatten() {
            for n in &p.nodes {
                walk(n, game, &mut out);
            }
        }
        let out = Arc::new(out);
        self.app.palette.kept.names.insert(kind, (palettes, out.clone()));
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
    crate::widgets::SearchField::new(state_id.with("find"), &mut filter).hint("Find").show(ui);
    let palette = palette_view::palette(app, kind, custom);
    let game = app.game.as_deref();
    let button = ui.spacing().interact_size.y + 2.0 * ui.spacing().item_spacing.y;
    let height =
        height.unwrap_or_else(|| (ui.clip_rect().bottom() - ui.cursor().top() - button).max(200.0));
    // What the search finds, kept while the palette and the search are the
    // same: it was worked out again a level of the tree at a time, every
    // frame (8 ms over 8,000 blueprints).
    let searched = !mg_core::text::Needle::new(&filter).is_empty();
    let found = match (&palette, game) {
        (Some(p), Some(game)) if searched => {
            let kept = app.palette.kept.found.get(&(kind, custom));
            Some(match kept {
                Some((from, search, f)) if Arc::ptr_eq(from, p) && *search == filter => f.clone(),
                _ => {
                    let f = Arc::new(Found::new(p, game, &filter));
                    let keep = (p.clone(), filter.clone(), f.clone());
                    app.palette.kept.found.insert((kind, custom), keep);
                    f
                }
            })
        }
        _ => None,
    };
    let game = app.game.as_deref();
    let list = egui::ScrollArea::vertical().id_salt(state_id.with("tree")).max_height(height);
    list.auto_shrink([false, true]).show(ui, |ui| {
        if let (Some(p), Some(game)) = (palette, game) {
            let mut at =
                Tree { game, found: found.as_deref(), chosen: &mut chosen, taken: &mut taken };
            for (i, node) in p.nodes.iter().enumerate() {
                at.branch(ui, node, state_id, &[i], (kind, add));
            }
        }
    });
    if ui.add_enabled(chosen.is_some(), egui::Button::new(add)).clicked() {
        taken = chosen;
    }
    ui.data_mut(|d| d.insert_temp(state_id, (custom, filter, chosen)));
    taken
}

/// What the editors read of blueprints every frame, kept: blueprints'
/// fields until the module changes (its revision), their names while the
/// palettes they come from are the same; all of it until the game data
/// changes (`PaletteView::forget_game_data`).
#[derive(Debug, Default)]
pub(crate) struct Kept {
    /// The workspace revision `blueprints` were read at.
    revision: Option<u64>,
    blueprints: HashMap<ResKey, Option<Arc<Struct>>>,
    /// By kind: the palettes the names were read from (standard,
    /// custom), and the names.
    #[allow(clippy::type_complexity)]
    names: HashMap<BlueprintKind, ([Option<Arc<Palette>>; 2], Arc<HashMap<ResRef, String>>)>,
    /// By kind and palette (custom or not): what a picker's search found,
    /// with the palette and the search it was found for.
    #[allow(clippy::type_complexity)]
    found: HashMap<(BlueprintKind, bool), (Arc<Palette>, String, Arc<Found>)>,
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

/// Set by a page that equips items (a creature's inventory) before its
/// picker is drawn: the rows' menu then offers Equip.
pub(crate) fn can_equip_id() -> egui::Id {
    egui::Id::new("item-picker-can-equip")
}

/// The item a picker row's Equip was chosen for, for the page to take.
pub(crate) fn equip_asked_id() -> egui::Id {
    egui::Id::new("item-picker-equip-asked")
}

/// What a search finds in a palette: the blueprints, and the branches
/// that hold one (by their places in the tree).
#[derive(Debug, Default)]
pub(crate) struct Found {
    blueprints: HashSet<ResRef>,
    branches: HashSet<Vec<usize>>,
}

impl Found {
    /// Every word of `typed` in a blueprint's name or its ResRef, as the
    /// palettes find them.
    fn new(p: &Palette, game: &mg_rules::GameData, typed: &str) -> Found {
        fn walk(
            n: &PaletteNode,
            path: &mut Vec<usize>,
            finds: &dyn Fn(&str, ResRef) -> bool,
            game: &mg_rules::GameData,
            out: &mut Found,
        ) -> bool {
            let mut any = false;
            for b in &n.blueprints {
                if finds(&b.name.text(game), b.resref) {
                    out.blueprints.insert(b.resref);
                    any = true;
                }
            }
            for (i, c) in n.children.iter().enumerate() {
                path.push(i);
                any |= walk(c, path, finds, game, out);
                path.pop();
            }
            if any {
                out.branches.insert(path.clone());
            }
            any
        }
        let filter = mg_core::text::Needle::new(typed);
        let finds = |name: &str, resref: ResRef| {
            filter.finds_in(&[&mg_core::text::fold(name), &resref.to_string().to_ascii_lowercase()])
        };
        let mut out = Found::default();
        for (i, n) in p.nodes.iter().enumerate() {
            walk(n, &mut vec![i], &finds, game, &mut out);
        }
        out
    }
}

/// A palette's tree as a picker draws it: what the search found (none:
/// every blueprint), the blueprint chosen and the one taken.
struct Tree<'a> {
    game: &'a mg_rules::GameData,
    found: Option<&'a Found>,
    chosen: &'a mut Option<ResRef>,
    taken: &'a mut Option<ResRef>,
}

impl Tree<'_> {
    /// A palette branch, at `path` in the tree.
    fn branch(
        &mut self,
        ui: &mut Ui,
        node: &PaletteNode,
        salt: egui::Id,
        path: &[usize],
        (kind, add): (BlueprintKind, &str),
    ) {
        if self.found.is_some_and(|f| !f.branches.contains(path)) {
            return;
        }
        // While searching, every branch with a match is open.
        egui::CollapsingHeader::new(node.name.text(self.game))
            .id_salt((salt, path))
            .open(self.found.is_some().then_some(true))
            .show(ui, |ui| {
                for (i, child) in node.children.iter().enumerate() {
                    let mut p = path.to_vec();
                    p.push(i);
                    self.branch(ui, child, salt, &p, (kind, add));
                }
                for b in &node.blueprints {
                    if self.found.is_some_and(|f| !f.blueprints.contains(&b.resref)) {
                        continue;
                    }
                    self.row(ui, b, (kind, add));
                }
            });
    }

    /// A blueprint's row: out of sight, its room only (a branch may hold
    /// thousands).
    fn row(
        &mut self,
        ui: &mut Ui,
        b: &mg_module::palette::PaletteBlueprint,
        (kind, add): (BlueprintKind, &str),
    ) {
        let height = ui.spacing().interact_size.y;
        let room = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(1.0, height));
        if !ui.is_rect_visible(room) {
            ui.allocate_space(egui::vec2(1.0, height));
            return;
        }
        let name = b.name.text(self.game);
        let label = match b.cr {
            Some(cr) => format!("{name}  (CR {cr})"),
            None => name,
        };
        let row = egui::Button::selectable(*self.chosen == Some(b.resref), label)
            .sense(egui::Sense::click_and_drag());
        let r = ui.add(row).on_hover_text(b.resref.to_string());
        if r.clicked() {
            *self.chosen = Some(b.resref);
        }
        if r.double_clicked() {
            *self.taken = Some(b.resref);
        }
        // An item dragged: onto a list of items to add it, onto a
        // creature's slot to equip it.
        if kind == BlueprintKind::Item && r.drag_started() {
            r.dnd_set_drag_payload(super::inventory::Dragged::Palette(b.resref));
        }
        r.context_menu(|ui| {
            if ui.button(add).clicked() {
                *self.taken = Some(b.resref);
                ui.close();
            }
            // (Where the page equips: a creature's inventory.)
            if ui.data(|d| d.get_temp::<bool>(can_equip_id())).unwrap_or(false)
                && ui.button("Equip").clicked()
            {
                ui.data_mut(|d| d.insert_temp(equip_asked_id(), b.resref));
                ui.close();
            }
        });
    }
}
