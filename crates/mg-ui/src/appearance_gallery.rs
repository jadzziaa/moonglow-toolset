//! The Appearance Gallery: every appearance of a kind as a picture, in a
//! grid, in a tab of its own: placeables (placeables.2da's models),
//! creatures (appearance.2da, a plain body of each) and doors
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
pub(crate) struct DraggedAppearance {
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

    fn object(self) -> mg_area::ObjectKind {
        match self {
            Kind::Placeable => mg_area::ObjectKind::Placeable,
            Kind::Creature => mg_area::ObjectKind::Creature,
            Kind::Door => mg_area::ObjectKind::Door,
        }
    }

    /// What pictures row `row`: its model, or a creature's look.
    fn pictured(self, table: &mg_2da::TwoDa, row: usize, gender: u8) -> Option<Pictured> {
        match self {
            Kind::Creature => {
                let mut look = mg_preview::CreatureLook::new(row as u16);
                look.gender = gender;
                Some(Pictured::Look(look))
            }
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

/// How many pictures of at least `side` points, `gap` apart, go across
/// `room`, and the side that makes them fill it (a row of pictures of the
/// size asked for left a strip of the window empty beside them).
pub(crate) fn fitted(room: f32, side: f32, gap: f32) -> (usize, f32) {
    let per_row = (((room + gap) / (side + gap)).floor() as usize).max(1);
    let fills = ((room - gap * (per_row - 1) as f32) / per_row as f32).floor();
    // (One picture alone isn't blown up to a whole window's width.)
    (per_row, fills.clamp(side.min(room.max(MIN_TILE)), side * 2.0))
}

const DEFAULT_TILE: u16 = 128;
const MIN_TILE: f32 = 64.0;
const MAX_TILE: f32 = 220.0;

/// The slider for the pictures' size (the palette's Gallery has it too).
pub(crate) fn size_slider(app: &mut Moonglow, ui: &mut egui::Ui) {
    let mut side = tile_side(app);
    let slider = egui::Slider::new(&mut side, MIN_TILE..=MAX_TILE).show_value(false);
    if ui.add(slider).on_hover_text("How large the pictures are").changed() {
        app.settings.gallery_tile = Some(side.round() as u16);
    }
}

/// The grid: Find, the pictures' size, and the kind's rows as pictures,
/// opening at `current` (the row chosen, highlighted). The row clicked.
pub(crate) fn grid(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    state: &mut Gallery,
    current: Option<i64>,
    gender: u8,
) -> Option<i64> {
    let kind = state.kind;
    let game = app.game.clone()?;
    let (name, cols) = kind.table();
    let table = game.table(name).ok()?;
    let mut all = game.choices(name, cols).ok()?;
    // (Creatures by name: the table's order is no one's.)
    if kind == Kind::Creature {
        all = mg_rules::by_name(all);
    }
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.filter).hint_text("Find").desired_width(180.0),
        );
        size_slider(app, ui);
        ui.weak(format!("{} appearances ({name}.2da)", all.len()));
    });
    let side = tile_side(app);
    let needle = state.filter.to_lowercase();
    let rows: Vec<&mg_rules::Choice> = all
        .iter()
        .filter(|c| needle.is_empty() || c.text.to_lowercase().contains(&needle))
        .collect();
    let ready = app.thumbnails.every();
    let (mut wanted, mut pick) = (Vec::new(), None);
    let gap = 4.0;
    // (The room less the scroll bar's.)
    let room = ui.available_width() - ui.spacing().scroll.bar_width - 10.0;
    let (per_row, side) = fitted(room, side, gap);
    let tall = side + crate::widgets::tile_label_height(ui);
    let height = tall + gap;
    let mut area = egui::ScrollArea::vertical().id_salt(kind).auto_shrink([false, false]);
    if !state.placed {
        state.placed = true;
        if let Some(at) = current.and_then(|now| rows.iter().position(|c| c.row as i64 == now)) {
            area = area.vertical_scroll_offset((at / per_row) as f32 * height);
        }
    }
    area.show_rows(ui, tall, rows.len().div_ceil(per_row), |ui, lines| {
        ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
        for line in lines {
            ui.horizontal(|ui| {
                for c in rows.iter().skip(line * per_row).take(per_row) {
                    let key = kind.pictured(&table, c.row, gender);
                    let made = match key {
                        Some(k) => ready.get(&k).copied(),
                        None => Some(None),
                    };
                    let chosen = current == Some(c.row as i64);
                    let (r, seen) = crate::widgets::picture_tile(ui, side, &c.text, chosen, made);
                    if let (true, None, Some(k)) = (seen, made, key) {
                        wanted.push(k);
                    }
                    let r = r.on_hover_text(format!("{} ({name}.2da row {})", c.text, c.row));
                    if r.clicked() {
                        pick = Some(c.row as i64);
                    }
                    // Dragged into an area's view: a placeable of it there.
                    if kind == Kind::Placeable && r.drag_started() {
                        r.dnd_set_drag_payload(DraggedAppearance {
                            row: c.row,
                            name: c.text.clone(),
                        });
                    }
                }
            });
        }
    });
    if !wanted.is_empty() {
        for key in wanted.into_iter().take(crate::palette_view::GALLERY_PER_FRAME) {
            crate::model_view::thumbnail_of(app, key);
        }
        ui.ctx().request_repaint();
    }
    pick
}

