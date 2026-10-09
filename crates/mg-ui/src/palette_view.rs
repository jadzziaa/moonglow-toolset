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

/// Whether blueprints of a kind have pictures (a model to draw: their own,
/// or a waypoint's flag).
fn pictured(kind: BlueprintKind) -> bool {
    use BlueprintKind as K;
    matches!(kind, K::Creature | K::Door | K::Item | K::Placeable | K::Waypoint)
}

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
    /// Expand All or Collapse All was clicked this frame: every category
    /// opens (true) or closes.
    pub(crate) fold: Option<bool>,
    standard: HashMap<BlueprintKind, Arc<Palette>>,
    /// Custom palettes, each built for a workspace revision.
    custom_cache: HashMap<BlueprintKind, (u64, Arc<Palette>)>,
    pub selected: Option<ResKey>,
    /// The hovered blueprint's thumbnail, once rendered.
    thumb: Option<(ResKey, Option<egui::TextureId>)>,
    /// What there is to say of the hovered blueprint, where it has no
    /// model to show (a sound, a trigger, an encounter, a store, a
    /// waypoint).
    about: Option<(ResKey, Vec<String>)>,
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
    /// The prefabs are shown (groups of placed objects saved to place
    /// again), rather than blueprints.
    pub prefabs: bool,
    /// The tileset brush chosen.
    pub tile_brush: Option<crate::terrain_mode::TileBrush>,
    /// The area shown last: its tileset's palette is the one shown.
    pub area: Option<ResRef>,
    pub(crate) tile_palettes: HashMap<ResRef, Arc<mg_area::terrain::TilesetPalette>>,
    /// Tilesets' names as Area Properties shows them, read once each (a
    /// big tileset takes milliseconds to read: too long for every frame).
    pub(crate) tileset_names: HashMap<ResRef, String>,
    /// The row the arrow keys are at: the one clicked last, or come to
    /// with them.
    pub(crate) cursor: Option<At>,
    /// A category to open or close when the palette is drawn next (the
    /// Left and Right keys).
    toggle: Option<(Branch, bool)>,
    /// The cursor's row is to be brought into view: the keys moved it.
    reveal: bool,
}

/// A branch of a palette's tree: a category (by its place among the
/// categories, down from the top) or the Favorites or Recent list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Branch {
    Category(Vec<usize>),
    Remembered(&'static str),
}

/// A row of a palette's tree: a branch's own, or a blueprint's under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct At {
    kind: BlueprintKind,
    custom: bool,
    pub(crate) branch: Branch,
    pub(crate) blueprint: Option<ResKey>,
}

/// The palette's rows as they were drawn, top to bottom, for the arrow
/// keys to move through: listed only in a frame a key is pressed.
#[derive(Debug, Default)]
struct Listed {
    branches: Vec<Listing>,
    /// Each row's branch, and the blueprint if it is one's.
    rows: Vec<(usize, Option<ResKey>)>,
}

/// A branch as listed: the branch it is in, its own row, and whether it
/// is open.
#[derive(Debug)]
struct Listing {
    branch: Branch,
    parent: Option<usize>,
    row: usize,
    open: bool,
}

/// What an arrow key does from a row.
#[derive(Debug, PartialEq)]
enum Step {
    /// The cursor goes to this row.
    To(usize),
    /// This branch opens or closes.
    Fold(usize, bool),
    Stay,
}

impl Listed {
    fn branch(&mut self, branch: Branch, parent: Option<usize>) -> usize {
        self.branches.push(Listing { branch, parent, row: self.rows.len(), open: false });
        self.rows.push((self.branches.len() - 1, None));
        self.branches.len() - 1
    }

    /// The branch a row is in (a branch's own row: its parent).
    fn within(&self, row: usize) -> Option<usize> {
        let (branch, blueprint) = self.rows[row];
        if blueprint.is_some() { Some(branch) } else { self.branches[branch].parent }
    }

    /// Whether a row shows: every branch it is in is open. (A branch
    /// closing still draws its rows for a moment.)
    fn shows(&self, row: usize) -> bool {
        let mut at = self.within(row);
        while let Some(b) = at {
            if !self.branches[b].open {
                return false;
            }
            at = self.branches[b].parent;
        }
        true
    }

    /// The row of `at`, if it is listed.
    fn find(&self, at: &At) -> Option<usize> {
        let branch = self.branches.iter().position(|b| b.branch == at.branch)?;
        match at.blueprint {
            None => Some(self.branches[branch].row),
            key => self.rows.iter().position(|r| *r == (branch, key)),
        }
    }

    /// A blueprint's row, in its category before Favorites or Recent.
    fn blueprint(&self, key: ResKey) -> Option<usize> {
        let of = |category: bool| {
            self.rows.iter().position(|(b, k)| {
                *k == Some(key)
                    && matches!(self.branches[*b].branch, Branch::Category(_)) == category
            })
        };
        of(true).or_else(|| of(false))
    }

    /// What `key` does with the cursor at `row`, as in any tree: Up and
    /// Down go through the rows showing; Left closes an open branch, and
    /// from a closed one or a blueprint goes to the branch it is in;
    /// Right opens a closed branch, and from an open one goes into it.
    fn step(&self, row: usize, key: egui::Key) -> Step {
        let (branch, blueprint) = self.rows[row];
        let next = (row + 1..self.rows.len()).find(|r| self.shows(*r));
        match key {
            egui::Key::ArrowDown => next.map_or(Step::Stay, Step::To),
            egui::Key::ArrowUp => {
                (0..row).rev().find(|r| self.shows(*r)).map_or(Step::Stay, Step::To)
            }
            egui::Key::ArrowLeft if blueprint.is_none() && self.branches[branch].open => {
                Step::Fold(branch, false)
            }
            egui::Key::ArrowLeft => {
                self.within(row).map_or(Step::Stay, |b| Step::To(self.branches[b].row))
            }
            egui::Key::ArrowRight if blueprint.is_none() && !self.branches[branch].open => {
                Step::Fold(branch, true)
            }
            egui::Key::ArrowRight if blueprint.is_none() => {
                next.filter(|r| self.within(*r) == Some(branch)).map_or(Step::Stay, Step::To)
            }
            _ => Step::Stay,
        }
    }
}

/// Whether the arrow keys are the palette's: from a click in its tree
/// until the pointer is in an area's view again (they move its camera
/// there, as W, A, S and D go on doing all along). The palette moves by
/// them with the pointer over it.
pub(crate) fn has_arrows(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::new("palette-arrow-keys"))).unwrap_or(false)
}

