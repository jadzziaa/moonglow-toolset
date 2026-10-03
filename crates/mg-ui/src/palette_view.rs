//! The blueprint palettes, as Aurora's right-hand pane: a blueprint type,
//! Standard (the game's) or Custom (the module's), and the categories with
//! their blueprints. Edit opens a custom blueprint; Edit Copy copies any
//! blueprint into the module as a new custom one; Delete removes a custom
//! one; Preview shows it in the 3D viewer.

use std::collections::HashMap;
use std::sync::Arc;

use mg_core::ResRef;
use mg_edit::{Command, Edit};
use mg_gff::{Gff, Value};
use mg_module::palette::{
    BlueprintKind, Palette, PaletteBlueprint, PaletteNode, rebuild_custom_palette,
};
use mg_resman::ResKey;

use crate::{Action, Moonglow, Tab};

/// A blueprint dragged from the palette (dropped in an area view, it is
/// placed there).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Dragged(pub(crate) ResKey);

/// Blueprints' tags, by resref.
type Tags = Arc<HashMap<ResRef, String>>;

/// The palette pane's state.
#[derive(Debug)]
pub struct PaletteView {
    pub kind: BlueprintKind,
    /// Custom (module) blueprints rather than the game's.
    pub custom: bool,
    /// Shows only blueprints whose name or resref contains this.
    pub filter: String,
    standard: HashMap<BlueprintKind, Arc<Palette>>,
    /// Custom palettes, each built for a workspace revision.
    custom_cache: HashMap<BlueprintKind, (u64, Arc<Palette>)>,
    pub selected: Option<ResKey>,
    /// The hovered blueprint's thumbnail, once rendered.
    thumb: Option<(ResKey, Option<egui::TextureId>)>,
    /// The last search's finds: (palette, search, revision), blueprints,
    /// and how found.
    #[allow(clippy::type_complexity)]
    found: Option<(
        (BlueprintKind, bool, String, Option<u64>),
        Arc<std::collections::HashSet<ResRef>>,
        Find,
    )>,
    /// Blueprints' tags, for Find: by palette (and module revision).
    tags: HashMap<(BlueprintKind, bool), (Option<u64>, Tags)>,
    /// More custom blueprints chosen with Ctrl+click, for bulk edits (with
    /// `selected`).
    pub chosen: Vec<ResKey>,
    /// The tileset palette is shown, rather than blueprints.
    pub tiles: bool,
    /// The tileset brush chosen.
    pub tile_brush: Option<crate::terrain_mode::TileBrush>,
    /// The area shown last: its tileset's palette is the one shown.
    pub area: Option<ResRef>,
    pub(crate) tile_palettes: HashMap<ResRef, Arc<mg_area::terrain::TilesetPalette>>,
    /// Tilesets' names as Area Properties shows them, read once each (a
    /// big tileset takes milliseconds to read: too long for every frame).
    pub(crate) tileset_names: HashMap<ResRef, String>,
}

impl PaletteView {
    /// Forgets palettes built from the game data (after Reload Resources).
    pub(crate) fn forget_game_data(&mut self) {
        self.standard.clear();
        self.custom_cache.clear();
        self.tile_palettes.clear();
        self.tileset_names.clear();
    }
}

