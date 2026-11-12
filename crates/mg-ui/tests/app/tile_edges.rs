//! Objects dragged across the edge between two tiles at different heights
//! (a builder's report: a tile forced where it does not fit, raised above
//! its neighbor, and a placeable dragged over the divide crashed the
//! toolset). Each case builds its area with the editor's own commands
//! (Tile Properties' All Tiles or the palette's Exact Tile, Tile
//! Properties' Height), puts an object from the game's blueprints on one
//! tile and drags it across, a frame for each small step of the pointer,
//! pausing on the edge; or leads a blueprint's or a paste's ghost across
//! and puts it down. Every frame of the drag, where the view shows the
//! object is checked to be a number, and a jump of more than a few
//! meters between two frames is said. Beside a raised tile with no cliff
//! face the view shows through the gap, and what the pointer is over
//! there is what is seen behind: an object let go there goes to it (the
//! owner's ruling of 2026-11-11). Pictures in
//! `target/test-output/tile-edges/CASE/`.

use crate::common::*;
use glam::{Vec2, Vec3};
use mg_area::ObjectKind;

/// How the tile that is raised is put in.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Forced {
    /// As the area has it (a tile that fits).
    No,
    /// Tile Properties' All Tiles (Ignore Fit).
    AllTiles,
    /// The tiles palette's Exact Tile.
    Palette,
}

/// What is led across the edge, and how.
#[derive(Clone, Copy, Debug, PartialEq)]
enum How {
    /// The object, pressed on and led.
    Free,
    /// The object by its arrow east (Shift held).
    SlideX,
    /// The object by its arrow north (Shift held).
    SlideY,
    /// The blueprint chosen in the palette: its ghost led, then a click.
    Place,
    /// The object copied: the paste's ghost led, then a click.
    Paste,
    /// The blueprint's row dragged from the palette's list into the view
    /// and let go there.
    Dnd,
}

/// What forcing the tile is left as when the drag begins.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Leave {
    /// Put away: Tile Properties closed, Select Tiles off, no brush.
    Nothing,
    /// The palette's Exact Tile on and the tile still chosen (the tile
    /// put in last, after the heights).
    Brush,
    /// Select Tiles on, the raised tile selected.
    TileSelect,
    /// Tile Properties open (its Height applied as it is changed), Select
    /// Tiles turned off.
    PropsOpen,
}

/// Where the pointer is let go at the end of the path (the path's end
/// being on the edge), checked by the ray through it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Aim {
    /// 10 cm before the edge, the ray on the first tile's ground.
    Near,
    /// 10 cm past it, on the second tile's ground.
    Far,
    /// On the edge between the two levels: the ray meets no walkmesh.
    Gap,
}

#[derive(Clone, Debug)]
struct Case {
    name: &'static str,
    tileset: &'static str,
    /// The area resized to these columns and rows (from 4 by 4).
    size: Option<(u32, u32)>,
    forced: Forced,
    /// The tile (column, row) forced and raised, by how many steps.
    raised: (u32, u32),
    steps: i32,
    /// Where the object stands, and where the drag leads it (the path's
    /// points, on the ground; the edge is crossed between them).
    from: Vec2,
    to: Vec2,
    /// The edge's points the pointer waits on (a few frames each).
    edges: Vec<Vec2>,
    angled: bool,
    /// The angled camera's turn (degrees: -120 from the south-west, 60
    /// from the north-east) and pitch (radians).
    yaw: f32,
    pitch: f32,
    snap: bool,
    onto: bool,
    how: How,
    /// How far above the ground the object stands.
    lift: f32,
    kind: ObjectKind,
    blueprint: &'static str,
    /// The walkmesh overlay on.
    walkmesh: bool,
    /// Placeables' and doors' walkmeshes shown.
    object_walkmesh: bool,
    /// The tile forced in: a cliff's (its east corners a step up: high
    /// ground against its eastern neighbor with no cliff between, at
    /// x = 30) instead of the end of a wall.
    cliff: bool,
    /// The game's shaders (the release's default) instead of Moonglow's.
    shaders: bool,
    /// The pointer led a few points a frame on screen, from the press to
    /// the end, rather than from one ground point to the next (in the
    /// angled view, it then passes over the gap beside a raised tile).
    pixels: bool,
    /// On each edge, the pointer held on points of the edge between the
    /// two tiles' levels (where a cliff's face would be).
    gap: bool,
    /// A second object, at this point, selected with the first.
    second: Option<Vec2>,
    /// Fade Geometry Always and Tile Border on.
    fade_border: bool,
    /// The other tile of the edge (where the path begins, or ends when it
    /// begins on the raised one) raised by this many steps first.
    other_steps: i32,
    leave: Leave,
    aim: Option<Aim>,
}

impl Case {
    fn new(name: &'static str) -> Case {
        Case {
            name,
            tileset: "ttr01",
            size: None,
            forced: Forced::AllTiles,
            raised: (2, 1),
            steps: 1,
            from: Vec2::new(15.0, 15.0),
            to: Vec2::new(25.0, 15.0),
            edges: vec![Vec2::new(20.0, 15.0)],
            angled: false,
            yaw: -120.0,
            pitch: 0.55,
            snap: false,
            onto: false,
            how: How::Free,
            lift: 0.0,
            kind: ObjectKind::Placeable,
            blueprint: "plc_chest1",
            walkmesh: false,
            object_walkmesh: false,
            cliff: false,
            shaders: false,
            pixels: false,
            gap: false,
            second: None,
            fade_border: false,
            other_steps: 0,
            leave: Leave::Nothing,
            aim: None,
        }
    }
}

/// The area's ground as the view reads it: its tiles' walkmeshes.
fn ground(h: &Harness<'_, Moonglow>, area: ResRef) -> mg_area::walk::Ground {
    let game = h.state().game.as_deref().unwrap();
    let model = h.state().area_views[&area].model.as_ref().expect("the area is read");
    mg_area::walk::Ground::new(game, model)
}

/// A point of the ground (the walkmesh's height there, nearest `near`).
fn on(g: &mg_area::walk::Ground, p: Vec2, near: f32) -> Vec3 {
    p.extend(g.height(p, near).unwrap_or(near))
}

fn tile_list(h: &mut Harness<'_, Moonglow>, area: ResRef) -> Vec<mg_gff::Struct> {
    let ws = h.state_mut().ws.as_mut().unwrap();
    ws.doc(&ResKey::new(area, ResType::ARE)).unwrap().root.list("Tile_List").unwrap().to_vec()
}

/// The area's width in tiles.
fn width(h: &Harness<'_, Moonglow>, area: ResRef) -> u32 {
    h.state().area_views[&area].model.as_ref().unwrap().width
}

/// Tile Properties of the tile at `p`, from Select Tiles' tile menu.
fn tile_properties(h: &mut Harness<'_, Moonglow>, area: ResRef, p: Vec3) {
    if !h.state().area_views[&area].tile_mode {
        area_tool(h, "Select Tiles").click();
        h.run_steps(2);
    }
    let at = screen(h, area, p);
    h.hover_at(at);
    press(h, at, true, egui::Modifiers::NONE);
    press(h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Default::default(),
        });
    }
    h.run_steps(2);
    h.get_by_label("Tile Properties…").click();
    h.run_steps(3);
    let path = h.state().dock.find_tab(&Tab::TileProperties).expect("Tile Properties is open");
    if let Some(w) = h.state_mut().dock.get_window_state_mut(path.surface) {
        w.set_position(egui::pos2(842.0, 60.0)).set_size(egui::vec2(255.0, 600.0));
    }
    h.run_steps(3);
}

/// A tile that fits nowhere in open ground: in Rural the end of a wall,
/// or (`cliff`) a cliff, its east corners a step up; in another tileset,
/// the first with a crosser on a level tile.
fn forced_tile(h: &Harness<'_, Moonglow>, tileset: &str, cliff: bool) -> (u32, String) {
    use mg_tiles::{NE, NW, SE, SW, TileIndex};
    let game = h.state().game.as_deref().unwrap();
    let set = mg_area::tileset(game, ResRef::from_str(tileset).unwrap()).unwrap();
    let index = TileIndex::new(&set);
    let tile = if cliff {
        *index
            .tiles_where(|c| {
                let k = c.corners;
                c.edges.iter().all(Option::is_none)
                    && k[SW].height == k[NW].height
                    && k[SE].height == k[NE].height
                    && k[SE].height > k[SW].height
            })
            .first()
            .expect("the tileset has cliffs")
    } else if tileset == "ttr01" {
        let (wall, grass) = (index.crosser("Wall1").unwrap(), index.terrain("Grass").unwrap());
        *index
            .tiles_where(|c| {
                c.edges.iter().filter(|e| **e == Some(wall)).count() == 1
                    && c.corners.iter().all(|k| k.terrain == grass && k.height == 0)
            })
            .first()
            .expect("Rural has wall ends")
    } else {
        *index
            .tiles_where(|c| {
                c.edges.iter().any(Option::is_some) && c.corners.iter().all(|k| k.height == 0)
            })
            .first()
            .expect("a tile with a crosser")
    };
    (tile, set.tiles[tile as usize].model.clone())
}

fn tile_centre(column: u32, row: u32) -> Vec3 {
    Vec3::new(column as f32 * 10.0 + 5.0, row as f32 * 10.0 + 5.0, 0.0)
}