/// Gives the arrow keys to the palette, or back to the area's view.
pub(crate) fn give_arrows(ctx: &egui::Context, palette: bool) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("palette-arrow-keys"), palette));
}

impl PaletteView {
    /// Whether the arrow keys' cursor is at a category's own row (not at
    /// a blueprint's).
    pub fn at_category(&self) -> bool {
        self.cursor.as_ref().is_some_and(|c| c.blueprint.is_none())
    }

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
            fold: None,
            about: None,
            found: None,
            tags: HashMap::new(),
            chosen: Vec::new(),
            tiles: false,
            prefabs: false,
            tile_brush: None,
            area: None,
            tile_palettes: HashMap::new(),
            tileset_names: HashMap::new(),
            cursor: None,
            toggle: None,
            reveal: false,
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

/// Opens one of the game's blueprints to look at (the module's own of
/// that name, if it has one, to edit).
pub(crate) fn view_blueprint(app: &mut Moonglow, key: ResKey) {
    let (Some(game), Some(ws)) = (app.game.as_deref(), app.ws.as_mut()) else { return };
    if !ws.module.contains(&key) {
        let Some(gff) = game.resman.get(&key).ok().and_then(|d| Gff::read(&d).ok()) else {
            app.log.warn(format!("{key} could not be read"));
            return;
        };
        ws.view(key, gff);
    }
    if let Some(t) = Tab::for_resource(key) {
        app.actions.push(Action::OpenTab(t));
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
    /// One of the game's, opened to look at.
    View(ResKey),
    EditCopy(ResKey),
    Delete(ResKey),
    Preview(ResKey),
    /// The objects selected in the area become ones of this blueprint.
    Replace(ResKey),
    References(ResKey),
    UpdateInstances(Vec<ResKey>),
    EditTogether(Vec<ResKey>),
    /// The Export window, for these blueprints of the module's.
    Export(Vec<ResKey>),
    /// A new blueprint of the palette's kind: its wizard, as New… opens.
    New,
    /// Into a custom category (its palette id).
    MoveTo(ResKey, u8),
    /// Added to (true) or removed from Favorites.
    Favorite(ResKey, bool),
}

/// Expand All and Collapse All as two small buttons, for a list's toolbar.
fn fold_icons(ui: &mut egui::Ui, what: &str) -> Option<bool> {
    let mut fold = None;
    if ui.small_button("⏷").on_hover_text(format!("Expand All: open every {what}")).clicked() {
        fold = Some(true);
    }
    if ui.small_button("⏶").on_hover_text(format!("Collapse All: close every {what}")).clicked() {
        fold = Some(false);
    }
    fold
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui) {
    if app.game.is_none() {
        ui.label("No game data.");
        return;
    }
    // Laid out in the pane's visible width: its rows wrap there. (A pane
    // that scrolls sideways offers the width of what it last held, so a
    // row once too long would never wrap again.)
    let seen = ui.clip_rect().right() - ui.cursor().left();
    if seen > 0.0 && seen < ui.available_width() {
        ui.set_max_width(seen);
    }
    let mut view = std::mem::take(&mut app.palette);
    crate::trace::changed("palette", || {
        format!(
            "in {:?}; tiles {}, prefabs {}, kind {:?}, custom {}, area {:?}, find {:?}",
            ui.max_rect(),
            view.tiles,
            view.prefabs,
            view.kind,
            view.custom,
            view.area,
            view.filter
        )
    });
    // The header (GitHub issue 16), top down by what each row governs:
    // the type, its name and the source, the search, then the list's own
    // toolbar right over the list.
    //
    // The types: one row of icons, a click each, named under the pointer
    // (and below, for the one shown); in a pane too narrow for the row, a
    // dropdown with their names.
    {
        use crate::icons::{PREFABS, TILES, blueprint, labelled};
        #[derive(Clone, Copy, PartialEq)]
        enum Type {
            Tiles,
            Kind(BlueprintKind),
            Prefabs,
        }
        let now = if view.tiles {
            Type::Tiles
        } else if view.prefabs {
            Type::Prefabs
        } else {
            Type::Kind(view.kind)
        };
        let mut types = vec![(Type::Tiles, TILES, "Tiles", "The area's tileset")];
        types.extend(BlueprintKind::ALL.map(|k| (Type::Kind(k), blueprint(k), k.label(), "")));
        types.push((
            Type::Prefabs,
            PREFABS,
            "Prefabs",
            "Groups of placed objects saved under a name (a camp, a furnished room), to place \
             again in any area or module",
        ));
        let mut chosen = None;
        // (Close together: eleven of them, in a pane that is narrow.)
        let each = 20.0;
        if ui.available_width() >= each * types.len() as f32 {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 1.0;
                ui.spacing_mut().button_padding.x = 2.0;
                for (t, glyph, name, tip) in &types {
                    let tip =
                        if tip.is_empty() { name.to_string() } else { format!("{name}: {tip}") };
                    if ui.selectable_label(now == *t, *glyph).on_hover_text(tip).clicked() {
                        chosen = Some(*t);
                    }
                }
            });
        } else {
            let shown = types.iter().find(|t| t.0 == now).expect("one of them");
            egui::ComboBox::from_id_salt("palette-type")
                .selected_text(labelled(shown.1, shown.2))
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for (t, glyph, name, _) in &types {
                        if ui.selectable_label(now == *t, labelled(glyph, name)).clicked() {
                            chosen = Some(*t);
                        }
                    }
                });
        }
        match chosen {
            Some(Type::Tiles) => {
                view.tiles = true;
                view.prefabs = false;
            }
            Some(Type::Kind(kind)) => {
                view.kind = kind;
                view.tiles = false;
                view.prefabs = false;
            }
            Some(Type::Prefabs) => {
                view.prefabs = true;
                view.tiles = false;
                // (No blueprint is about to be placed any more.)
                view.selected = None;
                view.chosen.clear();
            }
            None => {}
        }
        // The type shown, by name; the source beside it (a blueprint
        // type's: the game's, or the module's).
        let name = types.iter().find(|t| t.0 == now).map_or("", |t| t.2);
        ui.horizontal(|ui| {
            ui.strong(name);
            if !view.tiles && !view.prefabs {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(app.ws.is_some(), |ui| {
                        ui.selectable_value(&mut view.custom, true, "Custom")
                            .on_hover_text("The module's blueprints");
                    });
                    ui.selectable_value(&mut view.custom, false, "Standard")
                        .on_hover_text("The game's blueprints");
                });
            }
        });
    }
    view.fold = None;
    if view.prefabs {
        ui.separator();
        app.palette = view;
        prefabs_ui(app, ui);
        return;
    }
    if view.tiles {
        // (The tile palette's groups open and close, too.)
        ui.horizontal(|ui| view.fold = fold_icons(ui, "group"));
        ui.separator();
        app.palette = view;
        crate::terrain_mode::palette_ui(app, ui);
        return;
    }
    // The search: the pane's width, with what it looks in said, and a
    // button to clear it.
    ui.horizontal(|ui| {
        let clear = !view.filter.is_empty();
        let room = ui.available_width() - if clear { 26.0 } else { 0.0 };
        ui.add(
            egui::TextEdit::singleline(&mut view.filter)
                .hint_text("🔍 Search name, tag, resref")
                .desired_width(room.max(60.0)),
        );
        if clear && ui.small_button("×").on_hover_text("Clear the search").clicked() {
            view.filter.clear();
        }
    });

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

    // The list's toolbar, right over the list: every category opened or
    // closed, how many blueprints the list has (of the search, if any),
    // names or pictures, a new blueprint, and the rest under "more".
    let mut categories = None;
    ui.horizontal_wrapped(|ui| {
        view.fold = fold_icons(ui, "category");
        fn count(nodes: &[mg_module::palette::PaletteNode]) -> usize {
            nodes.iter().map(|n| n.blueprints.len() + count(&n.children)).sum()
        }
        let items = found.as_ref().map_or_else(|| count(&palette.nodes), |f| f.len());
        ui.weak(format!("{items} item{}", if items == 1 { "" } else { "s" })).on_hover_text(
            if searching { "Blueprints found" } else { "Blueprints in this palette" },
        );
        ui.separator();
        // List or Gallery: names, or pictures in their place for the types
        // that have them (the others are lists, whichever is chosen).
        let pictures = pictured(view.kind);
        let shown = app.settings.palette_gallery && pictures;
        if ui.selectable_label(!shown, "☰").on_hover_text("List: the blueprints by name").clicked()
        {
            app.settings.palette_gallery = false;
        }
        ui.add_enabled_ui(pictures, |ui| {
            if ui
                .selectable_label(shown, "⊞")
                .on_hover_text("Gallery: the blueprints as pictures, to choose by eye")
                .on_disabled_hover_text(
                    "Gallery: creatures, doors, items, placeables and waypoints have pictures",
                )
                .clicked()
            {
                app.settings.palette_gallery = true;
            }
        });
        let creature = view.kind == BlueprintKind::Creature;
        let wizard = creature || crate::blueprint_wizard::KINDS.contains(&view.kind);
        if ui
            .add_enabled(app.ws.is_some() && wizard, egui::Button::new("New…"))
            .on_hover_text("A new blueprint (the type's wizard)")
            .clicked()
        {
            new_blueprint(app, view.kind);
        }
        ui.menu_button("More", |ui| {
            if ui
                .add_enabled(app.ws.is_some(), egui::Button::new("Categories…"))
                .on_hover_text(
                    "Add, rename and remove the categories this module's blueprints of this \
                     kind go in",
                )
                .clicked()
            {
                categories = Some(view.kind);
                ui.close();
            }
            // Every appearance there is, not only the blueprints'.
            if view.kind == BlueprintKind::Placeable
                && ui
                    .button("All Appearances…")
                    .on_hover_text(
                        "The Placeable Gallery: every placeable appearance as a picture; a \
                         click gives it to the placeables selected in the area",
                    )
                    .clicked()
            {
                app.actions.push(Action::PlaceableGallery);
                ui.close();
            }
        })
        .response
        .on_hover_text("More: the module's categories, every placeable appearance");
        if shown {
            // (A slider doesn't wrap of itself: on a row of its own where
            // this one has no room for it.)
            if ui.available_size_before_wrap().x < ui.spacing().slider_width {
                ui.end_row();
            }
            crate::appearance_gallery::size_slider(app, ui);
        }
    });
    // A search typed opens every category with a match, as it is typed.
    // (So does another palette shown with the search still there.)
    let said = format!("{:?}/{}/{}/{}", view.kind, view.custom, view.tiles, view.filter);
    let typed = crate::widgets::text_changed(ui, egui::Id::new("palette-filter"), &said);
    if typed && !view.filter.trim().is_empty() {
        view.fold = view.fold.or(Some(true));
    }
    if let Some(kind) = categories {
        app.palette = std::mem::take(&mut view);
        crate::palette_categories::open(app, kind);
        view = std::mem::take(&mut app.palette);
    }
    ui.separator();
    let game = app.game.as_deref().expect("checked");
    let favorites = app.settings.palette_favorites.clone();
    let recent = app.settings.palette_recent.clone();
    let (resrefs, cr) = (app.settings.name_resrefs, !app.settings.palette_no_cr);
    let ext = kind.restype().extension().unwrap_or_default();
    // An arrow key pressed while the keys are the palette's, the pointer
    // is over it (not over a list elsewhere that has keys of its own) and
    // no text is being typed: the rows are listed as they are drawn, to
    // move through.
    let here = ui.rect_contains_pointer(ui.max_rect());
    let pressed = (here && has_arrows(ui.ctx()) && !ui.ctx().egui_wants_keyboard_input())
        .then(|| {
            use egui::Key::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp};
            ui.input(|i| {
                let plain = i.modifiers.is_none();
                [ArrowUp, ArrowDown, ArrowLeft, ArrowRight]
                    .into_iter()
                    .find(|k| plain && i.key_pressed(*k))
            })
        })
        .flatten();
    let mut tree = Tree {
        game,
        kind,
        custom,
        fold: view.fold,
        new: (app.ws.is_some() && wizard_for(kind)).then(|| new_label(kind)),
        found: found.as_deref(),
        tags: &tags,
        favorites: favorites
            .iter()
            .filter_map(|f| ResRef::from_str(f.strip_prefix(&format!("{ext}:"))?).ok())
            .collect(),
        thumb: view.thumb,
        gallery: app.settings.palette_gallery && pictured(kind),
        side: crate::appearance_gallery::tile_side(app),
        ready: app.thumbnails.all(),
        shown: Vec::new(),
        about: view.about.as_ref().map(|(k, lines)| (*k, lines.as_slice())),
        sel: Selection { selected: view.selected, chosen: std::mem::take(&mut view.chosen) },
        picks: Vec::new(),
        hovered: None,
        resrefs,
        cr,
        cursor: view.cursor.take().filter(|c| c.kind == kind && c.custom == custom),
        reveal: std::mem::take(&mut view.reveal),
        toggle: view.toggle.take(),
        listed: pressed.map(|_| Listed::default()),
        clicked: false,
    };
    if find.fuzzy {
        ui.weak("No exact matches: close ones");
    }
    let mut list = egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        // A row is one line: a name too long for the pane is cut short
        // (the pointer over it shows it whole, and its ResRef, tag and
        // challenge rating), as in the module tree. A wider pane shows
        // more.
        if !tree.gallery {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        }
        if find.is_empty() {
            tree.remembered(ui, "Favorites", &favorites, &palette);
            tree.remembered(ui, "Recent", &recent, &palette);
        }
        for (i, node) in palette.nodes.iter().enumerate() {
            tree.node(ui, node, &[i], None);
        }
        // The room under the list has the menu too.
        let rest = egui::vec2(ui.available_width(), ui.available_height().max(24.0));
        let (_, below) = ui.allocate_exact_size(rest, egui::Sense::click());
        if let Some(label) = tree.new {
            below.context_menu(|ui| {
                if ui.button(label).clicked() {
                    tree.picks.push(Pick::New);
                    ui.close();
                }
            });
        }
    });
    crate::widgets::home_and_end(ui, &mut list);
    let shown = std::mem::take(&mut tree.shown);
    // The arrow keys: the cursor moves through the rows as they were
    // drawn, or its category opens or closes (when the palette is drawn
    // next). A blueprint it comes to is selected, as by a click; on a
    // category, the blueprint selected stays in hand.
    if let (Some(key), Some(listed)) = (pressed, tree.listed.take()) {
        let from = tree
            .cursor
            .as_ref()
            .and_then(|c| listed.find(c))
            .filter(|r| listed.shows(*r))
            .or_else(|| tree.sel.selected.and_then(|k| listed.blueprint(k)));
        let step = match from {
            Some(row) => listed.step(row, key),
            // Nothing to start from: the first row, or the last.
            None => {
                let mut showing = (0..listed.rows.len()).filter(|r| listed.shows(*r));
                let to = match key {
                    egui::Key::ArrowDown => showing.next(),
                    egui::Key::ArrowUp => showing.next_back(),
                    _ => None,
                };
                to.map_or(Step::Stay, Step::To)
            }
        };
        match step {
            Step::To(row) => {
                let (branch, blueprint) = listed.rows[row];
                let branch = listed.branches[branch].branch.clone();
                tree.cursor = Some(At { kind, custom, branch, blueprint });
                if blueprint.is_some() {
                    tree.sel.selected = blueprint;
                    tree.sel.chosen.clear();
                }
                view.reveal = true;
            }
            Step::Fold(branch, open) => {
                view.toggle = Some((listed.branches[branch].branch.clone(), open));
            }
            Step::Stay => {}
        }
        ui.ctx().request_repaint();
    }
    if tree.clicked {
        give_arrows(ui.ctx(), true);
    }
    view.cursor = tree.cursor.take();
    let (sel, picks, hovered) = (tree.sel, tree.picks, tree.hovered);
    if sel.selected.is_some() && sel.selected != view.selected {
        view.tile_brush = None;
    }
    view.selected = sel.selected;
    view.chosen = sel.chosen;
    app.palette = view;
    // The Gallery's pictures in sight: kept while they are, and those
    // still to make are made a few a frame, the first in sight first, so
    // a long category doesn't hold the toolset up
    // (`model_view::make_thumbnails`).
    use crate::model_view::Pictured;
    app.thumbnails.show(shown.into_iter().map(Pictured::Resource));
    // The hovered blueprint's picture, for its tooltip once it is made.
    if let Some(key) = hovered
        && let Some(id) = app.thumbnails.get(Pictured::Resource(key))
        && app.palette.thumb != Some((key, id))
    {
        app.palette.thumb = Some((key, id));
        ui.ctx().request_repaint();
    }
    if let Some(key) = hovered
        && app.palette.about.as_ref().is_none_or(|(k, _)| *k != key)
    {
        let lines = about(app, key);
        app.palette.about = Some((key, lines));
        ui.ctx().request_repaint();
    }

    for pick in picks {
        match pick {
            Pick::Edit(key) => {
                if let Some(t) = Tab::for_resource(key) {
                    app.actions.push(Action::OpenTab(t));
                }
            }
            Pick::EditCopy(key) => app.copy_dialog(key, true),
            Pick::View(key) => view_blueprint(app, key),
            Pick::Delete(key) => {
                let cmd = Command::new(
                    format!("Delete {key}"),
                    vec![Edit::SetResource { key, data: None }],
                );
                if let Err(e) = app.apply(cmd) {
                    app.log.error(e.to_string());
                }
            }
            Pick::New => new_blueprint(app, kind),
            Pick::Preview(key) => app.actions.push(Action::OpenTab(Tab::Model(key))),
            Pick::Replace(key) => match app.palette.area {
                Some(area) => {
                    let n = crate::area_view::replace_selected(app, area, key);
                    if n > 0 {
                        app.log.info(format!("Replaced {n} with {}", key.resref));
                    }
                }
                None => app.log.warn("Open an area and select what to replace first"),
            },
            Pick::References(key) => app.actions.push(Action::FindReferences(key)),
            Pick::UpdateInstances(keys) => app.update_instances_of(keys),
            Pick::EditTogether(keys) => app.actions.push(Action::OpenTab(Tab::Blueprints(keys))),
            Pick::Export(keys) => app.actions.push(Action::ExportDialog(keys)),
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
/// A palette name as its row shows it: its text, or, where it is a
/// talk-table string that isn't there (a custom talk table that wasn't
/// found, a line the table lacks), which string it is, so that a row
/// without a name says why.
fn shown_name(name: &mg_module::palette::PaletteName, game: &mg_rules::GameData) -> String {
    let text = name.text(game);
    match name {
        mg_module::palette::PaletteName::StrRef(s) if text.trim().is_empty() => {
            let custom = mg_core::StrRef(*s).is_custom();
            format!("(no text: {}string {s})", if custom { "custom talk table " } else { "" })
        }
        _ => text,
    }
}

/// Whether a kind of blueprint has a wizard to make a new one.
fn wizard_for(kind: BlueprintKind) -> bool {
    kind == BlueprintKind::Creature || crate::blueprint_wizard::KINDS.contains(&kind)
}

/// "New Creature…" and the like, for the palettes' menus.
fn new_label(kind: BlueprintKind) -> &'static str {
    match kind {
        BlueprintKind::Creature => "New Creature…",
        BlueprintKind::Door => "New Door…",
        BlueprintKind::Encounter => "New Encounter…",
        BlueprintKind::Item => "New Item…",
        BlueprintKind::Placeable => "New Placeable…",
        BlueprintKind::Sound => "New Sound…",
        BlueprintKind::Store => "New Merchant…",
        BlueprintKind::Trigger => "New Trigger…",
        BlueprintKind::Waypoint => "New Waypoint…",
    }
}

/// Opens the wizard that makes a new blueprint of `kind`.
fn new_blueprint(app: &mut Moonglow, kind: BlueprintKind) {
    if kind == BlueprintKind::Creature {
        app.creature_wizard = Some(Default::default());
    } else if crate::blueprint_wizard::KINDS.contains(&kind) {
        app.blueprint_wizard = Some(crate::blueprint_wizard::BlueprintWizard::new(kind));
    }
}

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
    /// Every category opened or closed this frame.
    fold: Option<bool>,
    /// What a right click offers to make a new blueprint of the kind
    /// ("New Creature…"), where the kind has a wizard and a module is open.
    new: Option<&'static str>,
    /// The blueprints the search finds (`None`: no search).
    found: Option<&'a std::collections::HashSet<ResRef>>,
    tags: &'a HashMap<ResRef, String>,
    /// This palette's favorites.
    favorites: std::collections::HashSet<ResRef>,
    /// The thumbnail ready for the blueprint hovered last frame.
    thumb: Option<(ResKey, Option<egui::TextureId>)>,
    /// The Gallery: blueprints as pictures. Those made so far, and those
    /// in sight (made or not).
    gallery: bool,
    side: f32,
    ready: HashMap<ResKey, Option<egui::TextureId>>,
    shown: Vec<ResKey>,
    /// And what there is to say of it.
    about: Option<(ResKey, &'a [String])>,
    sel: Selection,
    picks: Vec<Pick>,
    /// The blueprint hovered now.
    hovered: Option<ResKey>,
    /// Options › General: ResRefs beside names, and creatures' challenge
    /// ratings.
    resrefs: bool,
    cr: bool,
    /// The row the arrow keys are at, and whether to bring it into view.
    cursor: Option<At>,
    reveal: bool,
    /// A branch the keys open or close in this frame.
    toggle: Option<(Branch, bool)>,
    /// The rows as drawn, where a key was pressed (`None`: not listed).
    listed: Option<Listed>,
    /// Something in the tree was clicked: the arrow keys are its own.
    clicked: bool,
}

/// Where a blueprint's row is: its branch, the branch's place in the
/// keys' listing (if the rows are listed) and the blueprint the cursor is
/// at in the branch.
type Within<'a> = (&'a Branch, Option<usize>, Option<ResKey>);

