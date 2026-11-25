//! What a tileset brings beside its editor (M4's U20, the Tileset board):
//! the Right Dock's **Preview** (the chosen tile's diagram, its corners'
//! terrains and heights round its edges' crossers, its model, orientation
//! and doors in the middle; its doors in a sentence under it; its model in
//! 3D where the panel has the room, and in a tab of its own from the
//! header's icon) over **Textures** (what the model draws); its parts in
//! the **Outliner** (General, Terrains, Crossers, the rules, Tiles, Groups
//! with their counts, a click going to the page and the row); Check's
//! findings in **Problems**, each with a place a click goes to; and the
//! Status Bar's hints and counts.

use super::*;
use crate::problems::{Go, Problem, Source};
use mg_module::doctor::Finding;

/// The tileset an editor's tab is, if it is one's.
pub(crate) fn tileset_of(editor: Option<&Tab>) -> Option<u32> {
    match editor? {
        Tab::Tileset(id) => Some(*id),
        _ => None,
    }
}

/// The tileset and its chosen tile an editor's tab shows.
fn chosen<'a>(app: &'a Moonglow, editor: Option<&Tab>) -> Option<(&'a TilesetDoc, Option<usize>)> {
    let id = tileset_of(editor)?;
    let d = app.tilesets.iter().find(|d| d.id == id)?;
    let tiles = d.tileset().map_or(0, |t| t.tiles.len());
    Some((d, d.tile.filter(|t| *t < tiles)))
}

/// What a tileset's Preview and Textures say after their titles (the
/// board's "Tile 16, tcn01_c02_01" and "of tcn01_c02_01 (21)").
pub(crate) fn panel_detail(
    app: &Moonglow,
    editor: Option<&Tab>,
    panel: crate::shell::Panel,
) -> Option<String> {
    let (d, tile) = chosen(app, editor)?;
    let Some(i) = tile else {
        return Some("No tile chosen".to_string());
    };
    let t = d.tileset()?;
    let model = t.tiles[i].model.trim();
    // (A tile with no model, a hole: its number alone, and no textures to
    // count.)
    if model.is_empty() {
        return match panel {
            crate::shell::Panel::Textures => None,
            _ => Some(format!("Tile {i}")),
        };
    }
    Some(match panel {
        crate::shell::Panel::Textures => match app.textures_seen.get(&model.to_ascii_lowercase()) {
            Some((n, 0)) => format!("of {model} ({n})"),
            Some((n, m)) => format!("of {model} ({n}, {m} missing)"),
            None => format!("of {model}"),
        },
        _ => format!("Tile {i}, {model}"),
    })
}

/// The model of the tile chosen, where the game data has it.
fn model_key(app: &Moonglow, d: &TilesetDoc, tile: Option<usize>) -> Option<ResKey> {
    let model = d.tileset()?.tiles.get(tile?)?.model.clone();
    let key = ResKey::parse(&model, ResType::MDL)?;
    app.game.as_deref().is_some_and(|g| g.resman.contains(&key)).then_some(key)
}

/// The Preview's header's icon (the board's `open-in-tab`): the chosen
/// tile's model in a tab of its own (off where the game data has it not,
/// its tip saying where it may be). Whether it was clicked.
pub(crate) fn open_model_button(app: &Moonglow, ui: &mut Ui, editor: Option<&Tab>) -> bool {
    let Some((d, tile)) = chosen(app, editor) else { return false };
    let found = model_key(app, d, tile).is_some();
    let tip = match (tile, found) {
        (None, _) => "Choose a tile to open its model",
        (_, true) => "Open the model in a tab",
        (_, false) => {
            "The model isn't in the game data (a hak the module uses, development, override)"
        }
    };
    let b = mg_uikit::IconButton::new(mg_uikit::Icon::OpenInTab, OPEN_MODEL).icon_only(true);
    ui.add_enabled(found, b).on_hover_text(tip).on_disabled_hover_text(tip).clicked()
}