impl Default for PaletteView {
    fn default() -> PaletteView {
        PaletteView {
            kind: BlueprintKind::Creature,
            custom: false,
            filter: String::new(),
            standard: HashMap::new(),
            custom_cache: HashMap::new(),
            selected: None,
            thumb: None,
            found: None,
            tags: HashMap::new(),
            chosen: Vec::new(),
            tiles: false,
            tile_brush: None,
            area: None,
            tile_palettes: HashMap::new(),
            tileset_names: HashMap::new(),
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
/// `TemplateResRef`, a store's `ResRef`, too); the new key.
fn edit_copy(app: &mut Moonglow, key: ResKey) -> Option<ResKey> {
    let game = app.game.as_deref()?;
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
    let field =
        BlueprintKind::from_restype(key.restype).map_or("TemplateResRef", |k| k.resref_field());
    gff.root.set(field, Value::resref(resref));
    let new = ResKey::new(resref, key.restype);
    let cmd = Command::new(
        format!("Edit copy of {key}"),
        vec![Edit::SetResource { key: new, data: Some(gff.to_bytes().ok()?) }],
    );
    match app.apply(cmd) {
        Ok(()) => Some(new),
        Err(e) => {
            app.log.error(e.to_string());
            None
        }
    }
}

/// A palette: the game's standard one, or the module's custom one (built
/// from its blueprints, again after each change); cached.
pub(crate) fn palette(
    app: &mut Moonglow,
    kind: BlueprintKind,
    custom: bool,
) -> Option<Arc<Palette>> {
    let game = app.game.as_deref()?;
    let view = &mut app.palette;
    if !custom {
        let p = view.standard.entry(kind).or_insert_with(|| {
            Arc::new(
                game.resman
                    .get_named(&format!("{}palstd", kind.name()), mg_core::ResType::ITP)
                    .ok()
                    .and_then(|d| Gff::read(&d).ok())
                    .map(|g| Palette::read(&g))
                    .unwrap_or_default(),
            )
        });
        return Some(p.clone());
    }
    let ws = app.ws.as_mut()?;
    let rev = ws.revision();
    if view.custom_cache.get(&kind).is_none_or(|(r, _)| *r != rev) {
        let built = ws
            .flush()
            .ok()
            .and_then(|()| rebuild_custom_palette(&ws.module, game, kind).ok())
            .map(|g| Palette::read(&g))
            .unwrap_or_default();
        view.custom_cache.insert(kind, (rev, Arc::new(built)));
    }
    view.custom_cache.get(&kind).map(|(_, p)| p.clone())
}

/// What a search finds in a palette, kept until the search, the palette
/// or the module changes: the blueprints, and how (by words, or by letters
/// in order when nothing has every word).
fn found(
    app: &mut Moonglow,
    palette: &Palette,
    kind: BlueprintKind,
    custom: bool,
    tags: &HashMap<ResRef, String>,
) -> (Arc<std::collections::HashSet<ResRef>>, Find) {
    let revision = app.ws.as_ref().map(|w| w.revision());
    let key = (kind, custom, app.palette.filter.clone(), revision);
    if let Some((k, f, find)) = &app.palette.found
        && *k == key
    {
        return (f.clone(), find.clone());
    }
    let Some(game) = app.game.as_deref() else { return Default::default() };
    let fields: Vec<(ResRef, [String; 3])> = palette
        .blueprints()
        .into_iter()
        .map(|(_, b)| {
            let tag = tags.get(&b.resref).map(|t| t.to_lowercase()).unwrap_or_default();
            (b.resref, [b.name.text(game).to_lowercase(), b.resref.to_string(), tag])
        })
        .collect();
    let mut find = Find::new(&app.palette.filter);
    let search = |find: &Find| -> std::collections::HashSet<ResRef> {
        fields.iter().filter(|(_, f)| find.hit(&[&f[0], &f[1], &f[2]])).map(|(r, _)| *r).collect()
    };
    let mut hits = search(&find);
    if hits.is_empty() {
        find.fuzzy = true;
        hits = search(&find);
    }
    let hits = Arc::new(hits);
    app.palette.found = Some((key, hits.clone(), find.clone()));
    (hits, find)
}

/// The tags of a palette's blueprints (read from each once; for a custom
/// palette, again after the module changes).
fn tags(
    app: &mut Moonglow,
    palette: &Palette,
    kind: BlueprintKind,
    custom: bool,
) -> Arc<HashMap<ResRef, String>> {
    let revision = if custom { app.ws.as_ref().map(|w| w.revision()) } else { None };
    if let Some((r, t)) = app.palette.tags.get(&(kind, custom))
        && *r == revision
    {
        return t.clone();
    }
    let (Some(game), ws) = (app.game.as_deref(), app.ws.as_ref()) else { return Arc::default() };
    let read = |r: ResRef| -> Option<String> {
        let k = ResKey::new(r, kind.restype());
        let data = ws
            .and_then(|w| w.module.get(&k).map(<[u8]>::to_vec))
            .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()))?;
        let g = Gff::read(&data).ok()?;
        Some(String::from_utf8_lossy(g.root.string("Tag")?).into_owned())
    };
    let tags: HashMap<ResRef, String> = palette
        .blueprints()
        .into_iter()
        .filter_map(|(_, b)| Some((b.resref, read(b.resref)?)))
        .collect();
    let tags = Arc::new(tags);
    app.palette.tags.insert((kind, custom), (revision, tags.clone()));
    tags
}

enum Pick {
    Edit(ResKey),
    EditCopy(ResKey),
    Delete(ResKey),
    Preview(ResKey),
    References(ResKey),
    UpdateInstances(Vec<ResKey>),
    EditTogether(Vec<ResKey>),
    /// Into a custom category (its palette id).
    MoveTo(ResKey, u8),
    /// Added to (true) or removed from Favorites.
    Favorite(ResKey, bool),
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui) {
    if app.game.is_none() {
        ui.label("No game data.");
        return;
    }
    let mut view = std::mem::take(&mut app.palette);
    ui.horizontal_wrapped(|ui| {
        use crate::icons::{TILES, blueprint, labelled};
        let tiles = labelled(TILES, "Tiles");
        if ui.selectable_label(view.tiles, tiles).on_hover_text("The area's tileset").clicked() {
            view.tiles = true;
        }
        for kind in BlueprintKind::ALL {
            let label = labelled(blueprint(kind), kind.label());
            if ui.selectable_label(!view.tiles && view.kind == kind, label).clicked() {
                view.kind = kind;
                view.tiles = false;
            }
        }
    });
    if view.tiles {
        ui.separator();
        app.palette = view;
        crate::terrain_mode::palette_ui(app, ui);
        return;
    }
    ui.horizontal(|ui| {
        ui.selectable_value(&mut view.custom, false, "Standard");
        ui.add_enabled_ui(app.ws.is_some(), |ui| {
            ui.selectable_value(&mut view.custom, true, "Custom");
        });
        ui.separator();
        ui.add(egui::TextEdit::singleline(&mut view.filter).hint_text("Find").desired_width(140.0));
        let creature = view.kind == BlueprintKind::Creature;
        let wizard = creature || crate::blueprint_wizard::KINDS.contains(&view.kind);
        if ui
            .add_enabled(app.ws.is_some() && wizard, egui::Button::new("New…"))
            .on_hover_text("A new blueprint (the type's wizard)")
            .clicked()
        {
            if creature {
                app.creature_wizard = Some(Default::default());
            } else {
                app.blueprint_wizard =
                    Some(crate::blueprint_wizard::BlueprintWizard::new(view.kind));
            }
        }
    });
    ui.separator();

    // The palette to show.
    let kind = view.kind;
    app.palette = view;
    let shown = palette(app, kind, app.palette.custom);
    let view = std::mem::take(&mut app.palette);
    let Some(palette) = shown else {
        ui.label("No module open.");
        app.palette = view;
        return;
    };
    let custom = view.custom;
    // Tags are read once a palette is searched (and shown on hover then).
    let searching = !view.filter.trim().is_empty();
    // (The caches are the palette state's: put back while they're used.)
    app.palette = view;
    let tags = if searching { tags(app, &palette, kind, custom) } else { Arc::default() };
    let (found, find) = if searching {
        let (f, find) = found(app, &palette, kind, custom, &tags);
        (Some(f), find)
    } else {
        (None, Find::default())
    };
    let mut view = std::mem::take(&mut app.palette);
    let game = app.game.as_deref().expect("checked");
    let favorites = app.settings.palette_favorites.clone();
    let recent = app.settings.palette_recent.clone();
    let ext = kind.restype().extension().unwrap_or_default();
    let mut tree = Tree {
        game,
        kind,
        custom,
        find: find.clone(),
        found: found.as_deref(),
        tags: &tags,
        favorites: favorites
            .iter()
            .filter_map(|f| ResRef::from_str(f.strip_prefix(&format!("{ext}:"))?).ok())
            .collect(),
        thumb: view.thumb,
        sel: Selection { selected: view.selected, chosen: std::mem::take(&mut view.chosen) },
        picks: Vec::new(),
        hovered: None,
    };
    if find.fuzzy {
        ui.weak("No exact matches: close ones");
    }
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        if find.is_empty() {
            tree.remembered(ui, "Favorites", &favorites, &palette);
            tree.remembered(ui, "Recent", &recent, &palette);
        }
        for (i, node) in palette.nodes.iter().enumerate() {
            tree.node(ui, node, &[i]);
        }
    });
    let (sel, picks, hovered) = (tree.sel, tree.picks, tree.hovered);
    if sel.selected.is_some() && sel.selected != view.selected {
        view.tile_brush = None;
    }
    view.selected = sel.selected;
    view.chosen = sel.chosen;
    app.palette = view;
    // The hovered blueprint's picture, for its tooltip next frame.
    if let Some(key) = hovered
        && app.palette.thumb.is_none_or(|(k, _)| k != key)
    {
        let id = crate::model_view::thumbnail(app, key);
        app.palette.thumb = Some((key, id));
        ui.ctx().request_repaint();
    }

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
                let cmd = Command::new(
                    format!("Delete {key}"),
                    vec![Edit::SetResource { key, data: None }],
                );
                if let Err(e) = app.apply(cmd) {
                    app.log.error(e.to_string());
                }
            }
            Pick::Preview(key) => app.actions.push(Action::OpenTab(Tab::Model(key))),
            Pick::References(key) => app.actions.push(Action::FindReferences(key)),
            Pick::UpdateInstances(keys) => app.update_instances_of(keys),
            Pick::EditTogether(keys) => app.actions.push(Action::OpenTab(Tab::Blueprints(keys))),
            Pick::MoveTo(key, id) => {
                let field = kind.palette_field();
                let current = app
                    .ws
                    .as_mut()
                    .and_then(|ws| ws.doc(&key).ok())
                    .and_then(|g| g.root.integer(field));
                if current.is_some_and(|c| c != i64::from(id)) {
                    app.actions.push(Action::Apply(Command::new(
                        format!("Move {} to another category", key.resref),
                        vec![Edit::SetField {
                            key,
                            path: mg_edit::GffPath::root(),
                            label: field.into(),
                            value: Some(Value::Byte(id)),
                        }],
                    )));
                }
            }
            Pick::Favorite(key, on) => {
                let list = &mut app.settings.palette_favorites;
                list.retain(|r| *r != remembered(key));
                if on {
                    list.push(remembered(key));
                }
            }
        }
    }
}

