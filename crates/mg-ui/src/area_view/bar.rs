//! What the Status Bar shows for an area's view (the UI redesign's M3,
//! T12): the hints of the state the view is in, as key caps and a word
//! each (the Main, AreaPaint, AreaTiles and GizmoStates boards), a notice
//! before them where the state says something that is no key (what is in
//! hand, why a stroke was refused), and the counts at the bar's right
//! (what is selected, the tile under a brush, the area's tiles and
//! objects). It replaces the line of words that stood under the area's
//! toolbar. Read from the view's state as the frame begins, before the
//! view is drawn: what the view changes in its frame shows in the next
//! (`Moonglow::ui` asks for that frame where the hints differ).

use std::sync::{Arc, OnceLock};

use egui::{Color32, Painter, Rect, Stroke, pos2};
use mg_area::ObjectKind;
use mg_uikit::status::{GlyphPainter, Hint, HintPart};

use super::{AreaView, Drag, brush, light_of_row};
use crate::Moonglow;
use crate::keys::Cmd;
use crate::shell::{Count, HintKeys, Shown};

/// The state's hints and counts for the area's view `view`.
pub(crate) fn status(app: &Moonglow, view: &AreaView, ctx: &egui::Context) -> Shown {
    let k = HintKeys::new(&app.keymap, ctx);
    let mut counts = selection_counts(view);
    let hints = hints(app, view, ctx, &k, &mut counts);
    if let Some(m) = &view.model {
        let n = m.objects.iter().filter(|o| view.show[o.kind.index()]).count();
        counts.push(Count::new(format!("{} by {} tiles", m.width, m.height)));
        counts.push(Count::new(format!("{n} objects")));
    }
    Shown { hints, counts }
}

/// "Right Click / Esc: Stop": what drops a tool in hand.
fn stop() -> Hint {
    Hint::either(["Right Click", "Esc"], "Stop")
}

/// "Esc: Stop", for a tool a right click does not drop.
fn esc_stop() -> Hint {
    Hint::keys(["Esc"], "Stop")
}