/// The name of the Preview's Open the Model in a Tab (its icon's).
pub const OPEN_MODEL: &str = "Open the Model in a Tab";

/// The chosen tile's model opened in a tab of its own (the Preview button
/// of the tile's page before M4's U20).
pub(crate) fn open_model(app: &mut Moonglow, editor: Option<&Tab>) {
    let key = chosen(app, editor).and_then(|(d, tile)| model_key(app, d, tile));
    if let Some(k) = key {
        app.actions.push(Action::OpenTab(Tab::Model(k)));
    }
}

/// The diagram's side and its bands (the board's `.tp`: 272 square, the
/// corners' columns 68 and rows 52).
const DIAGRAM: f32 = 272.0;
const BAND_X: f32 = 68.0;
const BAND_Y: f32 = 52.0;
/// The least room under the diagram the model's 3D view is shown in.
const MODEL_LEAST: f32 = 180.0;

/// The name of the diagram (a screen reader's, the tests').
pub const DIAGRAM_NAME: &str = "Tile diagram";

/// A tileset's Preview's body: the chosen tile's diagram on the panel's
/// ground (`.tile-pic`), its model in 3D under it where the panel is tall
/// enough for both (`MODEL_LEAST` more), and the door sentence at its
/// foot (`.pic-note`). With no tile chosen, it says so.
pub(crate) fn preview(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(id) = tileset_of(editor) else { return };
    crate::side_doc::note_pointer(app, ui, &Tab::Tileset(id));
    let t = mg_uikit::tokens(ui.ctx());
    let room = ui.max_rect();
    ui.painter().rect_filled(room, 0.0, t.base);
    let Some((d, tile)) = chosen(app, editor) else { return };
    let Some(set) = d.shared() else { return };
    let Some(i) = tile else {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.weak("Choose a tile to see it.");
        });
        return;
    };
    let tile = &set.tiles[i];
    let doors = d.doors(i);
    let found = model_key(app, d, Some(i));
    // The door sentence, a line a door.
    let sentences: Vec<String> = match doors.is_empty() {
        true => vec!["No doors: a door can't be placed on this tile.".to_string()],
        false => (doors.iter().enumerate())
            .map(|(n, (_, section))| {
                let f = |k: &str| d.get(section, k).and_then(|v| v.trim().parse::<f32>().ok());
                let row = d.get(section, "Type").unwrap_or_default();
                let (x, y, z) =
                    (f("X").unwrap_or(0.0), f("Y").unwrap_or(0.0), f("Z").unwrap_or(0.0));
                let turned = f("Orientation").unwrap_or(0.0);
                format!(
                    "Door {}: doortypes.2da row {}, at ({x:.2}, {y:.2}, {z:.2}), turned {turned}°",
                    n + 1,
                    row.trim()
                )
            })
            .collect(),
    };
    let small = egui::FontId::proportional(11.0);
    // (Each wrapped to the panel's width, as the board's note wraps.)
    let galleys: Vec<_> = (sentences.iter())
        .map(|line| ui.painter().layout(line.clone(), small.clone(), t.text_3, room.width() - 20.0))
        .collect();
    let note_high = 12.0 + galleys.iter().map(|g| g.size().y).sum::<f32>();
    let note = room.with_min_y((room.bottom() - note_high).max(room.top()));
    let above = room.with_max_y(note.top());
    // (The diagram as large as the board's where there is room, smaller
    // in a narrow or short panel; the model under it where the room
    // left is enough for it.)
    let side = DIAGRAM.min(above.width() - 16.0).min(above.height() - 16.0).max(200.0);
    let model_room = above.height() - side - 24.0;
    let with_model = found.is_some() && model_room >= MODEL_LEAST;
    let diagram_area = match with_model {
        true => above.with_max_y(above.top() + side + 24.0),
        false => above,
    };
    let square = egui::Rect::from_center_size(diagram_area.center(), egui::vec2(side, side));
    let orientation = d.get(&format!("TILE{i}"), "Orientation").unwrap_or_else(|| "0".into());
    let middle = (
        tile.model.clone(),
        format!(
            "{}°, {}",
            orientation.trim(),
            match doors.len() {
                0 => "no doors".to_string(),
                1 => "1 door".to_string(),
                n => format!("{n} doors"),
            }
        ),
    );
    diagram(ui, &t, square, tile, middle, found.is_some());
    let r = ui.interact(square, ui.id().with("tile-diagram"), egui::Sense::hover());
    let r = match tile.model.trim().is_empty() {
        true => r,
        false => r.on_hover_text(tile.model.trim()),
    };
    r.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Image,
            true,
            format!("{DIAGRAM_NAME}: tile {i}, its corners' terrain and height, and its edges"),
        )
    });
    if with_model && let Some(key) = found {
        let view = above.with_min_y(diagram_area.bottom()).shrink2(egui::vec2(0.0, 0.0));
        let tab = Tab::Model(key);
        let popped =
            app.open_models.contains(&tab) || app.panes_tabs.iter().flatten().any(|t| *t == tab);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(view).id_salt("tile-model"));
        child.set_clip_rect(view.intersect(ui.clip_rect()));
        if popped {
            child.add_space(6.0);
            child.horizontal(|ui| {
                ui.add_space(10.0);
                ui.weak("The model is in a tab of its own.");
            });
        } else {
            let shown = crate::model_view::Shown {
                info: false,
                pop_out: false,
                restore: false,
                paused: true,
            };
            crate::model_view::panel(
                app,
                &mut child,
                crate::model_view::Source::Resource(key),
                shown,
            );
        }
    }
    // The door sentence, the board's `.pic-note`.
    ui.painter().hline(note.x_range(), note.top() + 0.5, egui::Stroke::new(1.0, t.line_soft));
    let mut y = note.top() + 6.0;
    for galley in galleys {
        let high = galley.size().y;
        ui.painter().galley(egui::pos2(note.left() + 10.0, y), galley, t.text_3);
        y += high;
    }
    let said = sentences.join(" ");
    let r = ui.interact(note, ui.id().with("door-sentence"), egui::Sense::hover());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &said));
}