/// Resizes the area from the Edit menu (as `tiles.rs`'s `resize_area_to`).
fn resize_area_to(h: &mut Harness<'_, Moonglow>, area: ResRef, columns: u32, rows: u32) {
    h.get_by_label("Edit").click();
    h.run_steps(2);
    h.get_by_label("Resize Area…").click();
    h.run_steps(2);
    let d = h.state_mut().resize_area.as_mut().expect("the window is open");
    (d.columns, d.rows) = (columns, rows);
    h.run_steps(1);
    h.get_by_label("OK").click();
    h.run_steps(3);
    settle(h);
    let view = h.state_mut().area_views.get_mut(&area).unwrap();
    let model = view.model.as_ref().unwrap();
    assert_eq!((model.width, model.height), (columns, rows));
    view.orbit = Some(mg_ui::area_view::Orbit::overview(model));
    h.run_steps(2);
}

/// The tiles palette's Exact Tile with `model` chosen, clicked on the
/// tile at `centre`; the brush put away after unless `keep`.
fn exact_tile(h: &mut Harness<'_, Moonglow>, area: ResRef, model: &str, centre: Vec3, keep: bool) {
    show_palette(h.state_mut());
    h.run_steps(3);
    palette_type(h, "Tiles");
    h.run_steps(3);
    h.get_by_label("Gallery").click();
    h.run_steps(2);
    if !h.state().settings.exact_tiles {
        h.get_by_label("Exact Tile").click();
        h.run_steps(2);
    }
    h.state_mut().palette.filter = model.to_lowercase();
    h.run_steps(3);
    h.get_all_by_label(model).next().expect("its picture").scroll_to_me();
    h.run_steps(3);
    h.get_all_by_label(model).next().unwrap().click();
    h.run_steps(2);
    let at = screen(h, area, centre);
    h.hover_at(at);
    h.run_steps(2);
    press(h, at, true, egui::Modifiers::NONE);
    press(h, at, false, egui::Modifiers::NONE);
    h.run_steps(3);
    if !keep {
        // (The brush put away: what follows moves objects.)
        h.key_press(egui::Key::Escape);
        h.run_steps(2);
        h.state_mut().palette.tile_brush = None;
        h.state_mut().palette.chosen.clear();
        h.run_steps(2);
    }
}

/// Tile Properties' Height changed by `steps` with its buttons.
fn height_by(h: &mut Harness<'_, Moonglow>, steps: i32) {
    let button = if steps > 0 { "Raise Height" } else { "Lower Height" };
    for _ in 0..steps.unsigned_abs() {
        h.get_by_label(button).click();
        h.run_steps(2);
    }
}

/// The area as the case has it: the other tile raised (if asked), the
/// tile forced (or not) and raised, and what that leaves behind. The
/// raised tile's index.
fn build(h: &mut Harness<'_, Moonglow>, area: ResRef, case: &Case) -> usize {
    if let Some((columns, rows)) = case.size {
        resize_area_to(h, area, columns, rows);
    }
    let (tile, model) = forced_tile(h, case.tileset, case.cliff);
    let w = width(h, area);
    let raised = (case.raised.1 * w + case.raised.0) as usize;
    let centre = tile_centre(case.raised.0, case.raised.1);
    let before = tile_list(h, area);
    if case.other_steps != 0 {
        let other = other_tile(case, w, raised);
        let at = tile_centre(other as u32 % w, other as u32 / w);
        tile_properties(h, area, at);
        height_by(h, case.other_steps);
        h.get_by_label("OK").click();
        h.run_steps(4);
    }
    let brush_last = case.leave == Leave::Brush;
    if case.forced == Forced::Palette && !brush_last {
        exact_tile(h, area, &model, centre, false);
        let now = tile_list(h, area);
        assert_eq!(now[raised].integer("Tile_ID"), Some(i64::from(tile)), "exact tile in");
    }
    tile_properties(h, area, centre);
    if case.forced == Forced::AllTiles {
        h.get_by_label(mg_ui::tile_select::ALL_TILES).click();
        h.run_steps(3);
        type_into_hint(h, mg_ui::tile_select::ALL_TILES_SEARCH, &model);
        h.run_steps(10);
        let at = h.get_all_by_label(&model).next().expect("its picture").rect().center_top()
            - egui::vec2(0.0, 40.0);
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(4);
        let now = tile_list(h, area);
        assert_eq!(now[raised].integer("Tile_ID"), Some(i64::from(tile)), "forced in");
    }
    height_by(h, case.steps);
    if case.leave != Leave::PropsOpen {
        h.get_by_label("OK").click();
        h.run_steps(4);
    }
    if case.leave == Leave::PropsOpen {
        // (The window lies over the toolbar's switch: F10 over the view.)
        let c = h.state().area_views[&area].rect.center();
        h.hover_at(c);
        h.run_steps(1);
        h.key_press(egui::Key::F10);
        h.run_steps(3);
    } else if case.leave != Leave::TileSelect && h.state().area_views[&area].tile_mode {
        area_tool(h, "Select Tiles").click();
        h.run_steps(2);
    }
    if case.forced == Forced::Palette && brush_last {
        exact_tile(h, area, &model, centre, true);
        let now = tile_list(h, area);
        assert_eq!(now[raised].integer("Tile_ID"), Some(i64::from(tile)), "exact tile in");
    }
    settle(h);
    let now = tile_list(h, area);
    let was = before[raised].integer("Tile_Height").unwrap_or(0);
    assert_eq!(now[raised].integer("Tile_Height"), Some(was + i64::from(case.steps)), "raised");
    if case.forced == Forced::No {
        assert_eq!(now[raised].integer("Tile_ID"), before[raised].integer("Tile_ID"));
    }
    eprintln!(
        "left: Select Tiles {}, tiles selected {:?}, Tile Properties open {}, Exact Tile {}, \
         tile brush {}",
        h.state().area_views[&area].tile_mode,
        h.state().area_views[&area].tile_selection,
        h.state().tile_props.is_some(),
        h.state().settings.exact_tiles,
        h.state().palette.tile_brush.is_some()
    );
    raised
}

/// The tile of the case's edge that is not the raised one.
fn other_tile(case: &Case, width: u32, raised: usize) -> usize {
    let (a, b) = (tile_index(case.from, width), tile_index(case.to, width));
    if a == raised { b } else { a }
}

fn rtype(kind: ObjectKind) -> ResType {
    match kind {
        ObjectKind::Placeable => ResType::UTP,
        ObjectKind::Creature => ResType::UTC,
        ObjectKind::Door => ResType::UTD,
        ObjectKind::Item => ResType::UTI,
        ObjectKind::Waypoint => ResType::UTW,
        ObjectKind::Sound => ResType::UTS,
        ObjectKind::Store => ResType::UTM,
        ObjectKind::Trigger => ResType::UTT,
        ObjectKind::Encounter => ResType::UTE,
    }
}

/// How many objects of `kind` the area's GIT holds.
fn count(h: &mut Harness<'_, Moonglow>, area: ResRef, kind: ObjectKind) -> usize {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
    git.root.list(kind.list()).map_or(0, |l| l.len())
}

