//! The Appearance Gallery: every appearance of a kind as a picture, in a
//! grid, in a tab of its own: placeables (placeables.2da's models),
//! creatures (appearance.2da: the creature given appearances as it is in
//! each, its head, colors and what it wears, else a plain body) and doors
//! (genericdoors.2da). Opened from Tools, the palette's All Appearances…,
//! and Gallery… beside the appearance in a placeable's, creature's or
//! door's Properties. A click gives the appearance to the object whose
//! Properties opened it, else to the objects of that kind selected in the
//! area.

use mg_core::ResType;
use mg_edit::{Command, Edit, GffPath};
use mg_gff::Value;
use mg_resman::ResKey;
use mg_rules::ChoiceColumns;

use crate::model_view::Pictured;
use crate::{Action, Moonglow};

/// An appearance dragged from the gallery (dropped in an area view, a
/// placeable with it is placed there): its placeables.2da row and name.
#[derive(Debug, Clone, PartialEq)]
pub struct DraggedAppearance {
    pub row: usize,
    pub name: String,
}

/// What a gallery shows the appearances of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Kind {
    #[default]
    Placeable,
    Creature,
    Door,
}

impl Kind {
    pub(crate) const ALL: [Kind; 3] = [Kind::Placeable, Kind::Creature, Kind::Door];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Kind::Placeable => "Placeables",
            Kind::Creature => "Creatures",
            Kind::Door => "Doors",
        }
    }

    /// One, and several, of what it gives appearances to.
    fn noun(self, n: usize) -> String {
        let one = match self {
            Kind::Placeable => "placeable",
            Kind::Creature => "creature",
            Kind::Door => "door",
        };
        if n == 1 { format!("1 {one}") } else { format!("{n} {one}s") }
    }

    /// Its table, and the columns its rows are named by.
    fn table(self) -> (&'static str, ChoiceColumns<'static>) {
        match self {
            Kind::Placeable => {
                ("placeables", ChoiceColumns { name: Some("StrRef"), label: Some("Label") })
            }
            Kind::Creature => {
                ("appearance", ChoiceColumns { name: Some("STRING_REF"), label: Some("LABEL") })
            }
            Kind::Door => {
                ("genericdoors", ChoiceColumns { name: Some("Name"), label: Some("Label") })
            }
        }
    }

    /// The field an object keeps its appearance in.
    fn field(self) -> &'static str {
        match self {
            Kind::Placeable => "Appearance",
            Kind::Creature => "Appearance_Type",
            Kind::Door => "GenericType_New",
        }
    }

    /// The fields set to give an object row `row`.
    fn fields(self, row: i64) -> Vec<(&'static str, Value)> {
        match self {
            Kind::Placeable => vec![("Appearance", Value::Dword(row as u32))],
            Kind::Creature => vec![("Appearance_Type", Value::Word(row as u16))],
            // (A generic door: the tileset's own type 0, and the old byte.)
            Kind::Door => {
                let mut fields = vec![
                    ("Appearance", Value::Dword(0)),
                    ("GenericType_New", Value::Dword(row as u32)),
                ];
                if (0..=255).contains(&row) {
                    fields.push(("GenericType", Value::Byte(row as u8)));
                }
                fields
            }
        }
    }

    /// Why row `row` has no model to draw, if it has none
    /// ([`mg_rules::appearance`], as the Properties and Verify say it).
    fn problem(self, game: &mg_rules::GameData, row: usize) -> Option<String> {
        let mut object = mg_gff::Struct::default();
        for (label, value) in self.fields(row as i64) {
            object.set(label, value);
        }
        game.appearance_problem(self.object(), &object).map(|p| p.to_string())
    }

    /// The kind's icon, a picture's while it is made.
    fn icon(self) -> mg_uikit::Icon {
        crate::icons::object(self.object())
    }

    fn object(self) -> mg_area::ObjectKind {
        match self {
            Kind::Placeable => mg_area::ObjectKind::Placeable,
            Kind::Creature => mg_area::ObjectKind::Creature,
            Kind::Door => mg_area::ObjectKind::Door,
        }
    }

    /// What pictures row `row`: its model, or a creature's look: the
    /// creature the gallery gives appearances to as it is (`dressed`,
    /// [`Thumbnails::creature`](crate::model_view::Thumbnails::creature)),
    /// else a plain body.
    fn pictured(self, table: &mg_2da::TwoDa, row: usize, dressed: Option<u64>) -> Option<Pictured> {
        match self {
            Kind::Creature => Some(match dressed {
                Some(id) => Pictured::Creature(id, row as u16),
                None => Pictured::Look(mg_preview::CreatureLook::new(row as u16)),
            }),
            Kind::Placeable | Kind::Door => {
                let model = table.get(row, "ModelName")?.to_ascii_lowercase();
                ResKey::parse(&model, ResType::MDL).map(Pictured::Resource)
            }
        }
    }
}