/// The tile's diagram in `square` (the board's `.tp`): a dashed frame; at
/// its corners each corner's name and its terrain and height; at its
/// edges' middles each edge's name and its crosser (a dash for none); in
/// the middle, framed on the panel's ground, the model's name and its
/// orientation and doors (`middle`), the name in `error` where the game
/// data has the model not.
fn diagram(
    ui: &Ui,
    t: &mg_uikit::Tokens,
    square: egui::Rect,
    tile: &mg_set::Tile,
    middle: (String, String),
    found: bool,
) {
    let p = ui.painter();
    // (The bands keep the board's room for the corners' words; a smaller
    // diagram gives up its middle.)
    let (bx, by) = (BAND_X.min(square.width() * 0.3), BAND_Y.min(square.height() * 0.25));
    dashed(p, square, egui::Stroke::new(1.0, t.line_soft));
    let xs = [square.left(), square.left() + bx, square.right() - bx, square.right()];
    let ys = [square.top(), square.top() + by, square.bottom() - by, square.bottom()];
    let cell = |c: usize, r: usize| {
        egui::Rect::from_min_max(egui::pos2(xs[c], ys[r]), egui::pos2(xs[c + 1], ys[r + 1]))
    };
    let small = egui::FontId::proportional(11.0);
    let two = |rect: egui::Rect, first: &str, second: &str, second_color: egui::Color32| {
        let a = p.layout(first.to_string(), small.clone(), t.text_3, rect.width());
        let b = p.layout(second.to_string(), small.clone(), second_color, rect.width());
        let high = a.size().y + b.size().y;
        let top = rect.center().y - high / 2.0;
        let below = top + a.size().y;
        p.galley(egui::pos2(rect.center().x - a.size().x / 2.0, top), a, t.text_3);
        p.galley(egui::pos2(rect.center().x - b.size().x / 2.0, below), b, second_color);
    };
    // (Corners in the file's order: TopLeft, TopRight, BottomLeft,
    // BottomRight; edges Top, Right, Bottom, Left.)
    let corner = |k: usize| {
        // (A hole's corners name nothing: a dash, as an edge with none.)
        match tile.corners.get(k).cloned().unwrap_or_default() {
            (name, _) if name.is_empty() => "—".to_string(),
            (name, height) => format!("{name}, {height}"),
        }
    };
    let edge = |k: usize| {
        let e = tile.edges.get(k).cloned().unwrap_or_default();
        if e.is_empty() { "—".to_string() } else { e }
    };
    two(cell(0, 0), CORNERS[0], &corner(0), t.text);
    two(cell(2, 0), CORNERS[1], &corner(1), t.text);
    two(cell(0, 2), CORNERS[2], &corner(2), t.text);
    two(cell(2, 2), CORNERS[3], &corner(3), t.text);
    two(cell(1, 0), EDGES[0], &edge(0), t.text_2);
    two(cell(2, 1), EDGES[1], &edge(1), t.text_2);
    two(cell(1, 2), EDGES[2], &edge(2), t.text_2);
    two(cell(0, 1), EDGES[3], &edge(3), t.text_2);
    let mid = cell(1, 1);
    p.rect_filled(mid, 0.0, t.panel);
    p.rect_stroke(mid, 0.0, egui::Stroke::new(1.0, t.line_soft), egui::StrokeKind::Inside);
    let name_color = if found { t.text } else { t.error };
    let middle = match middle.0.trim().is_empty() {
        true => ("(no model)".to_string(), middle.1),
        false => middle,
    };
    // (On one line, cut with … where it is longer than the middle: the
    // whole name is the diagram's tip.)
    let mut job = egui::text::LayoutJob::simple_singleline(
        middle.0,
        egui::FontId::proportional(13.0),
        name_color,
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width((mid.width() - 8.0).max(1.0));
    let name = p.layout_job(job);
    let mut lines = vec![middle.1];
    if !found {
        lines.push("not in the game data".to_string());
    }
    let rest: Vec<_> = lines
        .into_iter()
        .map(|l| p.layout(l, small.clone(), t.text_3, mid.width() - 8.0))
        .collect();
    let high = name.size().y + rest.iter().map(|g| g.size().y).sum::<f32>();
    let mut y = mid.center().y - high / 2.0;
    let x = |g: &std::sync::Arc<egui::Galley>| mid.center().x - g.size().x / 2.0;
    p.galley(egui::pos2(x(&name), y), name.clone(), name_color);
    y += name.size().y;
    for g in rest {
        p.galley(egui::pos2(x(&g), y), g.clone(), t.text_3);
        y += g.size().y;
    }
}

/// A rectangle's dashed frame (the board's `1px dashed`).
fn dashed(p: &egui::Painter, r: egui::Rect, stroke: egui::Stroke) {
    let r = r.shrink(0.5);
    let corners = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    for w in corners.windows(2) {
        p.extend(egui::Shape::dashed_line(&[w[0], w[1]], stroke, 4.0, 3.0));
    }
}

/// A tileset's Textures' body: the chosen tile's model's textures
/// (`model_textures::panel`), as the tile's page listed them under its
/// fields; with no tile chosen, it says so.
pub(crate) fn textures(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(id) = tileset_of(editor) else { return };
    crate::side_doc::note_pointer(app, ui, &Tab::Tileset(id));
    let keys = crate::palette_view::Keys::Textures;
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        crate::palette_view::give_keys(ui.ctx(), keys);
    }
    let focus = app.dialog_panel_focus.take_if(|k| *k == keys).is_some();
    let Some((d, tile)) = chosen(app, editor) else { return };
    let Some(set) = d.shared() else { return };
    let Some(i) = tile else {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.weak("Choose a tile to see its model's textures.");
        });
        return;
    };
    let model = set.tiles[i].model.clone();
    let env = set.general.env_map.clone();
    // (For the header's count, the next frame: kept by the model's name.)
    if let Some(counts) = crate::model_textures::counts(app, ui, &model, env.as_deref()) {
        app.textures_seen.insert(model.to_ascii_lowercase(), counts);
    }
    ui.add_space(2.0);
    crate::model_textures::panel(app, ui, &model, env.as_deref(), focus);
}