/// Puts an object of the case's blueprint in, on the ground at `at`
/// (lifted).
fn place(h: &mut Harness<'_, Moonglow>, area: ResRef, case: &Case, at: Vec2) -> usize {
    use mg_module::instances::{Placement, Placing, instance};
    let g = ground(h, area);
    let at = on(&g, at, 0.0) + Vec3::Z * case.lift;
    let rtype = rtype(case.kind);
    let item = {
        let game = h.state().game.as_deref().unwrap();
        let key = ResKey::parse(case.blueprint, rtype).unwrap();
        let bp = Gff::read(&game.resman.get(&key).expect("the blueprint")).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let placement = Placement { position: at.to_array(), rotation: 0.0 };
        // (A trigger's or encounter's outline: a square 2 m across.)
        let z = at.z + 0.1;
        let square = [[-1.0, -1.0, z], [1.0, -1.0, z], [1.0, 1.0, z], [-1.0, 1.0, z]];
        let outline: &[[f32; 3]] = if case.kind.has_outline() { &square } else { &[] };
        instance(&placing, rtype, &bp, placement, outline).unwrap()
    };
    let list = case.kind.list();
    let index = count(h, area, case.kind);
    let edit = mg_edit::Edit::InsertItem {
        key: ResKey::new(area, ResType::GIT),
        path: mg_edit::GffPath::root(),
        list: list.into(),
        index,
        item,
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    h.run_steps(3);
    settle(h);
    index
}

fn position(h: &Harness<'_, Moonglow>, area: ResRef, kind: ObjectKind, index: usize) -> Vec3 {
    let model = h.state().area_views[&area].model.as_ref().unwrap();
    model.object(kind, index).expect("the object").position
}

/// What the view shows moving this frame: the dragged objects' places
/// and the ghost's.
fn shown(h: &Harness<'_, Moonglow>, area: ResRef) -> Vec<Vec3> {
    let view = &h.state().area_views[&area];
    let mut out: Vec<Vec3> = view.dragged().into_iter().map(|(_, p, _)| p).collect();
    out.extend(view.ghost_shown.as_ref().map(|g| g.position));
    out
}

/// Follows what the view shows from frame to frame: each place a number,
/// and a jump of more than 3 m said.
struct Watch {
    last: Vec<Vec3>,
    frames: usize,
    jumps: Vec<String>,
}

impl Watch {
    fn frame(&mut self, h: &Harness<'_, Moonglow>, area: ResRef, pointer: egui::Pos2) {
        self.frames += 1;
        let now = shown(h, area);
        for p in &now {
            assert!(p.is_finite(), "frame {}: shown at {p} (pointer at {pointer:?})", self.frames);
        }
        if let (Some(a), Some(b)) = (self.last.first(), now.first())
            && (*a - *b).length() > 3.0
        {
            let line = format!("frame {}: {a} to {b}, pointer at {pointer:?}", self.frames);
            eprintln!("(jump: {line})");
            self.jumps.push(line);
        }
        if !now.is_empty() {
            self.last = now;
        }
    }
}

/// A frame, watched.
fn frame(h: &mut Harness<'_, Moonglow>, area: ResRef, watch: &mut Watch, at: egui::Pos2) {
    h.step();
    watch.frame(h, area, at);
}

/// The pointer led along `points` (in the area), a frame each, waiting
/// `wait` frames where it is within 5 cm of one of `edges`; there, with
/// `gap`, held on the edge's points between `levels`.
#[allow(clippy::too_many_arguments)]
fn lead(
    h: &mut Harness<'_, Moonglow>,
    area: ResRef,
    watch: &mut Watch,
    points: &[Vec3],
    edges: &[Vec2],
    wait: usize,
    gap: Option<(f32, f32)>,
) -> egui::Pos2 {
    let mut last = egui::Pos2::ZERO;
    let mut gapped = false;
    for p in points {
        let view = &h.state().area_views[&area];
        let Some(at) = view.screen_pos(*p) else { continue };
        if !view.rect.contains(at) {
            eprintln!("(the pointer is out of the view at {p})");
        }
        h.hover_at(at);
        frame(h, area, watch, at);
        let on_edge = edges
            .iter()
            .find(|e| (p.truncate() - **e).length() < 0.05 || near_line(p.truncate(), **e));
        if let Some(e) = on_edge {
            for _ in 0..wait {
                frame(h, area, watch, at);
            }
            if let Some((low, high)) = gap.filter(|_| !gapped) {
                gapped = true;
                let n = ((high - low) / 0.5).ceil().max(1.0) as usize;
                for k in 0..=n {
                    let z = low + (high - low) * k as f32 / n as f32;
                    let p = p.truncate().extend(z);
                    let _ = e;
                    if let Some(g) = h.state().area_views[&area].screen_pos(p) {
                        h.hover_at(g);
                        for _ in 0..3 {
                            frame(h, area, watch, g);
                        }
                    }
                }
                h.hover_at(at);
                frame(h, area, watch, at);
            }
        }
        last = at;
    }
    last
}

/// The pointer led on screen from `a` to `b`, `step` points a frame,
/// waiting 3 frames a step over the middle half.
fn lead_pixels(
    h: &mut Harness<'_, Moonglow>,
    area: ResRef,
    watch: &mut Watch,
    a: egui::Pos2,
    b: egui::Pos2,
    step: f32,
) -> egui::Pos2 {
    let n = ((b - a).length() / step).ceil().max(1.0) as usize;
    for k in 1..=n {
        let t = k as f32 / n as f32;
        let at = a + (b - a) * t;
        h.hover_at(at);
        frame(h, area, watch, at);
        if (0.25..0.75).contains(&t) {
            for _ in 0..3 {
                frame(h, area, watch, at);
            }
        }
    }
    b
}

/// Whether `p` is within 5 cm of the edge line through `e` (an edge on a
/// multiple of 10 m, along x or y).
fn near_line(p: Vec2, e: Vec2) -> bool {
    let on_x = e.x % 10.0 == 0.0 && (p.x - e.x).abs() < 0.05 && (p.y - e.y).abs() < 1.0;
    let on_y = e.y % 10.0 == 0.0 && (p.y - e.y).abs() < 0.05 && (p.x - e.x).abs() < 1.0;
    on_x || on_y
}

/// The harness for a case: the game's shaders on when it asks, and their
/// programs made before it goes on.
fn harness(case: &Case) -> Option<(Harness<'static, Moonglow>, ResRef)> {
    let name = format!("edge-{}", case.name);
    let (mut app, rs, area) = area_app(&name, case.tileset, None, 4, None, &|_, _, _| {})?;
    if case.shaders {
        app.settings.game_shaders = Some(true);
    }
    if case.fade_border {
        app.settings.fade_geometry = 2;
        app.settings.tile_border = true;
    }
    let mut h = area_gpu_harness(app, rs, AREA_HARNESS_WIDTH, 800.0);
    h.run_steps(3);
    settle(&mut h);
    Some((h, area))
}

/// Waits for the game's programs to be made and says how many models
/// they draw (0 with Moonglow's own).
fn game_programs(h: &mut Harness<'_, Moonglow>, area: ResRef) -> usize {
    h.state_mut().viewport.as_mut().unwrap().renderer.game_programs = Some(Vec::new());
    for k in 0..60 {
        h.state_mut().viewport.as_mut().unwrap().renderer.wait_for_game_programs();
        // (The camera turned a little: the view is drawn again.)
        if let Some(o) = h.state_mut().area_views.get_mut(&area).and_then(|v| v.orbit.as_mut()) {
            o.yaw += if k % 2 == 0 { 0.001 } else { -0.001 };
        }
        h.step();
        let r = &h.state().viewport.as_ref().unwrap().renderer;
        if r.drawn.game_pending == 0 && r.drawn.textures_pending == 0 {
            break;
        }
    }
    h.step();
    let r = &h.state().viewport.as_ref().unwrap().renderer;
    r.game_programs.iter().flatten().count()
}

/// A case's area, built and looked at as the case asks, before anything
/// is placed or led.
struct Set {
    h: Harness<'static, Moonglow>,
    area: ResRef,
    g: mg_area::walk::Ground,
    /// The raised tile and the other tile of the edge (indices).
    raised: usize,
    other: usize,
    /// The raised tile's level, and the other tile's.
    high: f32,
    low: f32,
}

impl Set {
    /// The level of the tile at `p`.
    fn level_at(&self, p: Vec2) -> f32 {
        let i = tile_index(p, width(&self.h, self.area));
        self.g.tile_level(i).unwrap_or(0.0)
    }

    /// The two levels, lowest first.
    fn span(&self) -> (f32, f32) {
        (self.high.min(self.low), self.high.max(self.low))
    }
}

/// Builds the case's area and checks that the set-up is real: both tiles
/// of the edge have walkmeshes, at different levels where the case
/// raised one.
fn setup(case: &Case) -> Option<Set> {
    let (mut h, area) = harness(case)?;
    eprintln!("CASE {case:?}");
    let raised = build(&mut h, area, case);
    if case.shaders {
        let drawn = game_programs(&mut h, area);
        eprintln!("models drawn by the game's programs: {drawn}");
        assert!(drawn > 0, "the game's shaders draw the area");
    }
    let g = ground(&h, area);
    let w = width(&h, area);
    let step = h.state().area_views[&area].model.as_ref().unwrap().height_step;
    let high = g.tile_level(raised).expect("the raised tile's walkmesh");
    let other = other_tile(case, w, raised);
    let low = g.tile_level(other).expect("the other tile's walkmesh");
    eprintln!("levels: raised tile {raised} at {high}, tile {other} at {low}, step {step}");
    if case.steps != case.other_steps {
        assert!((high - low).abs() > 0.1, "the levels differ: {high} and {low}");
    }
    h.state_mut().settings.snap_grid = case.snap.then_some(100);
    h.state_mut().settings.place_onto_objects = case.onto;
    if case.angled
        && let Some(o) = h.state_mut().area_views.get_mut(&area).and_then(|v| v.orbit.as_mut())
    {
        let mid = (case.from + case.to) / 2.0;
        // (Both tiles in sight, the raised one too.)
        let top = high.max(low) - high.min(low);
        o.target = mid.extend(high.min(low) + top / 2.0);
        (o.distance, o.pitch, o.yaw) = (34.0 + 2.0 * top, case.pitch, case.yaw.to_radians());
    }
    h.state_mut().area_views.get_mut(&area).unwrap().walkmesh = case.walkmesh;
    h.state_mut().area_views.get_mut(&area).unwrap().object_walkmesh = case.object_walkmesh;
    h.run_steps(3);
    Some(Set { h, area, g, raised, other, high, low })
}

/// The ray from the camera through a point of the view, as the view
/// casts it (`AreaView::ray`).
fn ray_at(h: &Harness<'_, Moonglow>, area: ResRef, pos: egui::Pos2) -> mg_area::pick::Ray {
    let view = &h.state().area_views[&area];
    let camera = view.camera().expect("a camera");
    let r = view.rect;
    let aspect = (r.width() / r.height().max(1.0)).max(1e-3);
    let at = Vec2::new((pos.x - r.min.x) / r.width(), (pos.y - r.min.y) / r.height());
    mg_area::pick::Ray::from_screen(&camera, aspect, at)
}

/// Where on screen to let go for `aim` at the edge point `edge` (the
/// path from `from`), checked by the ray through it (a set-up check: the
/// view's own pixels can move); the point of the area meant; and where
/// the ray meets the ground.
fn aimed(set: &Set, from: Vec2, edge: Vec2, aim: Aim) -> (egui::Pos2, Vec2, Option<Vec3>) {
    let d = (edge - from).normalize();
    let (target, meant) = match aim {
        Aim::Near => {
            let p = edge - d * 0.1;
            (on(&set.g, p, set.level_at(p)), p)
        }
        Aim::Far => {
            let p = edge + d * 0.1;
            (on(&set.g, p, set.level_at(p)), p)
        }
        Aim::Gap => {
            let (low, high) = set.span();
            (edge.extend((low + high) / 2.0), edge)
        }
    };
    let pos = screen(&set.h, set.area, target);
    let hit = set.g.hit(&ray_at(&set.h, set.area, pos));
    let w = width(&set.h, set.area);
    // (Over the gap the ray passes under the raised tile: it meets the
    // ground behind it, or none.)
    let beyond = |p: Vec3| ![set.raised, set.other].contains(&tile_index(p.truncate(), w));
    match aim {
        Aim::Gap => assert!(
            hit.is_none_or(beyond),
            "set-up: the ray at {target} meets the edge's tiles at {hit:?}"
        ),
        _ => assert!(
            hit.is_some_and(|p| (p - target).length() < 0.3),
            "set-up: the ray at {target} meets the ground at {hit:?}"
        ),
    }
    eprintln!("let go at {pos:?}: {aim:?}, the ray meets {hit:?}");
    (pos, meant, hit)
}

/// A case led: where it ended, for [`run`]'s checks and for more.
struct Done {
    set: Set,
    kind: ObjectKind,
    /// The object led (or the one put down), and the first object's
    /// place before.
    index: usize,
    p0: Vec3,
    p1: Vec3,
    second: Option<(usize, Vec3)>,
    /// Where the pointer was let go (in the area).
    end: Vec2,
    /// Objects of the kind before.
    before: usize,
    /// Where the pointer was let go when aimed: how, on screen, and where
    /// its ray meets the ground.
    let_go: Option<(Aim, egui::Pos2, Option<Vec3>)>,
    /// The tiles the gesture changed.
    tiles_changed: usize,
}

/// The palette's list with `blueprint`'s kind and its row in sight; the
/// row's place.
fn palette_row(h: &mut Harness<'_, Moonglow>, blueprint: &str, kind: ObjectKind) -> egui::Pos2 {
    let key = ResKey::parse(blueprint, rtype(kind)).unwrap();
    show_palette(h.state_mut());
    h.run_steps(3);
    {
        let p = &mut h.state_mut().palette;
        p.kind = mg_module::palette::BlueprintKind::Placeable;
        (p.tiles, p.custom) = (false, false);
        p.filter = blueprint.into();
    }
    h.state_mut().settings.palette_gallery = false;
    h.run_steps(5);
    settle(h);
    let name = {
        let game = h.state().game.as_deref().unwrap();
        let utp = Gff::read(&game.resman.get(&key).unwrap()).unwrap();
        game.locstring(utp.root.locstring("LocName").unwrap()).unwrap()
    };
    // (The palette's row, right of the area; the module tree has one too.)
    let row = h
        .get_all_by_label(&name)
        .map(|n| n.rect().center())
        .find(|c| c.x > 842.0)
        .expect("the blueprint's row in the palette");
    eprintln!("the palette's row {name} at {row:?}");
    row
}

/// Leads a case: builds the area, places the object, leads it (or the
/// ghost, or the palette's row) from its tile across the edge, and lets
/// go. No checks of where it ended.
fn drive(case: &Case) -> Option<Done> {
    let mut set = setup(case)?;
    let (h, area) = (&mut set.h, set.area);
    let index = place(h, area, case, case.from);
    let second = case.second.map(|at| place(h, area, case, at));
    let p0 = position(h, area, case.kind, index);
    let q0 = second.map(|i| position(h, area, case.kind, i));
    eprintln!("placed at {p0} (and {q0:?})");
    let shots = mg_testkit::output_dir("tile-edges").join(case.name);
    std::fs::create_dir_all(&shots).unwrap();
    h.hover_at(egui::pos2(960.0, 610.0));
    h.run_steps(10);
    h.render().unwrap().save(shots.join("before.png")).unwrap();
    let tiles_before = tile_list(h, area);

    // The path: steps of 10 cm from the object to the end.
    let n = ((case.to - case.from).length() / 0.1).ceil().max(1.0) as usize;
    let path: Vec<Vec2> = (0..=n).map(|k| case.from.lerp(case.to, k as f32 / n as f32)).collect();
    let near = p0.z - case.lift;
    let pts: Vec<Vec3> = path.iter().map(|p| on(&set.g, *p, near)).collect();
    let gap = case.gap.then_some(set.span());
    let aim = case.aim.map(|a| aimed(&set, case.from, case.to, a));
    let h = &mut set.h;
    let shift = egui::Modifiers::SHIFT;
    let kind = case.kind;
    let mut watch = Watch { last: Vec::new(), frames: 0, jumps: Vec::new() };
    let object = {
        let model = h.state().area_views[&area].model.as_ref().unwrap();
        model.objects.iter().position(|o| o.kind == kind && o.index == index).unwrap()
    };
    let before = count(h, area, kind);
    let end_at = aim.map_or_else(|| screen(h, area, on(&set.g, case.to, near)), |a| a.0);
    // The pointer's last place: the path's end, or where it is aimed.
    let finish = |h: &mut Harness<'_, Moonglow>, watch: &mut Watch, end: egui::Pos2| {
        let end = aim.map_or(end, |a| a.0);
        h.hover_at(end);
        for _ in 0..4 {
            frame(h, area, watch, end);
        }
        end
    };
    match case.how {
        How::Free => {
            if let Some(i) = second {
                h.state_mut().area_views.get_mut(&area).unwrap().selection =
                    vec![(kind, index), (kind, i)];
                h.run_steps(3);
            }
            // Pressed on the object (picked there), led on the ground. (An
            // outline is picked by its shape, not its position: pressed
            // where it is picked, the path led as far from the object.)
            let mut a = screen(h, area, p0);
            if h.state().area_views[&area].pick(a) != Some(object) {
                let view = &h.state().area_views[&area];
                let found = (0..40 * 40)
                    .map(|k| {
                        a + egui::vec2((k % 40) as f32 * 3.0 - 60.0, (k / 40) as f32 * 3.0 - 60.0)
                    })
                    .filter(|p| view.pick(*p) == Some(object))
                    .min_by(|p, q| p.distance(a).total_cmp(&q.distance(a)));
                if let Some(p) = found {
                    a = p;
                }
            }
            let grab = h.state().area_views[&area].focus_at(a).map_or(Vec2::ZERO, |g| {
                if kind.has_outline() { g.truncate() - p0.truncate() } else { Vec2::ZERO }
            });
            let pts: Vec<Vec3> = path.iter().map(|p| on(&set.g, *p + grab, near)).collect();
            let edges: Vec<Vec2> = case.edges.iter().map(|e| *e + grab).collect();
            let picked = h.state().area_views[&area].pick(a);
            eprintln!("picked at the press: {picked:?} (the object: {object})");
            if matches!(case.leave, Leave::Nothing | Leave::Brush) {
                assert_eq!(picked, Some(object), "the press is on the object");
            }
            h.hover_at(a);
            h.step();
            press(h, a, true, egui::Modifiers::NONE);
            frame(h, area, &mut watch, a);
            let end = if case.pixels {
                lead_pixels(h, area, &mut watch, a, end_at, 3.0)
            } else {
                lead(h, area, &mut watch, &pts, &edges, 6, gap)
            };
            let end = finish(h, &mut watch, end);
            h.render().unwrap().save(shots.join("held.png")).unwrap();
            press(h, end, false, egui::Modifiers::NONE);
        }
        How::SlideX | How::SlideY => {
            h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(kind, index)];
            h.run_steps(3);
            let axis = if case.how == How::SlideX { Vec3::X } else { Vec3::Y };
            let radius = h.state().area_views[&area].handle_radius_at(p0).unwrap();
            let head = p0 + axis * radius * 1.45;
            h.event(egui::Event::ModifiersChanged(shift));
            let a = screen(h, area, head);
            h.hover_at(a);
            h.run_steps(2);
            let arrow = h.state().area_views[&area].arrow_axis_at(a);
            eprintln!("arrow at the press: {arrow:?}");
            assert!(arrow.is_some(), "the press is on the arrow");
            // (The button's own event, Shift held: `press` lets Shift go.)
            let left = |pos, pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: shift,
            };
            h.event(left(a, true));
            frame(h, area, &mut watch, a);
            // (The arrow's head led as far as the object goes.)
            let pts: Vec<Vec3> = path.iter().map(|p| head + (*p - case.from).extend(0.0)).collect();
            let edges: Vec<Vec2> = case.edges.iter().map(|e| *e + (head - p0).truncate()).collect();
            let end = lead(h, area, &mut watch, &pts, &edges, 6, None);
            h.render().unwrap().save(shots.join("held.png")).unwrap();
            h.event(left(end, false));
            h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
        }
        How::Place | How::Paste => {
            if case.how == How::Place {
                let key = ResKey::parse(case.blueprint, rtype(kind)).unwrap();
                h.state_mut().palette.selected = Some(key);
            } else {
                h.state_mut().area_views.get_mut(&area).unwrap().selection = vec![(kind, index)];
                let a = screen(h, area, p0);
                h.hover_at(a);
                h.run_steps(2);
                h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::C);
                h.run_steps(1);
                h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::V);
                h.run_steps(2);
                assert!(h.state().area_views[&area].pasting, "pasting");
            }
            let a = screen(h, area, p0);
            h.hover_at(a);
            frame(h, area, &mut watch, a);
            let end = if case.pixels {
                lead_pixels(h, area, &mut watch, a, end_at, 3.0)
            } else {
                lead(h, area, &mut watch, &pts, &case.edges, 6, gap)
            };
            let end = finish(h, &mut watch, end);
            let view = &h.state().area_views[&area];
            eprintln!(
                "ghost at the end: {:?}, pasted shown {}",
                view.ghost_shown.as_ref().map(|g| g.position),
                view.pasted_shown
            );
            h.render().unwrap().save(shots.join("held.png")).unwrap();
            press(h, end, true, egui::Modifiers::NONE);
            press(h, end, false, egui::Modifiers::NONE);
        }
        How::Dnd => {
            // The row pressed, carried into the view (a frame a step), led
            // across the edge and let go there.
            let row = palette_row(h, case.blueprint, kind);
            h.hover_at(row);
            h.step();
            press(h, row, true, egui::Modifiers::NONE);
            frame(h, area, &mut watch, row);
            let into = screen(h, area, pts[0]);
            for k in 1..=12 {
                let at = row + (into - row) * (k as f32 / 12.0);
                h.hover_at(at);
                frame(h, area, &mut watch, at);
            }
            let end = lead(h, area, &mut watch, &pts, &case.edges, 6, gap);
            let end = finish(h, &mut watch, end);
            let view = &h.state().area_views[&area];
            eprintln!("ghost at the end: {:?}", view.ghost_shown.as_ref().map(|g| g.position));
            h.render().unwrap().save(shots.join("held.png")).unwrap();
            press(h, end, false, egui::Modifiers::NONE);
        }
    }
    for _ in 0..6 {
        frame(h, area, &mut watch, end_at);
    }
    settle(h);
    h.render().unwrap().save(shots.join("after.png")).unwrap();
    let undo = h.state().ws.as_ref().unwrap().can_undo().map(String::from);
    let now = count(h, area, kind);
    let moved = match case.how {
        How::Place | How::Paste | How::Dnd if now > before => before,
        _ => index,
    };
    let p1 = position(h, area, kind, moved);
    let tiles = tile_list(h, area);
    let changed: Vec<String> = (tiles_before.iter().zip(&tiles).enumerate())
        .filter(|(_, (a, b))| a != b)
        .map(|(i, (_, b))| {
            let f = |l: &str| b.integer(l).unwrap_or(-1);
            format!("{i}: id {} height {}", f("Tile_ID"), f("Tile_Height"))
        })
        .collect();
    eprintln!(
        "after: Select Tiles {}, tiles selected {:?}, Tile Properties open {}",
        h.state().area_views[&area].tile_mode,
        h.state().area_views[&area].tile_selection,
        h.state().tile_props.is_some()
    );
    eprintln!(
        "RESULT {}: from {p0} to {p1}, {} of the kind (was {before}), undo {undo:?}, tiles \
         changed {changed:?}, {} frames, {} jumps",
        case.name,
        now,
        watch.frames,
        watch.jumps.len()
    );
    // Where the pointer let go: the end, kept in the area.
    let (east, north) = h.state().area_views[&area].model.as_ref().unwrap().size();
    let to = aim.map_or(case.to, |a| a.1);
    let end = Vec2::new(to.x.clamp(0.0, east), to.y.clamp(0.0, north));
    let second = second.zip(q0);
    let let_go = case.aim.zip(aim).map(|(a, (pos, _, hit))| (a, pos, hit));
    let tiles_changed = changed.len();
    Some(Done { set, kind, index: moved, p0, p1, second, end, before, let_go, tiles_changed })
}