/// The hints of the state the view is in, the first thing it says first;
/// `counts` given what a brush's spot adds.
fn hints(
    app: &Moonglow,
    view: &AreaView,
    ctx: &egui::Context,
    k: &HintKeys,
    counts: &mut Vec<Count>,
) -> Vec<Hint> {
    let ctrl = k.ctrl();
    let turn = || k.either(Cmd::TurnLeft, Cmd::TurnRight, "Turn");
    let mut out = Vec::new();
    if let Some(paint) = crate::paint_mode::hints(app, k) {
        let more = match view.paint.more {
            0 => None,
            more => Some(Hint::note(format!("{more} more are laid than are drawn"))),
        };
        let mut paint = paint.into_iter();
        out.extend(paint.next());
        out.extend(more);
        for h in paint {
            out.push(h);
        }
    } else if let Some(b) = crate::terrain_mode::active(app, view) {
        use mg_area::terrain::Brush;
        // (The tile under the pointer, at the bar's right; why the last
        // stroke was refused, after the brush's name.)
        if let Some(at) = crate::terrain_mode::spot_text(view) {
            counts.insert(0, Count::new(at));
        }
        // (Named apart from the palette's own entry of the same name.)
        out.push(Hint::note(format!("In hand: {}", b.label)));
        out.extend(view.notice.clone().map(Hint::note));
        // (Over an object a tile's or group's press would leave to it:
        // what the press does there.)
        let leaves = view.over_object && crate::terrain_mode::objects_keep_their_presses(&b);
        let tile_menu = || Hint::keys(["Right Click"], "Tile menu");
        if leaves {
            let exact = if b.exact { "Exact Tile: " } else { "" };
            out.push(Hint::note(format!(
                "{exact}A click or drag here goes to the object (select, move or turn it) and \
                 places nothing"
            )));
        } else {
            let hints: Vec<Hint> = match b.brush {
                Brush::Crosser(_) => vec![Hint::keys(["Drag"], "Draw across tiles")],
                Brush::RaiseLower => {
                    vec![Hint::keys(["Click"], "Raise"), Hint::keys(["Right Click"], "Lower")]
                }
                Brush::Eraser => vec![
                    Hint::keys(["Click"], "Erase"),
                    Hint::keys(["Shift", "Click"], "Next variant"),
                    tile_menu(),
                ],
                Brush::Appearance { back: false } => {
                    vec![Hint::keys(["Click"], "Another of its kind"), tile_menu()]
                }
                Brush::Appearance { back: true } => {
                    vec![Hint::keys(["Click"], "The one before of its kind"), tile_menu()]
                }
                Brush::Rotate => vec![
                    Hint::keys(["Click"], "Turn a quarter"),
                    Hint::keys(["Shift", "Click"], "Turn clockwise"),
                    tile_menu(),
                ],
                Brush::Refine if b.target.is_some() => {
                    vec![Hint::either(["Click", "Drag"], "Make them the tile chosen")]
                }
                Brush::Refine => vec![
                    Hint::keys(["Click"], "Next that fits"),
                    Hint::keys(["Drag"], "The first's, in each tile of its kind passed"),
                ],
                Brush::Group(_) => {
                    vec![Hint::keys(["Click"], "Place"), Hint::keys(["Right Click"], "Turn")]
                }
                Brush::Tile(_) if b.exact => vec![
                    Hint::note("Exact Tile: its neighbors are left as they are"),
                    Hint::either(["Click", "Drag"], "Place this tile alone"),
                    Hint::keys(["Right Click"], "Turn"),
                ],
                Brush::Tile(_) => vec![
                    Hint::either(["Click", "Drag"], "Place"),
                    Hint::keys(["Right Click"], "Turn"),
                ],
                _ => vec![Hint::keys(["Click"], "Paint a corner"), tile_menu()],
            };
            out.extend(hints);
        }
        out.push(esc_stop());
    } else if view.redraw.is_some() {
        out.extend([
            Hint::note("Redraw Polygon"),
            Hint::keys(["Click"], "Corner"),
            Hint::keys(["Double Click"], "Close"),
            Hint::keys([k.alt(), "Click"], "Rectangle from the first corner"),
            stop(),
        ]);
    } else if view.pasting {
        out.push(Hint::note("Pasting"));
        out.push(Hint::keys(["Click"], "Place the copies"));
        out.extend(turn());
        out.push(stop());
    } else if app.palette.placing_start {
        out.extend([
            Hint::note("Placing the start location"),
            Hint::keys(["Click"], "Put it here, facing the view"),
            Hint::keys(["Shift", "Click"], "Put it here and go on"),
            stop(),
        ]);
    } else if let Some(taken) = &app.palette.placing_appearance {
        out.push(Hint::note(format!("Placing {}", taken.name)));
        out.push(Hint::keys(["Click"], "Place"));
        out.push(Hint::keys(["Shift", "Click"], "Place more"));
        out.extend(turn());
        out.push(stop());
    } else if let Some(light) = app.palette.placing_light {
        let name = light_of_row(app, light).map(|l| l.name);
        let name = name.unwrap_or_else(|| light.to_string());
        out.extend([
            Hint::note(format!("Placing a light ({name})")),
            Hint::keys(["Click"], "Place"),
            Hint::keys(["Shift", "Click"], "Place more"),
            stop(),
        ]);
    } else if let Some(key) = brush(app) {
        out.push(Hint::note(format!("Placing {}", key.resref)));
        match ObjectKind::from_restype(key.restype) {
            Some(kind) if kind.has_outline() => out.extend([
                Hint::keys(["Click"], "Corner"),
                Hint::keys(["Double Click"], "Close"),
                Hint::keys([k.alt(), "Click"], "Rectangle from the first corner"),
            ]),
            Some(ObjectKind::Door) => out.extend([
                Hint::keys(["Click"], "Place"),
                Hint::keys(["Shift", "Click"], "Place more"),
            ]),
            _ => {
                out.extend([
                    Hint::keys(["Click"], "Place"),
                    Hint::keys(["Shift", "Click"], "Place more"),
                ]);
                out.extend(turn());
                let surface = "Onto the next surface up or down";
                out.extend(k.either(Cmd::SurfaceAbove, Cmd::SurfaceBelow, surface));
            }
        }
        out.push(stop());
    } else if let Some(visual) = view.wheel_scale.as_ref().and(view.scaled().first()) {
        out.extend([
            Hint::note(format!("Scale {:.2}", visual.1.scale.x)),
            Hint::keys([ctrl, "Wheel"], "Scale"),
            Hint::new("Let go to keep").key(ctrl),
            Hint::keys(["Esc"], "Drop"),
        ]);
    } else if let Some(left) = view.wheel_refused.map(|at| 4.0 - (ctx.input(|i| i.time) - at))
        && left > 0.0
    {
        // (Said for four seconds: a frame asked for at their end.)
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
        out.extend([
            Hint::note("A static placeable takes no scale: clear Static in its properties first"),
            Hint::keys([ctrl, "Wheel"], "Zoom slowly"),
        ]);
    } else if view.tile_pasting {
        out.extend([Hint::note("Pasting tiles"), Hint::keys(["Click"], "Place"), stop()]);
    } else if view.tile_mode {
        out.extend(view.notice.clone().map(Hint::note));
        out.extend([
            Hint::keys(["Click"], "Select a tile"),
            Hint::keys(["Drag"], "Box"),
            Hint::keys(["Shift", "Click"], "Add"),
            Hint::keys([ctrl, "Click"], "Add or take out"),
            Hint::keys(["Right Click"], "Tile menu"),
            Hint::keys(["Shift", "Right Click"], "Next tile"),
            Hint::keys(["Shift", "Drag"], "Add a box"),
        ]);
    } else if let Some(drag) = view.drag.filter(|d| !matches!(d, Drag::Box { .. })) {
        out.push(Hint::keys(["Esc"], "Cancel"));
        out.push(if app.settings.middle_drag_moves {
            Hint::keys([k.alt(), "Middle Drag"], "Turn view")
        } else {
            Hint::keys(["Middle Drag"], "Turn view")
        });
        let (grid, angle) = view.snap;
        match drag {
            Drag::Turn { .. } | Drag::Spin { .. } | Drag::FaceSpawn { .. } => {
                if let Some(a) = angle {
                    let at = Hint::new(format!("Turn {a}°"));
                    out.push(at.part(HintPart::Icon(mg_uikit::Icon::Rotate)));
                }
            }
            Drag::Start { turn: true, .. } => {}
            Drag::Tilt { .. } | Drag::Scale { .. } => {}
            _ => {
                if let Some(m) = grid {
                    let at = Hint::new(format!("Snap {m} m"));
                    out.push(at.part(HintPart::Icon(mg_uikit::Icon::Magnet)));
                }
            }
        }
    } else if let Some(Drag::Box { .. }) = view.drag {
        out.push(Hint::keys(["Esc"], "Cancel"));
    } else if view.read_only.is_some() || view.selection.is_empty() {
        out.push(Hint::keys(["Click"], "Select"));
        if view.read_only.is_none() {
            out.push(Hint::keys(["Drag"], "Move or box"));
        }
        out.push(Hint::keys([ctrl, "Drag"], "Move view"));
        out.push(Hint::keys([ctrl, "Right Drag"], "Turn view"));
    } else if view.turn_ring
        && ctx.input(|i| i.modifiers.shift && !i.modifiers.command && !i.modifiers.alt)
    {
        // Shift held: the handles (GizmoStates' second state).
        out.push(Hint::new("Held").held("Shift"));
        out.push(Hint::new("Move along an axis").part(glyph(Glyph::Axes)));
        let model = view.model.as_ref();
        let takes = model.is_some_and(|m| view.chosen(m).any(|o| o.takes_visual_transform()));
        if takes {
            out.push(Hint::new("Tilt").part(glyph(Glyph::Tilt)));
            out.push(Hint::new("Scale").part(glyph(Glyph::Scale)));
        }
        out.push(Hint::keys(["Right Drag"], "Turn view"));
    } else {
        // Selected (the Main board's).
        out.push(Hint::keys(["Drag"], "Move"));
        if view.turn_ring {
            out.push(Hint::new("Turn").part(glyph(Glyph::Ring)));
            out.push(Hint::keys(["Shift"], "Handles"));
        } else {
            out.push(Hint::keys([k.alt(), "Right Drag"], "Turn"));
        }
        out.push(Hint::keys([k.alt(), "Drag"], "Raise"));
        out.extend(k.both(Cmd::TurnLeft, Cmd::TurnRight, "Turn by a step"));
        out.extend(k.hint(Cmd::DropToGround, "Drop to ground"));
    }
    out
}

