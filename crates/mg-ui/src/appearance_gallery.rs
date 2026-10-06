//! The Placeable Gallery: every row of placeables.2da as a picture (its
//! model), in a grid, in a tab of its own (Tools, the palette's All
//! Appearances…, and Gallery… beside Appearance Type in a placeable's
//! Properties). A click gives the appearance to the placeable whose
//! Properties opened it, else to the placeables selected in the area.

use mg_core::ResType;
use mg_edit::{Command, Edit, GffPath};
use mg_gff::Value;
use mg_resman::ResKey;
use mg_rules::ChoiceColumns;

use crate::{Action, Moonglow};

/// A gallery's state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gallery {
    pub filter: String,
    /// Scrolled to the current appearance once, when it opens.
    placed: bool,
    /// The placeable it was opened for (its document, and where in it):
    /// its Properties' Gallery…. Else the area's selection is given the
    /// appearances.
    pub target: Option<(ResKey, GffPath)>,
}

impl Gallery {
    /// A gallery for one placeable, opening at its appearance.
    pub(crate) fn of(key: ResKey, path: GffPath) -> Gallery {
        Gallery { target: Some((key, path)), ..Default::default() }
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

/// The grid: Find, the pictures' size, and placeables.2da's rows as
/// pictures, opening at `current` (the row chosen, highlighted). The row
/// clicked.
pub(crate) fn grid(
    app: &mut Moonglow,
    ui: &mut egui::Ui,
    state: &mut Gallery,
    current: Option<i64>,
) -> Option<i64> {
    let game = app.game.clone()?;
    let table = game.table("placeables").ok()?;
    let cols = ChoiceColumns { name: Some("StrRef"), label: Some("Label") };
    let all = game.choices("placeables", cols).ok()?;
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.filter).hint_text("Find").desired_width(180.0),
        );
        size_slider(app, ui);
        ui.weak(format!("{} appearances (placeables.2da)", all.len()));
    });
    let side = tile_side(app);
    let needle = state.filter.to_lowercase();
    let rows: Vec<&mg_rules::Choice> = all
        .iter()
        .filter(|c| needle.is_empty() || c.text.to_lowercase().contains(&needle))
        .collect();
    let ready = app.thumbnails.all();
    let (mut wanted, mut pick) = (Vec::new(), None);
    let gap = 4.0;
    // (The room less the scroll bar's.)
    let room = ui.available_width() - ui.spacing().scroll.bar_width - 10.0;
    let (per_row, side) = fitted(room, side, gap);
    let height = side + 18.0 + gap;
    let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
    if !state.placed {
        state.placed = true;
        if let Some(at) = current.and_then(|now| rows.iter().position(|c| c.row as i64 == now)) {
            area = area.vertical_scroll_offset((at / per_row) as f32 * height);
        }
    }
    area.show_rows(ui, side + 18.0, rows.len().div_ceil(per_row), |ui, lines| {
        ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
        for line in lines {
            ui.horizontal(|ui| {
                for c in rows.iter().skip(line * per_row).take(per_row) {
                    let model = table.get(c.row, "ModelName").unwrap_or_default();
                    let key = ResKey::parse(&model.to_ascii_lowercase(), ResType::MDL);
                    let made = match key {
                        Some(k) => ready.get(&k).copied(),
                        None => Some(None),
                    };
                    let chosen = current == Some(c.row as i64);
                    let (r, seen) = crate::widgets::picture_tile(ui, side, &c.text, chosen, made);
                    if let (true, None, Some(k)) = (seen, made, key) {
                        wanted.push(k);
                    }
                    let r = r.on_hover_text(format!("{} (row {}, {model})", c.text, c.row));
                    if r.clicked() {
                        pick = Some(c.row as i64);
                    }
                }
            });
        }
    });
    if !wanted.is_empty() {
        for key in wanted.into_iter().take(crate::palette_view::GALLERY_PER_FRAME) {
            crate::model_view::thumbnail(app, key);
        }
        ui.ctx().request_repaint();
    }
    pick
}

/// The placeables selected in the area shown: the area's GIT, their places
/// in its list, and the first one's appearance.
fn selected(app: &mut Moonglow) -> Option<(ResKey, Vec<usize>, Option<i64>)> {
    let area = app.palette.area?;
    let view = app.area_views.get(&area)?;
    let chosen: Vec<usize> = view
        .selection
        .iter()
        .filter(|(k, _)| *k == mg_area::ObjectKind::Placeable)
        .map(|&(_, i)| i)
        .collect();
    let git = ResKey::new(area, ResType::GIT);
    let first = *chosen.first()?;
    let doc = app.ws.as_mut()?.doc(&git).ok()?;
    let appearance = doc.root.list("Placeable List")?.get(first)?.integer("Appearance");
    Some((git, chosen, appearance))
}

/// The Placeable Gallery's tab: to look through, and to give an
/// appearance with a click: to the placeable whose Properties opened it
/// (while it is there), else to the placeables selected in the area.
pub(crate) fn ui(app: &mut Moonglow, ui: &mut egui::Ui) {
    let mut state = app.placeable_gallery.take().unwrap_or_default();
    // The one placeable it was opened for, as it is now.
    let one = state.target.clone().and_then(|(key, path)| {
        let doc = app.ws.as_mut()?.doc(&key).ok()?;
        let now = path.get(&doc.root)?.integer("Appearance");
        Some((key, path, now))
    });
    if one.is_none() {
        state.target = None;
    }
    let chosen = if one.is_none() { selected(app) } else { None };
    let current = match (&one, &chosen) {
        (Some((_, _, now)), _) => *now,
        (None, Some((_, _, now))) => *now,
        _ => None,
    };
    ui.horizontal_wrapped(|ui| match (&one, &chosen) {
        (Some((key, path, _)), _) => {
            let what = if path.0.is_empty() { key.to_string() } else { "the placed object".into() };
            ui.label(format!("A click gives {what} the appearance"));
            if ui
                .small_button("Use the Selection")
                .on_hover_text("Give appearances to the placeables selected in the area instead")
                .clicked()
            {
                state.target = None;
            }
        }
        (None, Some((_, list, _))) => {
            let (n, them) = match list.len() {
                1 => ("1 placeable".to_string(), "it"),
                n => (format!("{n} placeables"), "them"),
            };
            ui.label(format!("{n} selected in the area: a click gives {them} the appearance"));
        }
        _ => {
            ui.weak(
                "Select placeables in an area, then click a picture to give them its appearance",
            );
        }
    });
    let pick = grid(app, ui, &mut state, current).filter(|row| Some(*row) != current);
    let set = |key: ResKey, path: GffPath, row: i64| Edit::SetField {
        key,
        path,
        label: "Appearance".into(),
        value: Some(Value::Dword(row as u32)),
    };
    let edits: Vec<Edit> = match (pick, one, chosen) {
        (Some(row), Some((key, path, _)), _) => vec![set(key, path, row)],
        (Some(row), None, Some((git, list, _))) => {
            list.iter().map(|&i| set(git, GffPath::root().item("Placeable List", i), row)).collect()
        }
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