/// Runs a case ([`drive`]) and checks where it ended: where the pointer
/// let go, on the ground there, a second object moved alike.
fn run(case: Case) {
    let Some(done) = drive(&case) else { return };
    let Done { set, kind, p0, p1, second, end, before, let_go, tiles_changed, .. } = done;
    let mut h = set.h;
    let area = set.area;
    if !matches!(case.leave, Leave::Nothing | Leave::Brush) {
        // (What the gesture does with this state left is what is looked
        // at: said above, not held to the drag's.)
        return;
    }
    // (The owner's ruling, 2026-11-11: with Exact Tile chosen a press on
    // an object picks it; the drag from it paints no tile.)
    assert_eq!(tiles_changed, 0, "no tile changed");
    if matches!(case.how, How::Place | How::Paste | How::Dnd) {
        assert_eq!(count(&mut h, area, kind), before + 1, "one put down");
    }
    // (A lifted object is pressed on above the ground behind it; the
    // pointer led a few points a frame ends within a point's worth.)
    let reach = match () {
        _ if case.lift > 0.0 || case.how == How::Paste => 4.0,
        _ if case.pixels => 1.5,
        _ => 0.6,
    };
    let w = width(&h, area);
    if let Some((Aim::Gap, pos, hit)) = let_go {
        // Over the open gap beside a raised tile the object goes to what
        // is seen through the gap (the owner's ruling, 2026-11-11): the
        // ground the ray meets behind, under the pointer on screen; with
        // nothing behind, the area's edge. Never the raised tile.
        let tile = tile_index(p1.truncate(), w);
        assert_ne!(tile, set.raised, "not on the raised tile: {p1}");
        match hit {
            Some(g) => {
                let shown = h.state().area_views[&area].screen_pos(p1).expect("in view");
                assert!(shown.distance(pos) < 4.0, "under the pointer {pos:?}: at {shown:?}");
                assert_eq!(tile, tile_index(g.truncate(), w), "on the tile seen: {p1}, {g}");
            }
            None => {
                let (east, north) = h.state().area_views[&area].model.as_ref().unwrap().size();
                let on_edge =
                    [p1.x, p1.y, east - p1.x, north - p1.y].iter().any(|d| d.abs() < 0.01);
                assert!(on_edge, "nothing behind: on the area's edge, {p1}");
            }
        }
    } else {
        assert!(
            (p1.truncate() - end).length() < reach,
            "ends where the pointer let go, {end}: from {p0} to {p1}"
        );
    }
    if let Some((i, q0)) = second {
        let q1 = position(&h, area, kind, i);
        let (a, b) = ((p1 - p0).truncate(), (q1 - q0).truncate());
        eprintln!("the second: from {q0} to {q1}");
        assert!((a - b).length() < 0.05, "moved together: {a} and {b}");
    }
    let expected = on(&set.g, p1.truncate(), p1.z - case.lift).z + case.lift;
    if matches!(case.how, How::SlideX | How::SlideY) {
        // (An X or Y arrow keeps the height, by design: a builder's report.)
        assert!((p1.z - p0.z).abs() < 1e-3, "its height kept: {p0} to {p1}");
    } else if !kind.has_outline() && kind != ObjectKind::Door {
        assert!((p1.z - expected).abs() < 0.2, "on the ground where it ends: {p1}, {expected}");
    }
}