/// What is selected, at the bar's right before the area's counts: the
/// tiles of Select Tiles ("3 by 2 tiles selected", "Tile (4, 4)
/// selected"); the objects ("1 selected", its kind, tag, blueprint, place
/// and facing in its tip; "3 selected").
fn selection_counts(view: &AreaView) -> Vec<Count> {
    if view.tile_mode {
        let tiles = &view.tile_selection;
        let text = match tiles.as_slice() {
            [] => return Vec::new(),
            [(x, y)] => format!("Tile ({x}, {y}) selected"),
            _ => {
                let (xs, ys) = (tiles.iter().map(|t| t.0), tiles.iter().map(|t| t.1));
                let (x0, x1) = (xs.clone().min().unwrap_or(0), xs.max().unwrap_or(0));
                let (y0, y1) = (ys.clone().min().unwrap_or(0), ys.max().unwrap_or(0));
                let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
                if (w * h) as usize == tiles.len() {
                    format!("{w} by {h} tiles selected")
                } else {
                    format!("{} tiles selected", tiles.len())
                }
            }
        };
        return vec![Count::new(text)];
    }
    match view.selection.as_slice() {
        [] => Vec::new(),
        [(kind, index)] => {
            let tip = view.model.as_ref().and_then(|m| m.object(*kind, *index)).map(|o| {
                format!(
                    "{:?} {} ({}) at {:.2}, {:.2}, {:.2}, facing {:.0}°",
                    kind,
                    o.tag,
                    o.template.map(|t| t.to_string()).unwrap_or_default(),
                    o.position.x,
                    o.position.y,
                    o.position.z,
                    o.facing().to_degrees().rem_euclid(360.0),
                )
            });
            vec![Count { text: "1 selected".into(), tip }]
        }
        many => vec![Count::new(format!("{} selected", many.len()))],
    }
}