/// Where in a tileset a finding is, as a click on its place goes there:
/// the page, the row and the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// The whole file (a line of it, what it can't be read as): the
    /// editor, as it is.
    File,
    /// A page (a list's count: `[TILES] Count`).
    Page(Page),
    /// A key of the general settings, or the page alone.
    General(Option<String>),
    /// A terrain (`crosser`: a crosser) by its number.
    Terrain { crosser: bool, index: usize },
    /// A painting rule (`secondary`) by its number.
    Rule { secondary: bool, index: usize },
    /// A tile by its number, and the field the finding names.
    Tile { tile: usize, field: Option<String> },
    /// A group by its number.
    Group(usize),
}

/// The number after `prefix` in `section` (`TILE40` → 40), where it is all
/// of what follows.
fn numbered(section: &str, prefix: &str) -> Option<usize> {
    section
        .strip_prefix(prefix)
        .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))?
        .parse()
        .ok()
}

/// Where a finding of the content doctor's tileset checks is (its `at`:
/// `[TILE40] Model`, `[GROUP3] Tile0`, `[TERRAIN2]`, `[TILES] Count`,
/// `door Type=7` with the tiles in its message, `line 12`, none; a model's
/// own finding by its model, at the first tile of it).
pub(crate) fn place_of(f: &Finding, t: Option<&Tileset>) -> Place {
    if f.resource.restype == ResType::MDL {
        let name = f.resource.resref.to_string();
        let tile = t.and_then(|t| t.tiles.iter().position(|x| x.model.eq_ignore_ascii_case(&name)));
        return match tile {
            Some(tile) => Place::Tile { tile, field: Some("Model".into()) },
            None => Place::File,
        };
    }
    let at = f.at.trim();
    if let Some(type_) = at.strip_prefix("door Type=") {
        let _ = type_;
        // (The tiles are named in the message, "(TILE3, TILE5)": the first.)
        let first =
            f.message.split(|c: char| !c.is_ascii_alphanumeric()).find_map(|w| numbered(w, "TILE"));
        return first.map_or(Place::Page(Page::Tiles), |tile| Place::Tile { tile, field: None });
    }
    let Some(rest) = at.strip_prefix('[') else { return Place::File };
    let Some((section, key)) = rest.split_once(']') else { return Place::File };
    let section = section.trim().to_ascii_uppercase();
    let key = Some(key.trim().to_string()).filter(|k| !k.is_empty());
    if let Some(n) = section
        .strip_prefix("TILE")
        .and_then(|s| s.split_once("DOOR"))
        .and_then(|(n, _)| n.parse().ok())
    {
        return Place::Tile { tile: n, field: None };
    }
    if let Some(tile) = numbered(&section, "TILE") {
        return Place::Tile { tile, field: key };
    }
    if let Some(g) = numbered(&section, "GROUP") {
        return Place::Group(g);
    }
    if let Some(index) = numbered(&section, "TERRAIN") {
        return Place::Terrain { crosser: false, index };
    }
    if let Some(index) = numbered(&section, "CROSSER") {
        return Place::Terrain { crosser: true, index };
    }
    if let Some(index) = numbered(&section, "PRIMARY RULE") {
        return Place::Rule { secondary: false, index };
    }
    if let Some(index) = numbered(&section, "SECONDARY RULE") {
        return Place::Rule { secondary: true, index };
    }
    match section.as_str() {
        "GENERAL" | "GRASS" => Place::General(key),
        "TILES" => Place::Page(Page::Tiles),
        "GROUPS" => Place::Page(Page::Groups),
        "TERRAIN TYPES" | "CROSSER TYPES" => Place::Page(Page::Terrain),
        "PRIMARY RULES" | "SECONDARY RULES" => Place::Page(Page::Rules),
        _ => Place::File,
    }
}