/// The blueprints picked in a palette: the one clicked, and more chosen
/// with Ctrl+click (custom palettes).
struct Selection {
    selected: Option<ResKey>,
    chosen: Vec<ResKey>,
}

impl Selection {
    fn has(&self, key: ResKey) -> bool {
        self.selected == Some(key) || self.chosen.contains(&key)
    }

    /// What a bulk edit on `key` applies to: the selection, if `key` is in
    /// it, else `key` alone.
    fn bulk(&self, key: ResKey) -> Vec<ResKey> {
        if !self.has(key) || self.chosen.is_empty() {
            return vec![key];
        }
        let mut keys: Vec<ResKey> = self.selected.into_iter().chain(self.chosen.clone()).collect();
        keys.sort();
        keys.dedup();
        keys
    }
}

/// Every blueprint under a palette node.
fn all_under(node: &PaletteNode, kind: BlueprintKind, out: &mut Vec<ResKey>) {
    out.extend(node.blueprints.iter().map(|b| ResKey::new(b.resref, kind.restype())));
    for c in &node.children {
        all_under(c, kind, out);
    }
}

/// A palette search: words, each found in a blueprint's name, resref or
/// tag; or, when nothing has every word, letters in order (`lngswd` finds
/// Longsword).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Find {
    words: Vec<String>,
    pub(crate) fuzzy: bool,
}