/// The handles' glyphs the hints draw (Main's and GizmoStates' `.hg`),
/// in the handles' own colors (not the theme's: the view's picture is the
/// same in both).
#[derive(Clone, Copy)]
enum Glyph {
    /// The turning ring with its facing mark.
    Ring,
    /// The three arrows of the axes.
    Axes,
    /// The two tilting rings.
    Tilt,
    /// The scale handle on its stalk.
    Scale,
}

/// A glyph's width (`.hg svg`: 22 by 16).
const GLYPH_WIDTH: f32 = 22.0;

const RING: Color32 = Color32::from_rgb(0xff, 0xb0, 0x2e);
const RED: Color32 = Color32::from_rgb(0xf0, 0x56, 0x6a);
const GREEN: Color32 = Color32::from_rgb(0x86, 0xd1, 0x3a);
const BLUE: Color32 = Color32::from_rgb(0x4d, 0x9b, 0xff);
const WHITE: Color32 = Color32::from_rgb(0xee, 0xf0, 0xf3);

/// The dark line under each stroke (black at 55%).
fn under() -> Color32 {
    Color32::from_black_alpha(140)
}

/// The part of a hint that draws `which`, named for a screen reader; made
/// once.
fn glyph(which: Glyph) -> HintPart {
    static PAINTERS: OnceLock<[GlyphPainter; 4]> = OnceLock::new();
    let painters = PAINTERS.get_or_init(|| {
        [
            Arc::new(|p: &Painter, r: Rect, _: &_| paint_glyph(p, r, Glyph::Ring)) as GlyphPainter,
            Arc::new(|p: &Painter, r: Rect, _: &_| paint_glyph(p, r, Glyph::Axes)),
            Arc::new(|p: &Painter, r: Rect, _: &_| paint_glyph(p, r, Glyph::Tilt)),
            Arc::new(|p: &Painter, r: Rect, _: &_| paint_glyph(p, r, Glyph::Scale)),
        ]
    });
    let (i, name) = match which {
        Glyph::Ring => (0, "Turning ring"),
        Glyph::Axes => (1, "Axis arrows"),
        Glyph::Tilt => (2, "Tilt rings"),
        Glyph::Scale => (3, "Scale handle"),
    };
    HintPart::Glyph { width: GLYPH_WIDTH, name: name.into(), paint: painters[i].clone() }
}