/// The tile index (in an area `width` tiles wide) a point is in.
fn tile_index(p: Vec2, width: u32) -> usize {
    let x = (p.x / 10.0).floor().clamp(0.0, width as f32 - 1.0) as usize;
    let y = (p.y / 10.0).floor().max(0.0) as usize;
    y * width as usize + x
}

/// The tilted view's set-up for the cases below: the raised tile three
/// steps up, seen from the south-west, the pointer held over the gap.
fn tilted(name: &'static str) -> Case {
    Case { angled: true, steps: 3, gap: true, ..Case::new(name) }
}

// Forced and height, each alone and together.

#[test]
fn edge_forced_equal_heights() {
    // (The two tiles' levels differ a little all the same: the wall's.)
    run(Case { steps: 0, ..Case::new("forced-equal") });
}

#[test]
fn edge_fitting_tile_raised() {
    run(Case { forced: Forced::No, ..Case::new("fitting-raised") });
}

#[test]
fn edge_forced_raised_three() {
    run(Case { steps: 3, ..Case::new("forced-three") });
}

#[test]
fn edge_forced_lowered() {
    run(Case { steps: -1, ..Case::new("forced-lowered") });
}

#[test]
fn edge_forced_raised_north() {
    run(Case {
        raised: (1, 2),
        to: Vec2::new(15.0, 25.0),
        edges: vec![Vec2::new(15.0, 20.0)],
        ..Case::new("north")
    });
}

#[test]
fn edge_palette_exact_raised() {
    run(Case { forced: Forced::Palette, ..Case::new("palette") });
}

#[test]
fn edge_corner_of_four() {
    run(Case {
        raised: (2, 2),
        to: Vec2::new(25.0, 25.0),
        edges: vec![Vec2::new(20.0, 20.0)],
        ..Case::new("corner")
    });
}

#[test]
fn edge_past_the_area_s_edge() {
    run(Case {
        raised: (3, 1),
        from: Vec2::new(25.0, 15.0),
        to: Vec2::new(44.0, 15.0),
        edges: vec![Vec2::new(30.0, 15.0), Vec2::new(40.0, 15.0)],
        ..Case::new("outer-edge")
    });
}

// Both tiles at heights of their own ("two tiles that are on altered
// heights"), one of them forced.

#[test]
fn edge_both_raised_two_and_five() {
    run(Case { steps: 5, other_steps: 2, ..tilted("both-2-5") });
}

#[test]
fn edge_both_raised_five_and_two_back() {
    run(Case {
        steps: 2,
        other_steps: 5,
        from: Vec2::new(25.0, 15.0),
        to: Vec2::new(15.0, 15.0),
        ..tilted("both-5-2-back")
    });
}

#[test]
fn edge_one_up_one_down() {
    // (Seen from above: from the side the sunk tile's floor is hidden by
    // its neighbors, and the pointer led over it is on them.)
    run(Case { steps: 1, other_steps: -2, ..Case::new("one-up-one-down") });
}

#[test]
fn edge_one_up_one_down_palette() {
    run(Case {
        forced: Forced::Palette,
        steps: -2,
        other_steps: 1,
        ..Case::new("down-up-palette")
    });
}

// The tilted view and the gap beside the raised tile.

#[test]
fn edge_angled() {
    run(Case { angled: true, ..Case::new("angled") });
}

#[test]
fn edge_angled_high_to_low() {
    run(Case {
        angled: true,
        steps: 3,
        from: Vec2::new(25.0, 15.0),
        to: Vec2::new(15.0, 15.0),
        ..Case::new("angled-high-to-low")
    });
}

#[test]
fn edge_angled_gap() {
    run(tilted("angled-gap"));
}

#[test]
fn edge_angled_pixels() {
    run(Case { angled: true, steps: 3, pixels: true, ..Case::new("angled-pixels") });
}