/// A gallery's state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gallery {
    pub kind: Kind,
    pub filter: String,
    /// Scrolled to the current appearance once, when it opens.
    placed: bool,
    /// The object it was opened for (its document, and where in it): its
    /// Properties' Gallery…. Else the area's selection is given the
    /// appearances.
    pub target: Option<(ResKey, GffPath)>,
}

impl Gallery {
    /// A gallery for one object, opening at its appearance.
    pub(crate) fn of(kind: Kind, key: ResKey, path: GffPath) -> Gallery {
        Gallery { kind, target: Some((key, path)), ..Default::default() }
    }
}

/// The side of a gallery's pictures, points: Options keep what the slider
/// leaves.
pub(crate) fn tile_side(app: &Moonglow) -> f32 {
    f32::from(app.settings.gallery_tile.unwrap_or(DEFAULT_TILE)).clamp(MIN_TILE, MAX_TILE)
}

const DEFAULT_TILE: u16 = 128;
const MIN_TILE: f32 = 64.0;
const MAX_TILE: f32 = 220.0;

/// The slider for the pictures' size (the palette's Gallery has it too).
pub(crate) fn size_slider(app: &mut Moonglow, ui: &mut egui::Ui) {
    let side = tile_side(app);
    if let Some(side) = crate::widgets::picture_size_slider(ui, side, MIN_TILE..=MAX_TILE, None) {
        app.settings.gallery_tile = Some(side);
    }
}

/// What was done to the gallery's pictures this frame: the row clicked,
/// double-clicked, or given to the selection from its menu.
#[derive(Debug, Default)]
struct Clicked {
    click: Option<i64>,
    double: Option<i64>,
    give: Option<i64>,
}

/// The grid: Find, the pictures' size, and the kind's rows as pictures,
/// opening at `current` (the row chosen, highlighted; or `taking`, the
/// one taken to place). What was clicked; `give`: whether a picture's menu
/// offers to give it to the selection.
fn grid(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    state: &mut Gallery,
    (current, taking): (Option<i64>, Option<i64>),
    dressed: Option<u64>,
    give: bool,
) -> Option<Clicked> {
    let kind = state.kind;
    let game = app.game.clone()?;
    let (name, cols) = kind.table();
    let table = game.table(name).ok()?;
    // (The table's rows as the game data keeps them, not a copy a frame;
    // creatures by name, sorted once: the table's order is no one's.)
    let mut all = game.choice_list(name, cols).ok()?;
    if kind == Kind::Creature {
        all = crate::blueprint::by_name_kept(ui.ctx(), "gallery", &all);
    }
    ui.horizontal(|ui| {
        let field = ui.id().with("gallery-filter-field");
        crate::widgets::SearchField::new(field, &mut state.filter)
            .hint("Find")
            .width(180.0)
            .find_over()
            .show(ui);
        size_slider(app, ui);
        ui.weak(format!("{} appearances ({name}.2da)", all.len()));
    });
    let side = tile_side(app);
    let found = crate::blueprint::found_kept(ui.ctx(), "gallery", &all, &state.filter);
    let rows: Vec<&mg_rules::Choice> = found.iter().map(|&i| &all[i]).collect();
    let ready = &app.thumbnails;
    let (mut shown, mut pick) = (Vec::new(), Clicked::default());
    let icon = kind.icon();
    let id = egui::Id::new(("appearance-gallery", kind));
    let keys = !crate::shell::keys_blocked(ui.ctx());
    let at = current.and_then(|now| rows.iter().position(|c| c.row as i64 == now));
    let grid = mg_uikit::gallery::GalleryGrid::new(id, rows.len(), side)
        .cursor(at)
        .keys_allowed(keys)
        .name(kind.name());
    let area = egui::ScrollArea::vertical().id_salt(kind).auto_shrink([false, false]);
    let shown_area = area.show(ui, |ui| {
        let out = grid.show(ui, |ui, i, rect, tile, r, cell| {
            let c = rows[i];
            let key = kind.pictured(&table, c.row, dressed);
            let made = match key {
                Some(k) => ready.made(k),
                None => Some(None),
            };
            let chosen = current == Some(c.row as i64) || taking == Some(c.row as i64);
            // (Marked where it has no model: a builder chose such rows and
            // saw nothing say why.)
            let problem = kind.problem(&game, c.row);
            mg_uikit::gallery::GalleryCell::new(&c.text, crate::widgets::cell_picture(made))
                .kind(icon)
                .selected(chosen)
                .no_model(problem.is_some())
                .paint(ui, rect, tile, cell, r);
            let mut about = format!("{} ({name}.2da row {})", c.text, c.row);
            if let Some(p) = &problem {
                about = format!("{about}\n{p}");
            }
            let r = r.clone().on_hover_text(about);
            if r.double_clicked() {
                pick.double = Some(c.row as i64);
            } else if r.clicked() {
                pick.click = Some(c.row as i64);
            }
            if give {
                mg_uikit::context_menu(&r, |ui| {
                    if ui.button("Give to the Selection").clicked() {
                        pick.give = Some(c.row as i64);
                        ui.close();
                    }
                });
            }
            // Dragged into an area's view: a placeable of it there.
            if kind == Kind::Placeable && r.drag_started() {
                r.dnd_set_drag_payload(DraggedAppearance { row: c.row, name: c.text.clone() });
            }
        });
        // (Enter or Space on the keys' picture: its click.)
        if let Some(i) = out.activated {
            pick.click = Some(rows[i].row as i64);
        }
        // (Those in sight and a row ahead: kept while they are, and made if
        // they aren't yet, a few a frame.)
        for i in out.ahead.clone() {
            shown.extend(kind.pictured(&table, rows[i].row, dressed));
        }
        out
    });
    // (Opened at the row chosen, once, where the grid put it.)
    if !state.placed {
        state.placed = match at {
            Some(at) => {
                crate::widgets::open_gallery_at(ui.ctx(), &shown_area, &shown_area.inner, at)
            }
            None => true,
        };
    }
    app.thumbnails.show(shown);
    Some(pick)
}