/// Paints glyph `which` in `rect` (22 by 16, the board's own points).
fn paint_glyph(p: &Painter, rect: Rect, which: Glyph) {
    let at = |x: f32, y: f32| pos2(rect.left() + x, rect.top() + y);
    // (A line drawn twice: its dark under-stroke, then its color.)
    let line = |a: (f32, f32), b: (f32, f32), color: Color32| {
        p.line_segment([at(a.0, a.1), at(b.0, b.1)], Stroke::new(3.5, under()));
        p.line_segment([at(a.0, a.1), at(b.0, b.1)], Stroke::new(1.5, color));
    };
    let head = |points: [(f32, f32); 3], color: Color32| {
        let points: Vec<_> = points.iter().map(|&(x, y)| at(x, y)).collect();
        p.add(egui::Shape::convex_polygon(points.clone(), under(), Stroke::new(2.0, under())));
        p.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
    };
    let ellipse = |c: (f32, f32), r: (f32, f32), from: f32, to: f32, color: Color32| {
        let n = 24;
        let points: Vec<_> = (0..=n)
            .map(|i| {
                let t = from + (to - from) * i as f32 / n as f32;
                at(c.0 + r.0 * t.cos(), c.1 + r.1 * t.sin())
            })
            .collect();
        p.add(egui::Shape::line(points.clone(), Stroke::new(3.5, under())));
        p.add(egui::Shape::line(points, Stroke::new(1.5, color)));
    };
    let tau = std::f32::consts::TAU;
    match which {
        Glyph::Ring => {
            ellipse((8.5, 8.0), (7.0, 4.5), 0.0, tau, RING);
            head([(20.0, 8.0), (15.0, 4.8), (15.0, 11.2)], RING);
        }
        Glyph::Axes => {
            line((9.0, 9.0), (3.74, 12.5), RED);
            head([(0.0, 15.0), (2.5, 9.73), (5.82, 14.72)], RED);
            line((9.0, 9.0), (16.05, 12.85), GREEN);
            head([(20.0, 15.0), (14.17, 15.24), (17.05, 9.97)], GREEN);
            line((9.0, 9.0), (9.0, 4.5), BLUE);
            head([(9.0, 0.0), (12.0, 5.0), (6.0, 5.0)], BLUE);
        }
        Glyph::Tilt => {
            let half = std::f32::consts::FRAC_PI_2;
            ellipse((11.0, 8.0), (5.5, 6.5), -half, half, RED);
            ellipse((11.0, 8.0), (5.5, 6.5), half, 3.0 * half, GREEN);
        }
        Glyph::Scale => {
            p.line_segment([at(3.0, 14.0), at(15.0, 5.0)], Stroke::new(3.0, under()));
            p.line_segment([at(3.0, 14.0), at(15.0, 5.0)], Stroke::new(1.0, WHITE));
            let dark = Rect::from_min_size(at(11.5, -0.5), egui::vec2(9.0, 9.0));
            p.rect_filled(dark, 2.0, under());
            p.rect_filled(Rect::from_min_size(at(12.5, 0.5), egui::vec2(7.0, 7.0)), 1.0, WHITE);
        }
    }
}