impl Find {
    fn new(filter: &str) -> Find {
        Find { words: filter.split_whitespace().map(str::to_lowercase).collect(), fuzzy: false }
    }

    fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Whether a blueprint with these (lower-case) fields matches.
    fn hit(&self, fields: &[&str]) -> bool {
        self.words.iter().all(|w| {
            fields.iter().any(|f| if self.fuzzy { in_order(f, w) } else { f.contains(w.as_str()) })
        })
    }
}

/// Whether `word`'s letters are in `text` in order.
fn in_order(text: &str, word: &str) -> bool {
    let mut t = text.chars();
    word.chars().all(|c| t.any(|x| x == c))
}

/// What the palette tree is drawn with.
struct Tree<'a> {
    game: &'a mg_rules::GameData,
    kind: BlueprintKind,
    custom: bool,
    find: Find,
    /// The blueprints the search finds (`None`: no search).
    found: Option<&'a std::collections::HashSet<ResRef>>,
    tags: &'a HashMap<ResRef, String>,
    /// This palette's favorites.
    favorites: std::collections::HashSet<ResRef>,
    /// The thumbnail ready for the blueprint hovered last frame.
    thumb: Option<(ResKey, Option<egui::TextureId>)>,
    sel: Selection,
    picks: Vec<Pick>,
    /// The blueprint hovered now.
    hovered: Option<ResKey>,
}

