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

    /// Why row `row` has no model to draw, if it has none
    /// ([`mg_rules::appearance`], as the Properties and Verify say it).
    fn problem(self, game: &mg_rules::GameData, row: usize) -> Option<String> {
        let mut object = mg_gff::Struct::default();
        for (label, value) in self.fields(row as i64) {
            object.set(label, value);
        }
        game.appearance_problem(self.object(), &object).map(|p| p.to_string())
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

/// How many pictures of at least `side` points, `gap` apart, go across
/// `room`, and the side that makes them fill it (a row of pictures of the
/// size asked for left a strip of the window empty beside them).
pub(crate) fn fitted(room: f32, side: f32, gap: f32) -> (usize, f32) {
    let per_row = (((room + gap) / (side + gap)).floor() as usize).max(1);
    let fills = ((room - gap * (per_row - 1) as f32) / per_row as f32).floor();
    // (One picture alone isn't blown up to a whole window's width.)
    (per_row, fills.clamp(side.min(room.max(MIN_TILE)), side * 2.0))
}

/// Pictures in rows that wrap, as the Galleries lay theirs out (the
/// blueprint palettes', the tiles palette's tiles and groups): as many of
/// at least `asked` points across as fit, grown to fill the row, `GAP`
/// apart; `add` lays them out, given their side. The row is of the pane's
/// visible width, and `ui` is narrowed to it: a pane once wider than it
/// shows, for a row that didn't wrap, would keep that width and the
/// pictures run past its edge.
pub(crate) fn picture_rows<R>(
    ui: &mut egui::Ui,
    asked: f32,
    add: impl FnOnce(&mut egui::Ui, f32) -> R,
) -> R {
    const GAP: f32 = 4.0;
    let seen = ui.clip_rect().right() - ui.cursor().left();
    let room = ui.available_width().min(seen) - 1.0;
    let side = fitted(room, asked, GAP).1;
    ui.set_max_width(room);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(GAP, GAP);
        add(ui, side)
    })
    .inner
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
    dressed: Option<u64>,
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
    let needle = mg_core::text::Needle::new(&state.filter);
    let rows: Vec<&mg_rules::Choice> = all.iter().filter(|c| needle.finds(&c.text)).collect();
    let ready = &app.thumbnails;
    let (mut shown, mut pick) = (Vec::new(), None);
    let gap = 4.0;
    // (The room less the scroll bar's.)
    let room = ui.available_width() - ui.spacing().scroll.bar_width - 10.0;
    let (per_row, side) = fitted(room, side, gap);
    let tall = side + crate::widgets::tile_label_height(ui);
    // A row and the space after it, as `show_rows` steps them: the scroll
    // area's spacing, not the `gap` set inside the rows (that put the
    // current one a pixel a row further down than it was).
    let height = tall + ui.spacing().item_spacing.y;
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
                    let key = kind.pictured(&table, c.row, dressed);
                    let made = match key {
                        Some(k) => ready.made(k),
                        None => Some(None),
                    };
                    let chosen = current == Some(c.row as i64);
                    let (r, seen) = crate::widgets::picture_tile(ui, side, &c.text, chosen, made);
                    if let (true, Some(k)) = (seen, key) {
                        shown.push(k);
                    }
                    // (Marked where it has no model: a builder chose such
                    // rows and saw nothing say why.)
                    let problem = if seen { kind.problem(&game, c.row) } else { None };
                    let mut about = format!("{} ({name}.2da row {})", c.text, c.row);
                    if let Some(p) = &problem {
                        let red = ui.visuals().error_fg_color;
                        let stroke = egui::Stroke::new(1.5, red);
                        ui.painter().rect_stroke(r.rect, 3.0, stroke, egui::StrokeKind::Inside);
                        ui.painter().text(
                            r.rect.left_top() + egui::vec2(5.0, 4.0),
                            egui::Align2::LEFT_TOP,
                            "No model",
                            egui::TextStyle::Body.resolve(ui.style()),
                            red,
                        );
                        about = format!(
                            "{about}
{p}"
                        );
                    }
                    let r = r.on_hover_text(about);
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
    // (Those in sight: kept while they are, and made if they aren't yet,
    // a few a frame.)
    app.thumbnails.show(shown);
    pick
}

/// The objects a gallery gives appearances to: their document, their
/// places in its list, and the first one's appearance and (a creature)
/// its fields to picture it by
/// ([`Thumbnails::creature`](crate::model_view::Thumbnails::creature)).
type Chosen = (ResKey, Vec<usize>, Option<i64>, Option<u64>);

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
    let of = doc.root.list(object.list())?.get(first)?.clone();
    let dressed = (kind == Kind::Creature).then(|| app.thumbnails.creature(&of));
    Some((git, chosen, of.integer(kind.field()), dressed))
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
    let chosen = if one.is_none() { selected(app, kind) } else { None };
    let (current, dressed) = match (&one, &chosen) {
        (Some((_, _, now, dressed)), _) => (*now, *dressed),
        (None, Some((_, _, now, dressed))) => (*now, *dressed),
        _ => (None, None),
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
    let pick = grid(app, ui, &mut state, current, dressed).filter(|row| Some(*row) != current);
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