#[test]
fn edge_angled_pixels_high_to_low() {
    run(Case {
        angled: true,
        steps: 3,
        pixels: true,
        from: Vec2::new(25.0, 15.0),
        to: Vec2::new(15.0, 15.0),
        ..Case::new("angled-pixels-back")
    });
}

#[test]
fn edge_from_the_high_side_low_pitch() {
    // (Looking from the north-east, low: the raised tile's open side
    // faces away; the low tile is seen past its top.)
    run(Case { steps: 6, yaw: 60.0, pitch: 0.2, ..tilted("high-side-low-pitch") });
}

#[test]
fn edge_low_pitch_pixels() {
    run(Case {
        angled: true,
        steps: 6,
        pixels: true,
        pitch: 0.15,
        ..Case::new("low-pitch-pixels")
    });
}

// Let go on the edge, the pointer aimed from the area (the ray through
// it checked: on the first tile, on the second, or in the gap between).

#[test]
fn edge_let_go_just_before_the_edge() {
    run(Case { to: Vec2::new(20.0, 15.0), aim: Some(Aim::Near), ..tilted("let-go-near") });
}

#[test]
fn edge_let_go_just_past_the_edge() {
    run(Case { to: Vec2::new(20.0, 15.0), aim: Some(Aim::Far), ..tilted("let-go-far") });
}

#[test]
fn edge_let_go_from_above_on_the_edge() {
    // (From straight above the ray meets one tile or the other.)
    run(Case { to: Vec2::new(20.0, 15.0), aim: Some(Aim::Far), ..Case::new("let-go-above") });
}

/// Let go with the pointer over the gap on the edge: the ray passes under
/// the raised tile, and the chest goes to what is seen through the gap
/// (`AreaView::ground_at`): the ground behind, under the pointer, or with
/// nothing behind the area's edge; not the raised tile (the owner's
/// ruling of 2026-11-11: kept as it is).
#[test]
fn edge_let_go_over_the_gap() {
    run(Case { to: Vec2::new(20.0, 15.0), aim: Some(Aim::Gap), ..tilted("let-go-gap") });
}

// Snap, arrows, lifted, overlays.

#[test]
fn edge_snapped_angled() {
    run(Case { snap: true, angled: true, ..Case::new("snap-angled") });
}

#[test]
fn edge_slide_x_angled() {
    run(Case { how: How::SlideX, angled: true, ..Case::new("slide-x-angled") });
}

#[test]
fn edge_slide_y() {
    run(Case {
        how: How::SlideY,
        raised: (1, 2),
        to: Vec2::new(15.0, 25.0),
        edges: vec![Vec2::new(15.0, 20.0)],
        ..Case::new("slide-y")
    });
}

#[test]
fn edge_lifted_angled() {
    run(Case { lift: 2.0, angled: true, ..Case::new("lifted-angled") });
}

#[test]
fn edge_walkmesh_shown_angled() {
    run(Case { walkmesh: true, angled: true, steps: 3, ..Case::new("walkmesh-angled") });
}

#[test]
fn edge_object_walkmesh_shown_angled() {
    // (The chest, plc_a08, has its own walkmesh.)
    run(Case { object_walkmesh: true, angled: true, ..Case::new("object-walkmesh") });
}

#[test]
fn edge_fade_and_tile_border() {
    run(Case { fade_border: true, ..tilted("fade-border") });
}

#[test]
fn edge_outer_edge_angled_border() {
    run(Case {
        raised: (3, 1),
        from: Vec2::new(25.0, 15.0),
        to: Vec2::new(44.0, 15.0),
        edges: vec![Vec2::new(30.0, 15.0), Vec2::new(40.0, 15.0)],
        fade_border: true,
        ..tilted("outer-edge-angled")
    });
}

// A forced cliff: high ground against a low neighbor, no cliff between.

#[test]
fn edge_forced_cliff_high_side_angled() {
    run(Case {
        cliff: true,
        steps: 0,
        angled: true,
        from: Vec2::new(35.0, 15.0),
        to: Vec2::new(25.0, 15.0),
        edges: vec![Vec2::new(30.0, 15.0)],
        ..Case::new("cliff-angled")
    });
}

#[test]
fn edge_forced_cliff_raised_angled_back() {
    run(Case {
        cliff: true,
        steps: 2,
        angled: true,
        from: Vec2::new(28.0, 15.0),
        to: Vec2::new(35.0, 15.0),
        edges: vec![Vec2::new(30.0, 15.0)],
        ..Case::new("cliff-raised-back")
    });
}

// The game's shaders (the release's default).

#[test]
fn edge_game_shaders_angled_gap() {
    run(Case { shaders: true, ..tilted("shaders-gap") });
}

#[test]
fn edge_game_shaders_pixels() {
    run(Case { shaders: true, angled: true, pixels: true, ..Case::new("shaders-pixels") });
}

// Placing, pasting and dropping from the palette across the edge (the
// ghost's way).

#[test]
fn edge_place_from_palette_angled_onto() {
    run(Case { how: How::Place, onto: true, ..tilted("place-onto") });
}

#[test]
fn edge_place_from_palette_snapped_onto() {
    run(Case { how: How::Place, snap: true, onto: true, ..Case::new("place-snap-onto") });
}

#[test]
fn edge_place_from_palette_pixels() {
    run(Case { how: How::Place, angled: true, pixels: true, ..Case::new("place-pixels") });
}

#[test]
fn edge_paste() {
    run(Case { how: How::Paste, ..Case::new("paste") });
}

#[test]
fn edge_paste_angled_onto() {
    run(Case { how: How::Paste, onto: true, ..tilted("paste-onto") });
}

#[test]
fn edge_palette_dragged_onto_the_high_tile() {
    run(Case { how: How::Dnd, ..tilted("dnd-high") });
}

#[test]
fn edge_palette_dragged_onto_the_low_tile() {
    run(Case {
        how: How::Dnd,
        from: Vec2::new(25.0, 15.0),
        to: Vec2::new(15.0, 15.0),
        ..tilted("dnd-low")
    });
}

#[test]
fn edge_palette_dragged_to_the_edge() {
    run(Case {
        how: How::Dnd,
        to: Vec2::new(20.0, 15.0),
        aim: Some(Aim::Far),
        ..tilted("dnd-edge")
    });
}

/// Dropped from the palette with the pointer over the gap: the chest is
/// placed on what is seen through the gap, as a drag puts it there.
#[test]
fn edge_palette_dragged_over_the_gap() {
    run(Case {
        how: How::Dnd,
        to: Vec2::new(20.0, 15.0),
        aim: Some(Aim::Gap),
        ..tilted("dnd-gap")
    });
}

// What forcing the tile leaves behind when the chest is pressed and
// dragged across the edge. With the Exact Tile brush still chosen the
// press picks the chest and the drag moves it, painting nothing (the
// owner's ruling of 2026-11-11); the other two are looked at, not held to
// a drag's outcome.

#[test]
fn edge_drag_with_the_exact_tile_brush_left() {
    run(Case { forced: Forced::Palette, leave: Leave::Brush, ..tilted("left-brush") });
}

#[test]
fn edge_drag_with_tiles_selected() {
    run(Case { leave: Leave::TileSelect, ..tilted("left-tiles-selected") });
}

#[test]
fn edge_drag_with_tile_properties_open() {
    run(Case { leave: Leave::PropsOpen, ..tilted("left-props-open") });
}

// Two selected, one each side.

#[test]
fn edge_two_selected_straddling_angled() {
    run(Case { second: Some(Vec2::new(22.0, 17.0)), ..tilted("two-angled") });
}

// The other kinds, once each across the same edge.

#[test]
fn edge_creature() {
    run(Case { kind: ObjectKind::Creature, blueprint: "nw_badger", ..Case::new("creature") });
}

#[test]
fn edge_item() {
    run(Case { kind: ObjectKind::Item, blueprint: "nw_wswls001", ..Case::new("item") });
}

#[test]
fn edge_waypoint() {
    run(Case { kind: ObjectKind::Waypoint, blueprint: "nw_waypoint001", ..Case::new("waypoint") });
}

#[test]
fn edge_store() {
    run(Case { kind: ObjectKind::Store, blueprint: "nw_storgenral001", ..Case::new("store") });
}

#[test]
fn edge_door() {
    run(Case { kind: ObjectKind::Door, blueprint: "nw_door_ttr_01", ..Case::new("door") });
}

#[test]
fn edge_sound() {
    run(Case { kind: ObjectKind::Sound, blueprint: "animalcriesday", ..Case::new("sound") });
}

#[test]
fn edge_trigger() {
    run(Case { kind: ObjectKind::Trigger, blueprint: "newgeneric", ..Case::new("trigger") });
}

#[test]
fn edge_encounter() {
    run(Case { kind: ObjectKind::Encounter, blueprint: "nw_giantevil", ..Case::new("encounter") });
}

// Another tileset, and an area not square.

#[test]
fn edge_city_exterior() {
    run(Case { tileset: "tcn01", steps: 2, ..tilted("city") });
}

#[test]
fn edge_crypt_interior() {
    // (Seen from above: from the side, the crypt's walls stand between
    // the camera and the floor the pointer is led over.)
    run(Case { tileset: "tic01", steps: 2, ..Case::new("crypt") });
}

#[test]
fn edge_wide_area() {
    run(Case { size: Some((7, 3)), steps: 2, ..tilted("wide") });
}

// Moves that are no pointer drag.

/// The chest's place.
fn chest(h: &Harness<'_, Moonglow>, area: ResRef, index: usize) -> Vec3 {
    position(h, area, ObjectKind::Placeable, index)
}