/// A finding's place in words, as Problems lists it: "Tile 40, Model",
/// "Group 3, Tile0", "Terrain 2", "Tiles, Count"; a line or a model as the
/// finding says.
pub(crate) fn place_words(place: &Place, f: &Finding) -> String {
    let with = |words: String, key: &Option<String>| match key {
        Some(k) => format!("{words}, {k}"),
        None => words,
    };
    let after =
        || f.at.split_once(']').map(|(_, k)| k.trim().to_string()).filter(|k| !k.is_empty());
    match place {
        Place::Tile { tile, field } => with(format!("Tile {tile}"), field),
        Place::Group(g) => with(format!("Group {g}"), &after()),
        Place::Terrain { crosser: false, index } => format!("Terrain {index}"),
        Place::Terrain { crosser: true, index } => format!("Crosser {index}"),
        Place::Rule { secondary: false, index } => format!("Primary rule {index}"),
        Place::Rule { secondary: true, index } => format!("Secondary rule {index}"),
        Place::General(key) => with("General".to_string(), key),
        Place::Page(p) => with(page_name(*p).to_string(), &after()),
        Place::File if f.resource.restype == ResType::MDL => f.resource.to_string(),
        Place::File => {
            let mut at = f.at.trim().to_string();
            if let Some(first) = at.get(..1) {
                at = first.to_uppercase() + &at[1..];
            }
            at
        }
    }
}