/// A blueprint's row: its name, its ResRef in parentheses and a creature's
/// challenge rating, where each is asked for.
fn blueprint_label(name: &str, resref: Option<ResRef>, cr: Option<f32>) -> String {
    let mut label = name.to_string();
    if let Some(r) = resref {
        label.push_str(&format!(" ({r})"));
    }
    if let Some(cr) = cr {
        label.push_str(&format!("  (CR {cr})"));
    }
    label
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

    /// Whether the arrow keys' cursor is in `branch`: at its own row, or
    /// at a blueprint's under it.
    fn cursor_in(&self, branch: &Branch) -> Option<Option<ResKey>> {
        self.cursor.as_ref().filter(|c| c.branch == *branch).map(|c| c.blueprint)
    }

    /// What the keys ask of a branch's header, this frame: open or closed.
    fn asked(&self, branch: &Branch) -> Option<bool> {
        self.fold.or(self.toggle.as_ref().filter(|(b, _)| b == branch).map(|(_, open)| *open))
    }

    /// A branch's header as drawn: marked where the cursor is at it,
    /// brought into view if the keys moved the cursor there, the cursor's
    /// on a click, and listed open or closed for the keys.
    fn header(
        &mut self,
        ui: &egui::Ui,
        branch: &Branch,
        listing: Option<usize>,
        header: &egui::Response,
        behind: egui::layers::ShapeIdx,
    ) {
        if self.cursor_in(branch) == Some(None) {
            let fill = ui.visuals().selection.bg_fill.gamma_multiply(0.6);
            let radius = ui.visuals().widgets.inactive.corner_radius;
            ui.painter().set(behind, egui::epaint::RectShape::filled(header.rect, radius, fill));
            if self.reveal {
                header.scroll_to_me(None);
            }
        }
        if header.clicked() {
            let (kind, custom) = (self.kind, self.custom);
            self.cursor = Some(At { kind, custom, branch: branch.clone(), blueprint: None });
            self.clicked = true;
        }
        if let (Some(at), Some(listed)) = (listing, &mut self.listed) {
            use egui::collapsing_header::CollapsingState;
            listed.branches[at].open =
                CollapsingState::load(ui.ctx(), header.id).is_some_and(|s| s.is_open());
        }
    }

    fn node(
        &mut self,
        ui: &mut egui::Ui,
        node: &PaletteNode,
        path: &[usize],
        parent: Option<usize>,
    ) {
        if !self.any_shown(node) {
            return;
        }
        let (game, kind, custom) = (self.game, self.kind, self.custom);
        let branch = Branch::Category(path.to_vec());
        let listing = self.listed.as_mut().map(|l| l.branch(branch.clone(), parent));
        let within = self.cursor_in(&branch).flatten();
        let behind = ui.painter().add(egui::Shape::Noop);
        let count = node.blueprints.len();
        let title = if count > 0 {
            format!("{} ({count})", shown_name(&node.name, game))
        } else {
            shown_name(&node.name, game)
        };
        // (Finding opens every category with a match as it is typed,
        // through `fold`: they can be closed again after.)
        let shown = egui::CollapsingHeader::new(title)
            .id_salt(("palette", kind, custom, path))
            .open(self.asked(&branch))
            .show(ui, |ui| {
                for (i, child) in node.children.iter().enumerate() {
                    let mut p = path.to_vec();
                    p.push(i);
                    self.node(ui, child, &p, listing);
                }
                let rows: Vec<&PaletteBlueprint> =
                    node.blueprints.iter().filter(|b| self.shown(b)).collect();
                self.rows(ui, &rows, (&branch, listing, within));
            });
        self.header(ui, &branch, listing, &shown.header_response, behind);
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
        let new = self.new;
        let mut made = false;
        shown.header_response.context_menu(|ui| {
            if let Some(label) = new {
                if ui.button(label).clicked() {
                    made = true;
                    ui.close();
                }
                ui.separator();
            }
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
        if made {
            self.picks.push(Pick::New);
        }
    }

    /// A blueprint's row in a branch: with the branch, its place in the
    /// keys' listing and the blueprint the cursor is at in it.
    fn row(&mut self, ui: &mut egui::Ui, b: &PaletteBlueprint, within: Within<'_>) {
        let (branch, listing, cursor) = within;
        let (kind, custom) = (self.kind, self.custom);
        let key = ResKey::new(b.resref, kind.restype());
        if let (Some(at), Some(listed)) = (listing, &mut self.listed) {
            listed.rows.push((at, Some(key)));
        }
        // (The row the keys moved the cursor to is brought into view.)
        let reveal = self.reveal && cursor == Some(key);
        // Rows out of sight take their room only (a palette may list
        // thousands).
        let height = ui.spacing().interact_size.y;
        let room = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(1.0, height));
        if !self.gallery && !reveal && !ui.is_rect_visible(room) {
            ui.allocate_space(egui::vec2(1.0, height));
            return;
        }
        let name = shown_name(&b.name, self.game);
        let favorite = self.favorites.contains(&b.resref);
        let label =
            blueprint_label(&name, self.resrefs.then_some(b.resref), b.cr.filter(|_| self.cr));
        let label = if favorite { format!("★ {label}") } else { label };
        // Dragged into an area view, it is placed where it is dropped; onto
        // a custom category, it moves there.
        let r = if self.gallery {
            self.tile(ui, key, &label)
        } else {
            ui.add(
                egui::Button::selectable(self.sel.has(key), label)
                    .sense(egui::Sense::click_and_drag()),
            )
        };
        if r.hovered() {
            self.hovered = Some(key);
        }
        if reveal {
            r.scroll_to_me(None);
        }
        if r.clicked() {
            // The arrow keys go on from here.
            self.cursor = Some(At { kind, custom, branch: branch.clone(), blueprint: Some(key) });
            self.clicked = true;
        }
        let thumb = self.thumb.filter(|(k, _)| *k == key).and_then(|(_, t)| t);
        let about = self.about.filter(|(k, _)| *k == key).map_or(&[][..], |(_, lines)| lines);
        let tags = self.tags;
        let r = r.on_hover_ui(|ui| {
            ui.strong(&name);
            ui.label(format!("ResRef {}", b.resref));
            if let Some(t) = tags.get(&b.resref).filter(|t| !t.is_empty()) {
                ui.label(format!("Tag {t}"));
            }
            if let Some(cr) = b.cr {
                ui.label(format!("Challenge rating {cr}"));
            }
            if let Some(id) = thumb {
                let size = 180.0;
                ui.image(egui::load::SizedTexture::new(id, egui::vec2(size, size)));
            }
            for line in about {
                ui.label(line);
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
            self.picks.push(if custom { Pick::Edit(key) } else { Pick::View(key) });
        }
        let (sel, picks, new) = (&self.sel, &mut self.picks, self.new);
        r.context_menu(|ui| {
            if let Some(label) = new {
                if ui.button(label).on_hover_text("A new blueprint (the type's wizard)").clicked() {
                    picks.push(Pick::New);
                    ui.close();
                }
                ui.separator();
            }
            if custom && ui.button("Edit").clicked() {
                picks.push(Pick::Edit(key));
                ui.close();
            }
            if !custom
                && ui
                    .button("View")
                    .on_hover_text("Its properties, to look at: the game's blueprint isn't changed")
                    .clicked()
            {
                picks.push(Pick::View(key));
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
                picks.push(Pick::EditTogether(keys.clone()));
                ui.close();
            }
            if custom {
                let label = if keys.len() > 1 {
                    format!("Export {}…", keys.len())
                } else {
                    "Export…".into()
                };
                if ui
                    .button(label)
                    .on_hover_text("Write it out as a file or an ERF, as in Aurora (File › Export)")
                    .clicked()
                {
                    let all = if keys.len() > 1 { keys.clone() } else { vec![key] };
                    picks.push(Pick::Export(all));
                    ui.close();
                }
            }
            if ui
                .button("Edit Copy…")
                .on_hover_text(
                    "Copy into the module's custom palette, under a ResRef and Tag you give",
                )
                .clicked()
            {
                picks.push(Pick::EditCopy(key));
                ui.close();
            }
            if crate::model_view::previewable(key.restype) && ui.button("Preview").clicked() {
                picks.push(Pick::Preview(key));
                ui.close();
            }
            if ui
                .button("Replace Selected with This")
                .on_hover_text(
                    "The objects of this type selected in the area become ones of this \
                     blueprint, where they stand",
                )
                .clicked()
            {
                picks.push(Pick::Replace(key));
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
    fn remembered(
        &mut self,
        ui: &mut egui::Ui,
        title: &'static str,
        list: &[String],
        palette: &Palette,
    ) {
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
        let branch = Branch::Remembered(title);
        let listing = self.listed.as_mut().map(|l| l.branch(branch.clone(), None));
        let within = self.cursor_in(&branch).flatten();
        let behind = ui.painter().add(egui::Shape::Noop);
        let shown = egui::CollapsingHeader::new(format!("{title} ({})", here.len()))
            .id_salt(("palette-remembered", title, self.kind, self.custom))
            .default_open(true)
            .open(self.asked(&branch))
            .show(ui, |ui| self.rows(ui, &here, (&branch, listing, within)));
        self.header(ui, &branch, listing, &shown.header_response, behind);
    }

    /// A category's blueprints: a list, or in the Gallery a grid.
    fn rows(&mut self, ui: &mut egui::Ui, rows: &[&PaletteBlueprint], within: Within<'_>) {
        if self.gallery {
            // As many across as fit, grown to fill the palette's width.
            let asked = self.side;
            // (Of the pane's visible width: a pane once wider than it
            // shows, for a row that didn't wrap, would keep that width
            // and the pictures run past its edge.)
            let seen = ui.clip_rect().right() - ui.cursor().left();
            let room = ui.available_width().min(seen) - 1.0;
            self.side = crate::appearance_gallery::fitted(room, asked, 4.0).1;
            ui.set_max_width(room);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                for b in rows {
                    self.row(ui, b, within);
                }
            });
            self.side = asked;
        } else {
            for b in rows {
                self.row(ui, b, within);
            }
        }
    }

    /// A blueprint in the Gallery: its picture over its name, cut short
    /// (whole on hover). Until the picture is made, or for a blueprint
    /// with nothing to draw, its name alone.
    fn tile(&mut self, ui: &mut egui::Ui, key: ResKey, label: &str) -> egui::Response {
        let made = self.ready.get(&key).copied();
        let (r, seen) = crate::widgets::picture_tile(ui, self.side, label, self.sel.has(key), made);
        if seen {
            self.shown.push(key);
        }
        r
    }
}

/// What a palette's hover says of a blueprint that has no model to show:
/// a sound's sounds, a trigger's kind, an encounter's creatures, a store's
/// prices, a waypoint's map note. Nothing for the kinds that have a
/// picture.
fn about(app: &Moonglow, key: ResKey) -> Vec<String> {
    use mg_core::ResType as T;
    if !matches!(key.restype, T::UTS | T::UTT | T::UTE | T::UTM | T::UTW) {
        return Vec::new();
    }
    let Some(gff) = crate::area_tools::blueprint(app, key) else { return Vec::new() };
    let s = &gff.root;
    let int = |label: &str| s.integer(label).unwrap_or(0);
    let on = |label: &str| int(label) != 0;
    // Up to four names, and how many more.
    let listed = |names: Vec<String>| {
        let more = names.len().saturating_sub(4);
        let mut text = names.into_iter().take(4).collect::<Vec<_>>().join(", ");
        if more > 0 {
            text.push_str(&format!(" and {more} more"));
        }
        text
    };
    let resrefs = |list: &str, label: &str| -> Vec<String> {
        let items = s.list(list).unwrap_or(&[]);
        items.iter().filter_map(|i| i.resref(label)).map(|r| r.to_string()).collect()
    };
    let mut out = Vec::new();
    match key.restype {
        T::UTS => {
            let sounds = resrefs("Sounds", "Sound");
            if !sounds.is_empty() {
                out.push(format!("Plays {}", listed(sounds)));
            }
            let place =
                if on("Positional") { "from where it stands" } else { "everywhere in the area" };
            let how = if on("Continuous") { "without a pause" } else { "at intervals" };
            out.push(format!("Heard {place}, {how}; volume {}", int("Volume")));
        }
        T::UTT => out.push(match int("Type") {
            1 => "An area transition".into(),
            2 => format!("A trap (type {})", int("TrapType")),
            _ => "A generic trigger".into(),
        }),
        T::UTE => {
            let creatures = resrefs("CreatureList", "ResRef");
            if !creatures.is_empty() {
                out.push(format!("Spawns {}", listed(creatures)));
            }
            out.push(format!(
                "{} to {} creatures; {}",
                int("RecCreatures"),
                int("MaxCreatures"),
                if on("SpawnOption") { "spawns once" } else { "spawns again" }
            ));
        }
        T::UTM => {
            out.push(format!("Sells at {}%, buys at {}%", int("MarkUp"), int("MarkDown")));
            let items: usize = s
                .list("StoreList")
                .unwrap_or(&[])
                .iter()
                .map(|page| page.list("ItemList").map_or(0, <[_]>::len))
                .sum();
            out.push(format!("{items} items for sale"));
        }
        T::UTW => {
            let note = s.locstring("MapNote").and_then(|l| app.game.as_deref()?.locstring(l));
            match note.filter(|n| on("HasMapNote") && !n.is_empty()) {
                Some(n) => out.push(format!("Map note: {n}")),
                None => out.push("No map note".into()),
            }
        }
        _ => {}
    }
    out
}

/// The palette's Prefabs: the groups of objects saved, each placed with a
/// click, and how to save one.
fn prefabs_ui(app: &mut Moonglow, ui: &mut egui::Ui) {
    let area = app.palette.area.filter(|a| app.area_views.contains_key(a));
    let chosen = area.and_then(|a| app.area_views.get(&a)).map_or(0, |v| v.selection.len());
    ui.label(
        "A prefab is a group of placed objects kept under a name: a camp, a market stall, a \
         furnished room. It is placed as one, in any area of any module.",
    );
    let save = ui
        .add_enabled(chosen > 0, egui::Button::new("Save Selection as Prefab…"))
        .on_hover_text("The objects selected in the area shown, kept as a prefab")
        .on_disabled_hover_text(
            "Select the objects in an area first (a click, Ctrl+click for more, or a box \
             dragged around them)",
        );
    if save.clicked()
        && let Some(area) = area
    {
        crate::area_view::save_selection_as_prefab(app, area);
    }
    ui.separator();
    let dir = app.prefab_dir.clone();
    // The prefab deleted last can be put back (its file is Moonglow's, not
    // the module's: Edit › Undo doesn't know it).
    if let Some((name, text)) = app.prefab_deleted.clone() {
        let mut keep = true;
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Prefab '{name}' deleted."));
            if ui.button("Undo Delete").on_hover_text("Put the prefab back").clicked() {
                match crate::prefabs::restore(dir.as_deref(), &name, &text) {
                    Ok(()) => app.log.info(format!("Prefab '{name}' put back")),
                    Err(e) => app.log.error(format!("Undo Delete: {e}")),
                }
                keep = false;
            }
            if ui.small_button("✖").on_hover_text("Forget it").clicked() {
                keep = false;
            }
        });
        if !keep {
            app.prefab_deleted = None;
        }
        ui.separator();
    }
    let names = crate::prefabs::list(dir.as_deref());
    if names.is_empty() {
        ui.weak("No prefabs yet. Select objects in an area, then Save Selection as Prefab….");
        return;
    }
    // The prefab Rename… was chosen for, and the name typed so far.
    let renamed_id = egui::Id::new("palette-prefab-rename");
    let mut renamed: Option<(String, String)> = ui.data(|d| d.get_temp(renamed_id));
    if let Some((name, mut to)) = renamed.clone().filter(|(n, _)| names.contains(n)) {
        let mut done = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Rename '{name}' to"));
            let r = ui.add(egui::TextEdit::singleline(&mut to).desired_width(160.0));
            crate::widgets::autofocus(ui, &r);
            let taken = names.iter().any(|n| n != &name && n.eq_ignore_ascii_case(to.trim()));
            let ok = crate::prefabs::valid_name(&to) && !taken;
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.add_enabled(ok, egui::Button::new("Rename")).clicked() || (ok && enter))
                && to.trim() != name
            {
                match crate::prefabs::rename(dir.as_deref(), &name, &to) {
                    Ok(()) => app.log.info(format!("Prefab '{name}' renamed to '{}'", to.trim())),
                    Err(e) => app.log.error(format!("Rename prefab: {e}")),
                }
                done = true;
            } else if ui.button("Cancel").clicked()
                || ui.input(|i| i.key_pressed(egui::Key::Escape))
                || (ok && enter)
            {
                done = true;
            }
            if taken {
                ui.colored_label(ui.visuals().warn_fg_color, "Another prefab has that name");
            } else if !ok && !to.is_empty() {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "Letters, digits, spaces, - and _ only",
                );
            }
        });
        renamed = (!done).then_some((name, to));
        ui.separator();
    } else {
        renamed = None;
    }
    egui::ScrollArea::vertical().id_salt("palette-prefabs").auto_shrink([false, false]).show(
        ui,
        |ui| {
            for name in crate::prefabs::list(dir.as_deref()) {
                let being = renamed.as_ref().is_some_and(|(n, _)| n == &name);
                let r = ui.selectable_label(being, &name).on_hover_ui(|ui| {
                    // What it holds, read while the pointer rests on it.
                    match crate::prefabs::summary(dir.as_deref(), &name) {
                        Ok(holds) => ui.label(holds),
                        Err(e) => ui.colored_label(ui.visuals().error_fg_color, e),
                    };
                    ui.weak("Click, then click in the area to place it; right-click for more");
                });
                if r.clicked() {
                    app.actions.push(Action::PlacePrefab(name.clone()));
                }
                r.context_menu(|ui| {
                    if ui.button("Place").clicked() {
                        app.actions.push(Action::PlacePrefab(name.clone()));
                        ui.close();
                    }
                    if ui.button("Rename…").clicked() {
                        renamed = Some((name.clone(), name.clone()));
                        ui.close();
                    }
                    if ui
                        .button("Delete")
                        .on_hover_text("Undo Delete, above the list, puts it back")
                        .clicked()
                    {
                        match crate::prefabs::delete(dir.as_deref(), &name) {
                            Ok(text) => {
                                app.log.info(format!("Prefab '{name}' deleted"));
                                app.prefab_deleted = Some((name.clone(), text));
                            }
                            Err(e) => app.log.error(format!("Delete prefab: {e}")),
                        }
                        ui.close();
                    }
                });
            }
        },
    );
    ui.data_mut(|d| match renamed {
        Some(v) => {
            d.insert_temp(renamed_id, v);
        }
        None => d.remove::<(String, String)>(renamed_id),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The arrow keys in a tree: Favorites (open) with one blueprint, a
    /// category (open) holding a closed category and two blueprints, and
    /// a closed category after it.
    #[test]
    fn the_arrow_keys_move_through_the_rows_showing() {
        use egui::Key::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp};
        let key = |name: &str| ResKey::parse(name, mg_core::ResType::UTP).unwrap();
        let mut l = Listed::default();
        let favorites = l.branch(Branch::Remembered("Favorites"), None); // row 0
        l.rows.push((favorites, Some(key("chest")))); // 1
        let outer = l.branch(Branch::Category(vec![0]), None); // 2
        let inner = l.branch(Branch::Category(vec![0, 0]), Some(outer)); // 3
        l.rows.push((inner, Some(key("hidden")))); // 4: drawn, its branch closing
        l.rows.push((outer, Some(key("chest")))); // 5
        l.rows.push((outer, Some(key("barrel")))); // 6
        let last = l.branch(Branch::Category(vec![1]), None); // 7
        l.branches[favorites].open = true;
        l.branches[outer].open = true;
        // Down and Up: the rows showing, not those of a closed branch.
        assert_eq!(l.step(3, ArrowDown), Step::To(5));
        assert_eq!(l.step(5, ArrowUp), Step::To(3));
        assert_eq!(l.step(6, ArrowDown), Step::To(7));
        assert_eq!(l.step(7, ArrowDown), Step::Stay);
        assert_eq!(l.step(0, ArrowUp), Step::Stay);
        // Left: a blueprint to its branch, an open branch closes, a closed
        // one goes to the branch it is in (none: it stays).
        assert_eq!(l.step(6, ArrowLeft), Step::To(2));
        assert_eq!(l.step(2, ArrowLeft), Step::Fold(outer, false));
        assert_eq!(l.step(3, ArrowLeft), Step::To(2));
        assert_eq!(l.step(7, ArrowLeft), Step::Stay);
        assert_eq!(l.step(1, ArrowLeft), Step::To(0));
        // Right: a closed branch opens, an open one goes into it, a
        // blueprint stays.
        assert_eq!(l.step(3, ArrowRight), Step::Fold(inner, true));
        assert_eq!(l.step(2, ArrowRight), Step::To(3));
        assert_eq!(l.step(0, ArrowRight), Step::To(1));
        assert_eq!(l.step(5, ArrowRight), Step::Stay);
        assert_eq!(l.step(7, ArrowRight), Step::Fold(last, true));
        // A blueprint is found in its category before Favorites; a row by
        // its branch.
        assert_eq!(l.blueprint(key("chest")), Some(5));
        let at = |branch, blueprint| At {
            kind: BlueprintKind::Placeable,
            custom: false,
            branch,
            blueprint,
        };
        assert_eq!(l.find(&at(Branch::Remembered("Favorites"), Some(key("chest")))), Some(1));
        assert_eq!(l.find(&at(Branch::Category(vec![0, 0]), None)), Some(3));
        assert!(!l.shows(4) && l.shows(3));
    }

    #[test]
    fn a_blueprints_row_names_what_is_asked_for() {
        let r = ResRef::from_str("nw_goblina").unwrap();
        assert_eq!(blueprint_label("Goblin", None, None), "Goblin");
        assert_eq!(blueprint_label("Goblin", Some(r), None), "Goblin (nw_goblina)");
        assert_eq!(blueprint_label("Goblin", None, Some(0.5)), "Goblin  (CR 0.5)");
        assert_eq!(blueprint_label("Goblin", Some(r), Some(1.0)), "Goblin (nw_goblina)  (CR 1)");
    }

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