/// A few frames with the pointer over the view's middle (the area's
/// keys act while it is hovered).
fn over_view(h: &mut Harness<'_, Moonglow>, area: ResRef) {
    let c = h.state().area_views[&area].rect.center();
    h.hover_at(c);
    h.run_steps(3);
}

/// Adjust Location on the chest (its menu), X set to `x` alone, OK.
fn adjust_x(h: &mut Harness<'_, Moonglow>, area: ResRef, index: usize, x: f32) {
    let at = screen(h, area, chest(h, area, index));
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run_steps(3);
    h.get_by_label("Adjust Location…").click();
    h.run_steps(3);
    let a = h.state_mut().adjust.as_mut().expect("Adjust Location is open");
    assert_eq!(a.objects, [(ObjectKind::Placeable, index)]);
    a.position.x = x;
    a.changed = [true, false, false, false];
    h.get_by_label("OK").click();
    h.run_steps(4);
}

/// Adjust Location, Drop to Ground (G) and Page Up and Down's surfaces
/// with the chest at the edge and on the raised tile. (No key moves the
/// selection sideways: the arrow keys and WASD move the camera.)
#[test]
fn edge_moves_by_commands() {
    let case = Case { lift: 0.0, ..tilted("commands") };
    let Some(mut set) = setup(&case) else { return };
    let area = set.area;
    let index = place(&mut set.h, area, &case, Vec2::new(19.9, 15.0));
    let h = &mut set.h;
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(ObjectKind::Placeable, index)];
    over_view(h, area);
    let mut seen = Vec::new();
    let mut note = |h: &mut Harness<'_, Moonglow>, what: &str| {
        h.run_steps(4);
        let p = chest(h, area, index);
        let undo = h.state().ws.as_ref().unwrap().can_undo().map(String::from);
        eprintln!("MOVE {what}: {p}, undo {undo:?}");
        assert!(p.is_finite(), "{what}: at {p}");
        seen.push((what.to_string(), p));
    };
    note(h, "placed");
    adjust_x(h, area, index, 20.0);
    note(h, "Adjust Location X 20 (the edge)");
    over_view(h, area);
    h.key_press(egui::Key::G);
    note(h, "G on the edge");
    h.key_press(egui::Key::PageUp);
    note(h, "Page Up on the edge");
    h.key_press(egui::Key::PageDown);
    note(h, "Page Down on the edge");
    adjust_x(h, area, index, 25.0);
    note(h, "Adjust Location X 25 (the raised tile)");
    over_view(h, area);
    h.key_press(egui::Key::PageUp);
    note(h, "Page Up on the raised tile");
    h.key_press(egui::Key::PageDown);
    note(h, "Page Down on the raised tile");
    h.key_press(egui::Key::G);
    note(h, "G on the raised tile");
    adjust_x(h, area, index, 19.9);
    note(h, "Adjust Location X 19.9 (back on the low tile)");
    over_view(h, area);
    h.key_press(egui::Key::G);
    note(h, "G on the low tile");
    let ground = on(&set.g, Vec2::new(19.9, 15.0), set.low).z;
    let last = seen.last().unwrap().1;
    assert!((last.z - ground).abs() < 0.2, "on the low tile's ground at last: {last}, {ground}");
}

/// The module's start location dragged by its ring across the edge and
/// back, held over the gap, in the tilted view.
#[test]
fn edge_start_location_dragged() {
    let case = tilted("start-location");
    let Some(mut set) = setup(&case) else { return };
    let area = set.area;
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let low = on(&set.g, Vec2::new(15.0, 15.0), set.low).z;
    {
        let edit = |label: &str, value: mg_gff::Value| mg_edit::Edit::SetField {
            key: ifo,
            path: mg_edit::GffPath::root(),
            label: label.into(),
            value: Some(value),
        };
        let edits = vec![
            edit("Mod_Entry_Area", mg_gff::Value::resref(area)),
            edit("Mod_Entry_X", mg_gff::Value::Float(15.0)),
            edit("Mod_Entry_Y", mg_gff::Value::Float(15.0)),
            edit("Mod_Entry_Z", mg_gff::Value::Float(low)),
            edit("Mod_Entry_Dir_X", mg_gff::Value::Float(1.0)),
            edit("Mod_Entry_Dir_Y", mg_gff::Value::Float(0.0)),
        ];
        set.h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    }
    set.h.run_steps(4);
    let entry = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let r = ws.doc(&ifo).unwrap().root.clone();
        let f = |l: &str| r.float(l).unwrap_or(f32::NAN);
        Vec3::new(f("Mod_Entry_X"), f("Mod_Entry_Y"), f("Mod_Entry_Z"))
    };
    let mut watch = Watch { last: Vec::new(), frames: 0, jumps: Vec::new() };
    let gap = Some(set.span());
    for (from, to) in [(15.0, 25.0), (25.0, 15.0)] {
        // By the ring's south side (a meter from its middle, at its own
        // height; the west side, seen from here, is near the arrow's
        // tip, which turns it), led along y = 14.
        let path: Vec<Vec3> = (0..=100)
            .map(|k| {
                let x = from + (to - from) * k as f32 / 100.0;
                let p = Vec2::new(x, 14.0);
                on(&set.g, p, set.level_at(p))
            })
            .collect();
        let was = entry(&mut set.h);
        let h = &mut set.h;
        // (Not selected: selected, its arrow's tip turns it, and from this
        // far the tip is a few points from the ring.)
        h.state_mut().area_views.get_mut(&area).unwrap().start_selected = false;
        h.run_steps(2);
        let a = screen(h, area, was - Vec3::Y);
        h.hover_at(a);
        h.run_steps(2);
        press(h, a, true, egui::Modifiers::NONE);
        frame(h, area, &mut watch, a);
        let end = lead(h, area, &mut watch, &path, &[Vec2::new(20.0, 14.0)], 6, gap);
        press(h, end, false, egui::Modifiers::NONE);
        h.run_steps(6);
        let at = entry(h);
        let undo = h.state().ws.as_ref().unwrap().can_undo().map(String::from);
        eprintln!("RESULT start location led {from} to {to}: at {at}, undo {undo:?}");
        assert!(at.is_finite(), "at {at}");
        assert_eq!(undo.as_deref(), Some("Move Start Location"));
        assert!((at.x - to).abs() < 0.6 && (at.y - 15.0).abs() < 0.6, "at {at}");
    }
}

/// An encounter's spawn point dragged by its post's foot across the edge
/// and back, held over the gap, in the tilted view.
#[test]
fn edge_spawn_point_dragged() {
    use mg_module::instances::{Placement, Placing, instance};
    let case =
        Case { kind: ObjectKind::Encounter, blueprint: "nw_giantevil", ..tilted("spawn-point") };
    let Some(mut set) = setup(&case) else { return };
    let area = set.area;
    let git = ResKey::new(area, ResType::GIT);
    let z = on(&set.g, Vec2::new(15.0, 15.0), set.low).z;
    {
        let app = set.h.state_mut();
        let game = app.game.as_deref().unwrap();
        let key = ResKey::parse("nw_giantevil", ResType::UTE).unwrap();
        let blueprint = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let outline = [[-1.0, -1.0, z], [1.0, -1.0, z], [1.0, 1.0, z], [-1.0, 1.0, z]];
        let at = Placement { position: [12.0, 15.0, z], rotation: 0.0 };
        let mut enc = instance(&placing, ResType::UTE, &blueprint, at, &outline).unwrap();
        let mut point = mg_gff::Struct::new(2);
        for (label, v) in [("X", 15.0), ("Y", 15.0), ("Z", z), ("Orientation", 0.0)] {
            point.set(label, mg_gff::Value::Float(v));
        }
        enc.set("SpawnPointList", mg_gff::Value::List(vec![point]));
        let edit = mg_edit::Edit::InsertItem {
            key: git,
            path: mg_edit::GffPath::root(),
            list: "Encounter List".into(),
            index: 0,
            item: enc,
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", vec![edit])));
    }
    set.h.run_steps(3);
    set.h.state_mut().area_views.get_mut(&area).unwrap().selection =
        vec![(ObjectKind::Encounter, 0)];
    set.h.run_steps(3);
    let point = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let enc = ws.doc(&git).unwrap().root.list("Encounter List").unwrap()[0].clone();
        let p = enc.list("SpawnPointList").unwrap()[0].clone();
        let f = |l: &str| p.float(l).unwrap_or(f32::NAN);
        Vec3::new(f("X"), f("Y"), f("Z"))
    };
    let mut watch = Watch { last: Vec::new(), frames: 0, jumps: Vec::new() };
    let gap = Some(set.span());
    for (from, to) in [(15.0, 25.0), (25.0, 15.0)] {
        let path: Vec<Vec3> = (0..=100)
            .map(|k| {
                let p = Vec2::new(from + (to - from) * k as f32 / 100.0, 15.0);
                on(&set.g, p, set.level_at(p))
            })
            .collect();
        let h = &mut set.h;
        let a = screen(h, area, path[0]);
        h.hover_at(a);
        h.run_steps(2);
        press(h, a, true, egui::Modifiers::NONE);
        frame(h, area, &mut watch, a);
        let end = lead(h, area, &mut watch, &path, &[Vec2::new(20.0, 15.0)], 6, gap);
        press(h, end, false, egui::Modifiers::NONE);
        h.run_steps(6);
        let at = point(h);
        let undo = h.state().ws.as_ref().unwrap().can_undo().map(String::from);
        eprintln!("RESULT spawn point led {from} to {to}: at {at}, undo {undo:?}");
        assert_eq!(undo.as_deref(), Some("Move Spawn Point"));
        assert!(at.is_finite(), "at {at}");
        assert!((at.x - to).abs() < 0.6 && (at.y - 15.0).abs() < 0.6, "at {at}");
        let ground = on(&set.g, at.truncate(), at.z).z;
        assert!((at.z - ground).abs() < 0.2, "on the ground there: {at}, {ground}");
    }
}