/// What a tileset's Check found, into Problems in place of what it found
/// before; the dock opened on it where it found anything, the log saying
/// so where it found nothing.
pub(crate) fn to_problems(app: &mut Moonglow, d: &TilesetDoc, findings: &[Finding]) {
    let t = d.tileset();
    let problems: Vec<Problem> = findings
        .iter()
        .map(|f| {
            let place = place_of(f, t);
            // (A finding of the whole file with no line names the file: no
            // link of no words.)
            let mut at = place_words(&place, f);
            if at.is_empty() {
                at = Source::Tileset(d.path.clone()).title();
            }
            Problem::new(
                f.severity,
                at,
                f.message.clone(),
                Some(Go::Tileset { path: d.path.clone(), place }),
            )
        })
        .collect();
    let found = !problems.is_empty();
    let source = Source::Tileset(d.path.clone());
    if !found {
        app.log.info(format!("Check: {}: no problems found", source.title()));
    }
    app.problems.replace(source, problems, Some(d.fingerprint()));
    app.problems.ask_forward(found);
}

/// Goes to `place` in the tileset file at `path`: its editor brought
/// forward (opened again where it was closed), on the place's page, its
/// tile or group chosen and in sight (Find let go of where it hides the
/// tile), the field it names given the keyboard.
pub fn go_to(app: &mut Moonglow, path: &Path, place: &Place) {
    if !app.tilesets.iter().any(|d| d.path == path) {
        // (Not found where it was: `open_path` says so in Problems too.)
        open_path(app, path.to_path_buf());
    } else if let Some(d) = app.tilesets.iter().find(|d| d.path == path) {
        app.actions.push(Action::OpenTab(Tab::Tileset(d.id)));
    }
    let Some(d) = app.tilesets.iter_mut().find(|d| d.path == path) else { return };
    let (tiles, groups) = d.tileset().map_or((0, 0), |t| (t.tiles.len(), t.groups.len()));
    match place {
        Place::File => {}
        Place::Page(p) => d.page = *p,
        Place::General(_) => d.page = Page::General,
        Place::Terrain { .. } => d.page = Page::Terrain,
        Place::Rule { .. } => d.page = Page::Rules,
        Place::Group(g) => {
            d.page = Page::Groups;
            if *g < groups {
                d.group = Some(*g);
            }
        }
        Place::Tile { tile, field } => {
            d.page = Page::Tiles;
            if *tile < tiles {
                d.tile = Some(*tile);
                if !d.rows().contains(tile) {
                    d.filter.clear();
                }
                if let Some(key) = field {
                    d.focus_field = Some((format!("TILE{tile}"), key.clone()));
                }
            }
        }
    }
    if let Place::General(Some(key)) = place {
        d.focus_field = Some(("GENERAL".into(), key.clone()));
    }
}