/// The objects of the gallery's kind selected in the area shown: the
/// area's GIT, their places in its list, and the first one's appearance
/// and gender.
fn selected(app: &mut Moonglow, kind: Kind) -> Option<(ResKey, Vec<usize>, Option<i64>, u8)> {
    let area = app.palette.area?;
    let view = app.area_views.get(&area)?;
    let object = kind.object();
    let chosen: Vec<usize> =
        view.selection.iter().filter(|(k, _)| *k == object).map(|&(_, i)| i).collect();
    let git = ResKey::new(area, ResType::GIT);
    let first = *chosen.first()?;
    let doc = app.ws.as_mut()?.doc(&git).ok()?;
    let of = doc.root.list(object.list())?.get(first)?;
    let gender = of.integer("Gender").unwrap_or(0).clamp(0, 1) as u8;
    Some((git, chosen, of.integer(kind.field()), gender))
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
        let of = path.get(&doc.root)?;
        let gender = of.integer("Gender").unwrap_or(0).clamp(0, 1) as u8;
        Some((key, path, of.integer(kind.field()), gender))
    });
    if one.is_none() {
        state.target = None;
    }
    let chosen = if one.is_none() { selected(app, kind) } else { None };
    let (current, gender) = match (&one, &chosen) {
        (Some((_, _, now, gender)), _) => (*now, *gender),
        (None, Some((_, _, now, gender))) => (*now, *gender),
        _ => (None, 0),
    };
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
        (None, Some((_, list, ..))) => {
            let them = if list.len() == 1 { "it" } else { "them" };
            ui.label(format!(
                "{} selected in the area: a click gives {them} the appearance",
                kind.noun(list.len())
            ));
        }
        _ => {
            ui.weak(format!(
                "Select {} in an area, then click a picture to give them its appearance",
                kind.name().to_lowercase()
            ));
        }
    });
    let pick = grid(app, ui, &mut state, current, gender).filter(|row| Some(*row) != current);
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
    let edits: Vec<Edit> = match (pick, one, chosen) {
        (Some(row), Some((key, path, ..)), _) => set(key, path, row),
        (Some(row), None, Some((git, list, ..))) => list
            .iter()
            .flat_map(|&i| set(git, GffPath::root().item(kind.object().list(), i), row))
            .collect(),
        _ => Vec::new(),
    };
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new("Appearance", edits)));
    }
    app.placeable_gallery = Some(state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_fill_the_width() {
        // 500 points, pictures of 128 with 4 between: three fit, and grow
        // to fill it.
        let (per_row, side) = fitted(500.0, 128.0, 4.0);
        assert_eq!(per_row, 3);
        assert!(3.0 * side + 2.0 * 4.0 <= 500.0 && 3.0 * side + 2.0 * 4.0 > 496.0, "{side}");
        // Narrower than one: the one, as wide as the room.
        assert_eq!(fitted(100.0, 128.0, 4.0), (1, 100.0));
        // A lone picture isn't blown up past twice its size.
        assert_eq!(fitted(250.0, 128.0, 4.0).0, 1);
    }
}