/// The objects a gallery gives appearances to: their document, their
/// places in its list, the first one's appearance and (a creature) its
/// fields to picture it by
/// ([`Thumbnails::creature`](crate::model_view::Thumbnails::creature)),
/// and whether they all have the first one's appearance.
type Chosen = (ResKey, Vec<usize>, Option<i64>, Option<u64>, bool);

/// The objects of the gallery's kind selected in the area shown (in its
/// GIT).
fn selected(app: &mut Moonglow, kind: Kind) -> Option<Chosen> {
    let area = app.palette.area?;
    let view = app.area_views.get(&area)?;
    let object = kind.object();
    let chosen: Vec<usize> =
        view.selection.iter().filter(|(k, _)| *k == object).map(|&(_, i)| i).collect();
    let git = ResKey::new(area, ResType::GIT);
    let first = *chosen.first()?;
    let doc = app.ws.as_mut()?.doc(&git).ok()?;
    let list = doc.root.list(object.list())?;
    let of = list.get(first)?.clone();
    let now = of.integer(kind.field());
    let alike = chosen.iter().all(|&i| list.get(i).and_then(|s| s.integer(kind.field())) == now);
    let dressed = (kind == Kind::Creature).then(|| app.thumbnails.creature(&of));
    Some((git, chosen, now, dressed, alike))
}