/// A row of a tileset's Outliner: a group (its place in
/// [`OUTLINE_GROUPS`]) or one of its parts (the group's place and the
/// part's number).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutlineRow {
    General,
    Group(usize),
    Part(usize, usize),
}

/// The Outliner's groups of a tileset (the board's): their names, icons,
/// the words a screen reader says with their counts, and the page each
/// goes to.
pub(crate) const OUTLINE_GROUPS: [(&str, mg_uikit::Icon, &str, Page); 6] = [
    ("Terrains", mg_uikit::Icon::Terrain, "terrain", Page::Terrain),
    ("Crossers", mg_uikit::Icon::TileBorder, "crosser", Page::Terrain),
    ("Primary rules", mg_uikit::Icon::Paint, "primary rule", Page::Rules),
    ("Secondary rules", mg_uikit::Icon::Paint, "secondary rule", Page::Rules),
    ("Tiles", mg_uikit::Icon::Tile, "tile", Page::Tiles),
    ("Groups", mg_uikit::Icon::Layers, "group", Page::Groups),
];

/// How many parts each group of a tileset has.
pub(crate) fn outline_counts(t: &Tileset) -> [usize; 6] {
    [
        t.terrains.len(),
        t.crossers.len(),
        t.primary_rules.len(),
        t.secondary_rules.len(),
        t.tiles.len(),
        t.groups.len(),
    ]
}

/// A part's words in the Outliner: a terrain's or crosser's name, a rule
/// as "Grass beside Water makes Sand", a tile as "16 tcn01_c02_01", a group
/// as "3 Bridge (2×1)".
pub(crate) fn part_words(t: &Tileset, group: usize, at: usize) -> String {
    match group {
        0 => t.terrains.get(at).map(|x| x.name.clone()).unwrap_or_default(),
        1 => t.crossers.get(at).map(|x| x.name.clone()).unwrap_or_default(),
        2 | 3 => {
            let list = if group == 2 { &t.primary_rules } else { &t.secondary_rules };
            list.get(at)
                .map(|r| format!("{at} {} beside {} makes {}", r.placed, r.adjacent, r.changed))
                .unwrap_or_default()
        }
        4 => t.tiles.get(at).map(|x| format!("{at} {}", x.model)).unwrap_or_default(),
        _ => t
            .groups
            .get(at)
            .map(|g| format!("{at} {} ({}×{})", g.name, g.columns, g.rows))
            .unwrap_or_default(),
    }
}

/// A part's name for a screen reader (the Outliner's): its kind and
/// number with its words, "Terrain 0: Grass", "Tile 16: tcn01_c02_01".
pub(crate) fn part_label(t: &Tileset, group: usize, at: usize) -> String {
    let words = part_words(t, group, at);
    let rest = match group {
        0 | 1 => words.as_str(),
        _ => words.split_once(' ').map_or(words.as_str(), |(_, r)| r),
    };
    let kind = ["Terrain", "Crosser", "Primary rule", "Secondary rule", "Tile", "Group"][group];
    format!("{kind} {at}: {rest}")
}