/// A blueprint as Favorites and Recent remember it: `utp:plc_chest1`.
pub(crate) fn remembered(key: ResKey) -> String {
    format!("{}:{}", key.restype.extension().unwrap_or_default(), key.resref)
}

impl Tree<'_> {
    fn shown(&self, b: &PaletteBlueprint) -> bool {
        self.found.is_none_or(|f| f.contains(&b.resref))
    }

    /// Whether a node or anything under it passes the search.
    fn any_shown(&self, node: &PaletteNode) -> bool {
        self.found.is_none()
            || node.blueprints.iter().any(|b| self.shown(b))
            || node.children.iter().any(|c| self.any_shown(c))
    }

    fn node(&mut self, ui: &mut egui::Ui, node: &PaletteNode, path: &[usize]) {
        if !self.any_shown(node) {
            return;
        }
        let (game, kind, custom) = (self.game, self.kind, self.custom);
        let count = node.blueprints.len();
        let title = if count > 0 {
            format!("{} ({count})", node.name.text(game))
        } else {
            node.name.text(game)
        };
        // Finding opens every category with a match, whatever was open before.
        let shown = egui::CollapsingHeader::new(title)
            .id_salt(("palette", kind, custom, path))
            .open((!self.find.is_empty()).then_some(true))
            .show(ui, |ui| {
                for (i, child) in node.children.iter().enumerate() {
                    let mut p = path.to_vec();
                    p.push(i);
                    self.node(ui, child, &p);
                }
                for b in &node.blueprints {
                    if self.shown(b) {
                        self.row(ui, b);
                    }
                }
            });
        if !custom {
            return;
        }
        // A custom blueprint dropped on a category moves into it.
        if let Some(id) = node.id
            && let Some(d) = shown.header_response.dnd_release_payload::<Dragged>()
            && d.0.restype == kind.restype()
        {
            self.picks.push(Pick::MoveTo(d.0, id));
        }
        // A custom category: Update Instances of everything in it.
        shown.header_response.context_menu(|ui| {
            let mut keys = Vec::new();
            all_under(node, kind, &mut keys);
            if ui
                .add_enabled(!keys.is_empty(), egui::Button::new("Update Instances"))
                .on_hover_text("Every blueprint in this category")
                .clicked()
            {
                self.picks.push(Pick::UpdateInstances(keys));
                ui.close();
            }
        });
    }

    fn row(&mut self, ui: &mut egui::Ui, b: &PaletteBlueprint) {
        // Rows out of sight take their room only (a palette may list
        // thousands).
        let height = ui.spacing().interact_size.y;
        let room = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(1.0, height));
        if !ui.is_rect_visible(room) {
            ui.allocate_space(egui::vec2(1.0, height));
            return;
        }
        let (kind, custom) = (self.kind, self.custom);
        let name = b.name.text(self.game);
        let key = ResKey::new(b.resref, kind.restype());
        let favorite = self.favorites.contains(&b.resref);
        let label = match b.cr {
            Some(cr) => format!("{name}  (CR {cr})"),
            None => name.clone(),
        };
        let label = if favorite { format!("★ {label}") } else { label };
        // Dragged into an area view, it is placed where it is dropped; onto
        // a custom category, it moves there.
        let r = ui.add(
            egui::Button::selectable(self.sel.has(key), label).sense(egui::Sense::click_and_drag()),
        );
        if r.hovered() {
            self.hovered = Some(key);
        }
        let thumb = self.thumb.filter(|(k, _)| *k == key).and_then(|(_, t)| t);
        let tags = self.tags;
        let r = r.on_hover_ui(|ui| {
            ui.strong(&name);
            ui.label(format!("ResRef {}", b.resref));
            if let Some(t) = tags.get(&b.resref).filter(|t| !t.is_empty()) {
                ui.label(format!("Tag {t}"));
            }
            if let Some(id) = thumb {
                let size = crate::model_view::THUMBNAIL as f32;
                ui.image(egui::load::SizedTexture::new(id, egui::vec2(size, size)));
            }
        });
        if r.clicked() {
            // Ctrl+click chooses several custom blueprints.
            let sel = &mut self.sel;
            if custom && ui.input(|i| i.modifiers.command) {
                if let Some(at) = sel.chosen.iter().position(|k| *k == key) {
                    sel.chosen.remove(at);
                } else if sel.selected != Some(key) {
                    sel.chosen.push(key);
                }
            } else {
                sel.selected = Some(key);
                sel.chosen.clear();
            }
        }
        if r.drag_started() {
            r.dnd_set_drag_payload(Dragged(key));
        }
        if r.double_clicked() {
            self.picks.push(if custom { Pick::Edit(key) } else { Pick::Preview(key) });
        }
        let (sel, picks) = (&self.sel, &mut self.picks);
        r.context_menu(|ui| {
            if custom && ui.button("Edit").clicked() {
                picks.push(Pick::Edit(key));
                ui.close();
            }
            let keys = sel.bulk(key);
            if custom
                && keys.len() > 1
                && ui
                    .button(format!("Edit {} Together", keys.len()))
                    .on_hover_text("One editor: what you change is set on each")
                    .clicked()
            {
                picks.push(Pick::EditTogether(keys));
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
            if crate::model_view::previewable(key.restype) && ui.button("Preview").clicked() {
                picks.push(Pick::Preview(key));
                ui.close();
            }
            let text = if favorite { "Remove from Favorites" } else { "Add to Favorites" };
            if ui.button(text).clicked() {
                picks.push(Pick::Favorite(key, !favorite));
                ui.close();
            }
            if ui
                .button("Find References")
                .on_hover_text("Where the module places or names this blueprint")
                .clicked()
            {
                picks.push(Pick::References(key));
                ui.close();
            }
            if custom {
                let keys = sel.bulk(key);
                let text = match keys.len() {
                    1 => "Update Instances".to_string(),
                    n => format!("Update Instances of {n}"),
                };
                if ui
                    .button(text)
                    .on_hover_text("Make the objects placed from it again from it")
                    .clicked()
                {
                    picks.push(Pick::UpdateInstances(keys));
                    ui.close();
                }
            }
            if custom && ui.button("Delete").clicked() {
                picks.push(Pick::Delete(key));
                ui.close();
            }
        });
    }

    /// Favorites or Recent: the remembered blueprints this palette has, in
    /// order.
    fn remembered(&mut self, ui: &mut egui::Ui, title: &str, list: &[String], palette: &Palette) {
        let ext = self.kind.restype().extension().unwrap_or_default();
        let all = palette.blueprints();
        let here: Vec<&PaletteBlueprint> = list
            .iter()
            .filter_map(|r| r.strip_prefix(&format!("{ext}:")))
            .filter_map(|r| all.iter().find(|(_, b)| b.resref.to_string() == r).map(|(_, b)| *b))
            .collect();
        if here.is_empty() {
            return;
        }
        egui::CollapsingHeader::new(format!("{title} ({})", here.len()))
            .id_salt(("palette-remembered", title, self.kind, self.custom))
            .default_open(true)
            .show(ui, |ui| {
                for b in here {
                    self.row(ui, b);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_words_anywhere_or_letters_in_order() {
        let f = Find::new("Long  SWORD");
        assert!(f.hit(&["longsword +1", "nw_wswls001", ""]));
        assert!(f.hit(&["blade", "longsword", "sword_tag"]));
        assert!(!f.hit(&["short sword", "nw_wswss001", ""]));
        let mut f = Find::new("lngswd");
        assert!(!f.hit(&["longsword", "", ""]));
        f.fuzzy = true;
        assert!(f.hit(&["longsword", "", ""]));
        assert!(!f.hit(&["swordlong", "", ""]));
        assert!(Find::new("  ").is_empty());
    }

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