/// The gallery's tab: to look through, and to give an appearance with a
/// click: to the object whose Properties opened it (while it is there),
/// else to the objects of its kind selected in the area.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui) {
    let mut state = app.placeable_gallery.take().unwrap_or_default();
    // Placeables, creatures or doors.
    ui.horizontal(|ui| {
        for kind in Kind::ALL {
            if ui.selectable_label(state.kind == kind, kind.name()).clicked() && state.kind != kind
            {
                state = Gallery { kind, ..Default::default() };
            }
        }
    });
    let kind = state.kind;
    // The one object it was opened for, as it is now.
    let one = state.target.clone().and_then(|(key, path)| {
        let doc = app.ws.as_mut()?.doc(&key).ok()?;
        let of = path.get(&doc.root)?.clone();
        let dressed = (kind == Kind::Creature).then(|| app.thumbnails.creature(&of));
        Some((key, path, of.integer(kind.field()), dressed))
    });
    if one.is_none() {
        state.target = None;
    }
    // (Opened on its own, for placeables: a click takes one to place.)
    let places = one.is_none() && kind == Kind::Placeable;
    let chosen = if one.is_none() { selected(app, kind) } else { None };
    let (current, dressed) = match (&one, &chosen) {
        (Some((_, _, now, dressed)), _) => (*now, *dressed),
        (None, Some((_, _, now, dressed, _))) => (*now, *dressed),
        _ => (None, None),
    };
    // (Nothing to give where every object it is given to has it: the
    // first's alone was looked at, and a click on its appearance did
    // nothing for the others selected with it.)
    let alike = chosen.as_ref().is_none_or(|c| c.4);
    ui.horizontal_wrapped(|ui| match (&one, &chosen) {
        (Some((key, path, ..)), _) => {
            let what = if path.0.is_empty() { key.to_string() } else { "the placed object".into() };
            ui.label(format!("A click gives {what} the appearance"));
            if ui
                .small_button("Use the Selection")
                .on_hover_text("Give appearances to the objects selected in the area instead")
                .clicked()
            {
                state.target = None;
            }
        }
        // (Placeables, opened on their own: a click takes one to place, as
        // the palette's does; the selection is given one from a picture's
        // menu.)
        (None, Some((_, list, ..))) if places => {
            let them = if list.len() == 1 { "it" } else { "them" };
            ui.label(format!(
                "A click takes a placeable to place; right click to give the {} selected {them}",
                kind.noun(list.len())
            ));
        }
        (None, Some((_, list, ..))) => {
            let them = if list.len() == 1 { "it" } else { "them" };
            ui.label(format!(
                "{} selected in the area: a click gives {them} the appearance",
                kind.noun(list.len())
            ));
        }
        _ if places => {
            ui.weak("A click takes a placeable to place in an area; a double click makes a blueprint of it");
        }
        _ => {
            ui.weak(format!(
                "Select {} in an area, then click a picture to give them its appearance",
                kind.name().to_lowercase()
            ));
        }
    });
    let taking = app.palette.placing_appearance.as_ref().map(|a| a.row as i64);
    let clicked = grid(app, ui, &mut state, (current, taking), dressed, places && chosen.is_some())
        .unwrap_or_default();
    // A double click: a blueprint of it, by its kind's wizard, its
    // Properties opened after (as a palette's blueprint opens them).
    if let Some(row) = clicked.double {
        start_wizard(app, kind, row);
    }
    // Opened on its own, a placeable's click takes it to place (again:
    // put down).
    if places && let Some(row) = clicked.click {
        let name = (app.game.as_deref())
            .and_then(|g| g.choices(kind.table().0, kind.table().1).ok())
            .and_then(|all| all.into_iter().find(|c| c.row as i64 == row).map(|c| c.text))
            .unwrap_or_else(|| row.to_string());
        app.palette.placing_appearance = match taking == Some(row) {
            true => None,
            false => Some(DraggedAppearance { row: row as usize, name }),
        };
    }
    let pick = match places {
        true => clicked.give,
        false => clicked.click.or(clicked.give),
    }
    .filter(|row| Some(*row) != current || !alike);
    let set = |key: ResKey, path: GffPath, row: i64| -> Vec<Edit> {
        kind.fields(row)
            .into_iter()
            .map(|(label, value)| Edit::SetField {
                key,
                path: path.clone(),
                label: label.into(),
                value: Some(value),
            })
            .collect()
    };
    let mut edits: Vec<Edit> = match (pick, one, chosen) {
        (Some(row), Some((key, path, ..)), _) => set(key, path, row),
        (Some(row), None, Some((git, list, ..))) => list
            .iter()
            .flat_map(|&i| set(git, GffPath::root().item(kind.object().list(), i), row))
            .collect(),
        _ => Vec::new(),
    };
    if !edits.is_empty() {
        crate::blueprint::with_a_body(app, &mut edits);
        app.actions.push(Action::Apply(Command::new("Appearance", edits)));
    }
    app.placeable_gallery = Some(state);
}

/// A blueprint of appearance `row` of `kind`, made by its kind's wizard
/// (the Placeable or Door Wizard, the Creature Wizard), named for it, its
/// Properties opened after.
fn start_wizard(app: &mut Moonglow, kind: Kind, row: i64) {
    use mg_module::palette::BlueprintKind;
    if kind == Kind::Creature {
        let appearance = row as u16;
        let w = crate::creature_wizard::CreatureWizard { appearance, ..Default::default() };
        app.creature_wizard = Some(w);
        return;
    }
    let name = (app.game.as_deref())
        .and_then(|g| g.choices(kind.table().0, kind.table().1).ok())
        .and_then(|all| all.into_iter().find(|c| c.row as i64 == row).map(|c| c.text))
        .unwrap_or_default();
    let of = if kind == Kind::Door { BlueprintKind::Door } else { BlueprintKind::Placeable };
    let w = crate::blueprint_wizard::BlueprintWizard::new(of).given(kind.fields(row), &name);
    app.blueprint_wizard = Some(w);
}