/// The chest let go over the gap (today put on the ground behind the
/// raised tile by `AreaView::ground_at`), then dragged again from
/// wherever it is to (15, 15), and taken back and forth through Undo and
/// Redo, frames drawn between.
#[test]
fn edge_stranded_chest_dragged_again_undone_and_redone() {
    let case = Case { to: Vec2::new(20.0, 15.0), aim: Some(Aim::Gap), ..tilted("stranded") };
    let Some(done) = drive(&case) else { return };
    let Done { mut set, index, p0, p1, .. } = done;
    let area = set.area;
    eprintln!("let go over the gap: from {p0} to {p1}");
    let back = Vec2::new(15.0, 15.0);
    let mut watch = Watch { last: Vec::new(), frames: 0, jumps: Vec::new() };
    let path: Vec<Vec3> = (0..=100)
        .map(|k| {
            let p = p1.truncate().lerp(back, k as f32 / 100.0);
            on(&set.g, p, set.level_at(p))
        })
        .collect();
    let h = &mut set.h;
    let a = screen(h, area, p1);
    let object = {
        let model = h.state().area_views[&area].model.as_ref().unwrap();
        model.objects.iter().position(|o| o.kind == ObjectKind::Placeable && o.index == index)
    };
    assert_eq!(h.state().area_views[&area].pick(a), object, "pressed on the chest where it is");
    h.hover_at(a);
    h.run_steps(2);
    press(h, a, true, egui::Modifiers::NONE);
    frame(h, area, &mut watch, a);
    let end = lead(h, area, &mut watch, &path, &[Vec2::new(20.0, 15.0)], 6, None);
    press(h, end, false, egui::Modifiers::NONE);
    h.run_steps(6);
    let p2 = chest(h, area, index);
    eprintln!("dragged again: to {p2}");
    assert!((p2.truncate() - back).length() < 0.6, "dragged back: {p2}");
    let step = |h: &mut Harness<'_, Moonglow>, action: mg_ui::Action| {
        h.state_mut().actions.push(action);
        for _ in 0..6 {
            h.step();
        }
        chest(h, area, index)
    };
    let close = |a: Vec3, b: Vec3| (a - b).length() < 1e-3;
    let u1 = step(h, mg_ui::Action::Undo);
    let u2 = step(h, mg_ui::Action::Undo);
    let r1 = step(h, mg_ui::Action::Redo);
    let r2 = step(h, mg_ui::Action::Redo);
    eprintln!("RESULT stranded: undo {u1}, {u2}; redo {r1}, {r2}");
    assert!(close(u1, p1) && close(u2, p0) && close(r1, p1) && close(r2, p2));
}

/// What the tile and group brushes' hover decision costs
/// (`area_view::object_takes_press`, asked every frame the pointer is over
/// the view with one of them in hand): a 32 by 32 rural area with 3,000
/// chests spread over it, three of them selected, seen whole. With each
/// of the four in hand (Exact Tile's, the plain tile, a group, an exact
/// group), times the decision over 1,000 pointer places (on chests, on
/// bare ground, on the selection's turn ring), the ground under the
/// pointer for comparison (`AreaView::focus_at`, the ray against the
/// walkmeshes, what the brush's spot already cost), and whole frames with
/// the brush hovering; and frames with a terrain brush, which asks
/// nothing. By hand, in release:
/// `cargo test --release -p mg-ui --test app -- tile_edges::exact_tile_hover_decision_timing
/// --ignored --nocapture --test-threads=1`.
#[test]
#[ignore = "a hand-run timing"]
fn exact_tile_hover_decision_timing() {
    use std::time::{Duration, Instant};
    let stats = |mut v: Vec<Duration>| {
        v.sort();
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        (ms(v[v.len() / 2]), ms(v[v.len() * 99 / 100]), ms(*v.last().unwrap()), v.len())
    };
    // Each tile and group brush in hand (the decision is asked with each),
    // and a terrain brush (it asks nothing) for the frames to compare.
    let hands = crate::tiles::HANDS.map(Some);
    let mut said = false;
    for hand in hands.into_iter().chain([None]) {
        // (A harness each: a picture clicked again lets its brush go.)
        let Some((mut h, area, places)) = timing_area() else { return };
        if !said {
            let objects = h.state().area_views[&area].model.as_ref().unwrap().objects.len();
            eprintln!("{objects} objects, {} places", places.len());
            said = true;
        }
        match hand {
            Some(hand) => {
                crate::tiles::choose_hand(&mut h, hand);
            }
            None => crate::tiles::choose_tile_brush(&mut h, "Water"),
        }
        let name = hand.map_or("terrain (Water)", |h| h.name());
        h.hover_at(egui::pos2(960.0, 610.0));
        h.run_steps(3);
        if hand.is_some() {
            let mut decision = Vec::new();
            let mut by_kind: std::collections::BTreeMap<&str, (Vec<Duration>, usize)> =
                Default::default();
            let mut ground = Vec::new();
            for &(kind, at) in &places {
                let t = Instant::now();
                let taken = mg_ui::area_view::object_takes_press_in(h.state_mut(), area, at, false);
                let took = t.elapsed();
                decision.push(took);
                let e = by_kind.entry(kind).or_default();
                e.0.push(took);
                e.1 += usize::from(taken == Some(true));
                let view = &h.state().area_views[&area];
                let t = Instant::now();
                std::hint::black_box(view.focus_at(at));
                ground.push(t.elapsed());
            }
            let (m, p99, w, n) = stats(decision);
            eprintln!("{name} decision: median {m:.3} ms, 99th {p99:.3} ms, worst {w:.3} ms ({n})");
            for (kind, (v, taken)) in by_kind {
                let (m, p99, w, n) = stats(v);
                eprintln!(
                    "  {kind}: median {m:.3}, 99th {p99:.3}, worst {w:.3} ms ({taken} of {n} taken)"
                );
            }
            let (m, p99, w, _) = stats(ground);
            eprintln!("  ground under the pointer: median {m:.3}, 99th {p99:.3}, worst {w:.3} ms");
        }
        // Whole frames, the pointer moving over the view (chests and
        // ground), twice.
        for _ in 0..2 {
            let mut frames = Vec::new();
            for &(_, at) in places.iter().step_by(4) {
                h.hover_at(at);
                let t = Instant::now();
                h.step();
                frames.push(t.elapsed());
            }
            let (m, p99, w, n) = stats(frames);
            eprintln!("{name} frames: median {m:.2} ms, 99th {p99:.2} ms, worst {w:.2} ms ({n})");
        }
    }
}

/// The timing's area ([`exact_tile_hover_decision_timing`]): 3,000 chests
/// over a 32 by 32 rural area, three selected, and the 1,000 pointer
/// places, by what they are over.
#[allow(clippy::type_complexity)]
fn timing_area() -> Option<(Harness<'static, Moonglow>, ResRef, Vec<(&'static str, egui::Pos2)>)> {
    use mg_module::instances::{Placement, Placing, instance};
    let chests = |m: &mut mg_module::Module, game: &mg_rules::GameData, area: ResRef| {
        let key = ResKey::parse("plc_chest1", ResType::UTP).unwrap();
        let bp = Gff::read(&game.resman.get(&key).unwrap()).unwrap().root;
        let none = |_: ResRef| None;
        let placing = Placing { game, item: &none };
        let mut rng = fastrand::Rng::with_seed(11);
        let list: Vec<mg_gff::Struct> = (0..3000)
            .map(|_| {
                let p = [rng.f32() * 316.0 + 2.0, rng.f32() * 316.0 + 2.0, 0.0];
                let at = Placement { position: p, rotation: rng.f32() * std::f32::consts::TAU };
                instance(&placing, ResType::UTP, &bp, at, &[]).unwrap()
            })
            .collect();
        let git_key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&git_key).unwrap().unwrap();
        git.root.set("Placeable List", mg_gff::Value::List(list));
        m.set_gff(git_key, &git).unwrap();
    };
    let (mut h, area) = area_harness_with("exact-timing", "ttr01", None, 32, None, &chests)?;
    settle(&mut h);
    let picked: Vec<usize> = (0..3).collect();
    h.state_mut().area_views.get_mut(&area).unwrap().selection =
        picked.iter().map(|&i| (ObjectKind::Placeable, i)).collect();
    settle(&mut h);
    h.run_steps(5);
    let rect = h.state().area_views[&area].rect;
    // The places: 600 over the view on a grid, 300 on chests, 100 on the
    // selection's rings.
    let mut places = Vec::new();
    for k in 0..600 {
        let (x, y) = ((k % 30) as f32 + 0.5, (k / 30) as f32 + 0.5);
        places.push(("grid", rect.min + egui::vec2(x / 30.0, y / 20.0) * rect.size()));
    }
    for i in 0..300 {
        let p = position(&h, area, ObjectKind::Placeable, i * 10);
        if let Some(at) = h.state().area_views[&area].screen_pos(p + Vec3::Z * 0.3) {
            places.push(("chest", at));
        }
    }
    for k in 0..100 {
        let p = position(&h, area, ObjectKind::Placeable, k % 3);
        let r = h.state().area_views[&area].handle_radius_at(p).unwrap();
        let a = k as f32 * 0.37;
        if let Some(at) =
            h.state().area_views[&area].screen_pos(p + Vec3::new(a.cos(), a.sin(), 0.0) * r)
        {
            places.push(("ring", at));
        }
    }
    Some((h, area, places))
}