/// Where `row` is among the Outliner's rows, by the groups' counts and
/// which are open (`None`: in a closed group, or past its group's end).
pub(crate) fn outline_index(
    row: OutlineRow,
    open: &[bool; 6],
    counts: &[usize; 6],
) -> Option<usize> {
    let group_at =
        |g: usize| 1 + (0..g).map(|k| 1 + if open[k] { counts[k] } else { 0 }).sum::<usize>();
    match row {
        OutlineRow::General => Some(0),
        OutlineRow::Group(g) => Some(group_at(g)),
        OutlineRow::Part(g, p) => (open[g] && p < counts[g]).then(|| group_at(g) + 1 + p),
    }
}

/// The Outliner's rows of a tileset: General, then each group and, while
/// it is open, its parts; made once for the file's revision and the groups
/// open (a tileset's thousands of tiles each frame).
pub(crate) fn outline_rows(d: &TilesetDoc) -> Arc<Vec<OutlineRow>> {
    let mark = (d.revision, d.outline_open);
    if let Some((was, rows)) = &*d.outline_seen.borrow()
        && *was == mark
    {
        return rows.clone();
    }
    let counts = d.tileset().map(outline_counts).unwrap_or_default();
    let mut rows = vec![OutlineRow::General];
    for (g, n) in counts.iter().enumerate() {
        rows.push(OutlineRow::Group(g));
        if d.outline_open[g] {
            rows.extend((0..*n).map(|at| OutlineRow::Part(g, at)));
        }
    }
    let rows = Arc::new(rows);
    d.walks.set(d.walks.get() + 1);
    *d.outline_seen.borrow_mut() = Some((mark, rows.clone()));
    rows
}

/// What the Status Bar shows beside a tileset (the Tileset board's): on
/// Tiles, Click Choose a tile and Find's keys (as bound, while the pointer
/// is over the page, where Find's keys go to its field); none while a field
/// is typed in, a popover, dialog or menu is open. Its counts: "408 tiles, 89
/// groups, 5 terrains, 4 crossers", its path in their tip.
pub(crate) fn shown(
    app: &Moonglow,
    k: &crate::shell::HintKeys<'_>,
    ctx: &egui::Context,
    id: u32,
) -> crate::shell::Shown {
    use crate::keys::Cmd;
    use crate::shell::{Count, Shown};
    use mg_uikit::status::Hint;
    let Some(d) = app.tilesets.iter().find(|d| d.id == id) else { return Shown::default() };
    let mut counts = Vec::new();
    if let Some(t) = d.tileset() {
        let mut c = Count::new(format!(
            "{}, {}, {}, {}",
            crate::problems::count(t.tiles.len(), "tile"),
            crate::problems::count(t.groups.len(), "group"),
            crate::problems::count(t.terrains.len(), "terrain"),
            crate::problems::count(t.crossers.len(), "crosser"),
        ));
        c.tip = Some(d.path.display().to_string());
        counts.push(c);
    }
    let mut hints = Vec::new();
    let typing = ctx.memory(|m| m.focused().is_some()) && ctx.egui_wants_keyboard_input();
    if crate::shell::keys_blocked(ctx) || mg_uikit::popover::Popover::any_open(ctx) || typing {
        return Shown { hints, counts, note: None };
    }
    if d.page == Page::Tiles && d.set.is_ok() {
        if super::list_has_keys(app, ctx, id) {
            hints.push(Hint::either(["Up", "Down"], "Move in the list"));
        }
        hints.push(Hint::keys(["Click"], "Choose a tile"));
        if d.hovered {
            hints.extend(k.hint(Cmd::Find, "Find"));
        }
    }
    // (No Save's keys: they are the module's Save, as beside a 2DA or a
    // talk table, whose hints have none either; the header's Save saves the
    // tileset. The coordinator's ruling at U20's review.)
    Shown { hints, counts, note: None }
}
