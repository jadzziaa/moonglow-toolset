//! The area viewer's dialogs: Adjust Location (`TdlgLocation`: an exact
//! position, bearing and visual transform for the selected objects) and Find Instance
//! (`TdlgFindInstance`: placed objects across the module by type, area,
//! blueprint and tag; a double click goes to one).

use egui::Ui;
use glam::Vec3;
use mg_area::ObjectKind;
use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_resman::ResKey;
use mg_uikit::Controls as _;

use crate::{Action, Moonglow, Tab};

/// What a window shows that is read from disk (the names in a folder,
/// the module's haks found there): made by `make` as the window opens,
/// and kept under `id` for as long as it is asked for every pass, not
/// read every frame. A window closed and opened again reads anew.
pub(crate) fn kept_while_shown<T: Clone + Send + Sync + 'static>(
    ctx: &egui::Context,
    id: egui::Id,
    make: impl FnOnce() -> T,
) -> T {
    let now = ctx.cumulative_pass_nr();
    let kept: Option<(u64, T)> = ctx.data(|d| d.get_temp(id));
    let value = match kept {
        Some((at, value)) if now <= at + 1 => value,
        _ => make(),
    };
    ctx.data_mut(|d| d.insert_temp(id, (now, value.clone())));
    value
}

/// The range of Adjust Location's fields that have none of their own (a
/// position, a turn, a translation): any number.
const FAR: std::ops::RangeInclusive<f32> = f32::MIN..=f32::MAX;

/// The Adjust Location window's values.
#[derive(Debug, Clone, PartialEq)]
pub struct AdjustLocation {
    pub area: ResRef,
    pub objects: Vec<(ObjectKind, usize)>,
    pub position: Vec3,
    /// Aurora's Bearing: degrees counter-clockwise, 0 facing north (the
    /// model's turn).
    pub bearing: f32,
    /// Which of X, Y, Z and the bearing were changed (only those are set
    /// on every object).
    pub changed: [bool; 4],
    /// The visual transform (EE), and whether it was changed.
    pub visual: mg_area::VisualTransform,
    pub visual_changed: bool,
    /// How fast the model's animations play (the visual transform's
    /// `AnimationSpeed`, 1 as made), and whether it was changed.
    pub speed: f32,
    pub speed_changed: bool,
    /// What the first object's visual transforms have that the window
    /// doesn't set ([`kept_notes`]): its parts' own (a creature's head…)
    /// and lerps in progress.
    pub kept: Vec<String>,
    /// Some object chosen takes a visual transform (creatures, items,
    /// doors and placeables that aren't static): else it isn't offered, as
    /// in Aurora.
    pub shaped: bool,
    /// How many of the objects chosen are static placeables, and how many
    /// of those have a visual transform in their file: the game draws a
    /// static placeable without one (the window says so).
    pub fixed: usize,
    pub fixed_shaped: usize,
    /// How many of the objects chosen that take a visual transform have a
    /// `ScaleY` or `ScaleZ` in their file other than their `ScaleX` (the
    /// one the game scales by): kept until Scale is changed (the window
    /// says so).
    pub uneven: usize,
    /// It is the module's start location (`Mod_Entry_*` in module.ifo),
    /// selected in the area's view, rather than objects.
    pub start: bool,
}

/// Adjust Location for the module's start location in `area`, at
/// `(position, facing)` (radians anticlockwise from east, the way its
/// arrow points): its position and bearing, as an object's.
pub(crate) fn adjust_start(area: ResRef, (position, facing): (Vec3, f32)) -> AdjustLocation {
    AdjustLocation {
        area,
        objects: Vec::new(),
        position,
        // (An object's bearing faces north at 0, as the start location's
        // arrow at a quarter turn from east.)
        bearing: (facing - std::f32::consts::FRAC_PI_2).to_degrees().rem_euclid(360.0),
        changed: [false; 4],
        visual: mg_area::VisualTransform::default(),
        visual_changed: false,
        speed: 1.0,
        speed_changed: false,
        kept: Vec::new(),
        shaped: false,
        fixed: 0,
        fixed_shaped: 0,
        uneven: 0,
        start: true,
    }
}

/// Adjust Location for the selection of an area's view (showing the first
/// object's location).
pub(crate) fn adjust(view: &crate::area_view::AreaView) -> Option<AdjustLocation> {
    let model = view.model.as_ref()?;
    let first = view.selection.first().and_then(|&(k, i)| model.object(k, i))?;
    let fixed: Vec<&mg_area::AreaObject> = view
        .selection
        .iter()
        .filter_map(|&(k, i)| model.object(k, i))
        .filter(|o| o.kind == ObjectKind::Placeable && o.is_static)
        .collect();
    Some(AdjustLocation {
        fixed: fixed.len(),
        fixed_shaped: fixed.iter().filter(|o| o.visual.is_some_and(|v| !v.is_identity())).count(),
        area: view.area,
        objects: view.selection.clone(),
        position: first.position,
        bearing: first.rotation.to_degrees().rem_euclid(360.0),
        changed: [false; 4],
        visual: first.visual.unwrap_or_default(),
        visual_changed: false,
        speed: first.scopes.iter().find(|s| s.scope == 0).map_or(1.0, |s| s.speed),
        speed_changed: false,
        kept: kept_notes(first.kind, &first.scopes),
        uneven: view
            .selection
            .iter()
            .filter_map(|&(k, i)| model.object(k, i))
            .filter(|o| o.takes_visual_transform())
            .filter_map(|o| o.visual)
            .filter(|v| v.scale.y != v.scale.x || v.scale.z != v.scale.x)
            .count(),
        shaped: view
            .selection
            .iter()
            .filter_map(|&(k, i)| model.object(k, i))
            .any(|o| o.takes_visual_transform()),
        start: false,
    })
}

/// Draws the open dialogs.
pub(crate) fn windows(app: &mut Moonglow, ui: &mut Ui) {
    adjust_window(app, ui);
    find_window(app, ui);
}

fn adjust_window(app: &mut Moonglow, ui: &mut Ui) {
    let Some(mut a) = app.adjust.take() else { return };
    let mut open = true;
    let mut cancel = false;
    let mut done = None;
    crate::widgets::dialog(ui.ctx(), "Adjust Location", &mut open)
        .resizable(false)
        .collapsible(false)
        .show(ui.ctx(), |ui| {
            if a.objects.len() > 1 {
                ui.weak(format!("{} objects: what you change is set on each", a.objects.len()));
            }
            if a.start {
                ui.weak("The module's start location");
            }
            egui::Grid::new("adjust").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                for (i, label) in ["X", "Y", "Z"].into_iter().enumerate() {
                    crate::widgets::field_label(ui, label);
                    // (A step of an arrow: a tenth of a meter.)
                    let r = crate::widgets::spin_draft(ui, &mut a.position[i], FAR, 0.1, |d| {
                        d.speed(0.05).max_decimals(3)
                    });
                    a.changed[i] |= r.changed();
                    ui.end_row();
                }
                crate::widgets::field_label(ui, "Bearing (°)")
                    .on_hover_text("0 faces north, 90 west (as Aurora shows it)");
                let r = crate::widgets::spin_draft(ui, &mut a.bearing, 0.0..=360.0, 1.0, |d| {
                    d.speed(1.0).max_decimals(2)
                });
                a.changed[3] |= r.changed();
                ui.end_row();
            });
            if a.shaped {
                ui.add_space(crate::widgets::SECTION_GAP);
                crate::widgets::section_heading(ui, "Visual Transforms");
            }
            let visual = egui::Grid::new("adjust-visual").num_columns(4).spacing([12.0, 6.0]);
            if a.shaped {
                visual.show(ui, |ui| {
                    crate::widgets::field_label(ui, "Scale");
                    let mut scale = a.visual.scale.x;
                    let r = crate::widgets::spin_draft(ui, &mut scale, 0.01..=100.0, 0.1, |d| {
                        d.speed(0.01).max_decimals(2)
                    });
                    if r.changed() {
                        a.visual.scale = Vec3::splat(scale);
                        a.visual_changed = true;
                        // (Written alike from now on.)
                        a.uneven = 0;
                    }
                    crate::widgets::field_label(ui, "Animation Speed").on_hover_text(
                        "How fast the model's animations play: 1 as made, 2 twice as fast",
                    );
                    let r = crate::widgets::spin_draft(ui, &mut a.speed, 0.0..=100.0, 0.1, |d| {
                        d.speed(0.01).max_decimals(2)
                    });
                    a.speed_changed |= r.changed();
                    ui.end_row();
                    // What each turn does, as the game draws it
                    // (`mg_area::VisualTransform::matrix`).
                    let turns = [
                        "Turns the model about its upright axis, as Bearing does",
                        "Tips the model forward or back, about its own X axis \
                         (east at Bearing 0)",
                        "Leans the model to one side, about its own Y axis \
                         (north at Bearing 0)",
                    ];
                    for (i, axis) in ["X", "Y", "Z"].into_iter().enumerate() {
                        crate::widgets::field_label(ui, format!("{axis} Rotation"))
                            .on_hover_text(turns[i]);
                        let turn = &mut a.visual.rotate[i];
                        let r = crate::widgets::spin_draft(ui, turn, FAR, 1.0, |d| {
                            d.speed(1.0).max_decimals(1)
                        });
                        a.visual_changed |= r.changed();
                        crate::widgets::field_label(ui, format!("{axis} Translation"));
                        let to = &mut a.visual.translate[i];
                        let r = crate::widgets::spin_draft(ui, to, FAR, 0.1, |d| {
                            d.speed(0.01).max_decimals(2)
                        });
                        a.visual_changed |= r.changed();
                        ui.end_row();
                    }
                });
            }
            if a.shaped && !a.kept.is_empty() {
                ui.add_space(crate::widgets::SECTION_GAP);
                ui.scope(|ui| {
                    ui.set_max_width(340.0);
                    if a.objects.len() > 1 {
                        ui.weak("The first of them:");
                    }
                    for note in &a.kept {
                        ui.add(egui::Label::new(egui::RichText::new(note).weak()).wrap());
                    }
                });
            }
            if let Some(note) = scale_note(&a) {
                ui.add_space(crate::widgets::SECTION_GAP);
                // (About as wide as the fields above: the window sizes
                // itself to what it holds.)
                ui.scope(|ui| {
                    ui.set_max_width(340.0);
                    ui.add(egui::Label::new(egui::RichText::new(note).weak()).wrap());
                });
            }
            if let Some(note) = static_note(&a) {
                ui.add_space(crate::widgets::SECTION_GAP);
                ui.colored_label(ui.visuals().warn_fg_color, note);
            }
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() || crate::widgets::enter(ui) {
                    done = Some(true);
                }
                if ui.button("Apply").clicked() {
                    done = Some(false);
                }
                if crate::widgets::cancel(ui) {
                    cancel = true;
                }
            });
        });
    if cancel {
        return;
    }
    if let Some(close) = done {
        apply_location(app, &a);
        a.changed = [false; 4];
        a.visual_changed = false;
        a.speed_changed = false;
        if close {
            return;
        }
    }
    if open {
        app.adjust = Some(a);
    }
}

/// What Adjust Location says of the static placeables chosen: the game
/// draws a static placeable without a visual transform (merged into the
/// area's tiles), whatever its file has, so none is offered for one;
/// `None` when none is chosen.
pub fn static_note(a: &AdjustLocation) -> Option<String> {
    if a.fixed == 0 {
        return None;
    }
    let which = if a.objects.len() == 1 {
        "This placeable is static".to_string()
    } else if a.fixed == 1 {
        "1 of these placeables is static".to_string()
    } else {
        format!("{} of these placeables are static", a.fixed)
    };
    // (Ticking Static here takes the transform off: such a file was made
    // elsewhere, or before Moonglow did so.)
    let kept = match a.fixed_shaped {
        0 => String::new(),
        1 if a.fixed == 1 => {
            " Its file has a visual transform all the same, which the game doesn't draw.".into()
        }
        1 => " 1 has a visual transform in its file all the same, which the game doesn't draw."
            .into(),
        n => format!(
            " {n} have a visual transform in their file all the same, which the game doesn't draw."
        ),
    };
    Some(format!(
        "{which}: the game draws static placeables without visual transforms, so none is \
         offered. Untick Static (Properties) to scale, tilt or move a model.{kept}"
    ))
}

/// What Adjust Location says of what an object's visual transforms have
/// that it doesn't set (`scopes`, its `VisTransformList`): a part's own
/// transform (a creature's head, tail, wings or cloak, drawn on that part;
/// an item's parts), and each lerp in progress (a script's, kept in a
/// saved game: drawn at its end, ended by a value set here). Nothing for
/// an object with neither.
pub fn kept_notes(kind: ObjectKind, scopes: &[mg_module::placement::Scoped]) -> Vec<String> {
    use mg_module::placement::scope_name;
    let mut out = Vec::new();
    for s in scopes {
        let named = scope_name(kind, s.scope);
        if s.scope != 0 {
            let t = s.transform;
            let mut parts = Vec::new();
            if t.scale[0] != 1.0 {
                parts.push(format!("scale {}", short(t.scale[0])));
            }
            if t.rotate != [0.0; 3] {
                let [x, y, z] = t.rotate.map(short);
                parts.push(format!("rotation {x}°, {y}°, {z}°"));
            }
            if t.translate != [0.0; 3] {
                let [x, y, z] = t.translate.map(short);
                parts.push(format!("translation {x}, {y}, {z}"));
            }
            if s.speed != 1.0 {
                parts.push(format!("animation speed {}", short(s.speed)));
            }
            if parts.is_empty() {
                parts.push("none that changes it".into());
            }
            let drawn = match (kind, s.scope) {
                (ObjectKind::Creature, 254 | 253 | 252 | 243) => ", drawn on it",
                _ => "",
            };
            let named = named.strip_prefix("its ").unwrap_or(&named).to_string();
            out.push(format!(
                "Its {named} has a visual transform of its own ({}){drawn}: kept as it is, set by \
                 a script, not here.",
                parts.join(", ")
            ));
        }
        for l in &s.lerps {
            let axis = match l.axis {
                "AnimationSpeed" => "animation speed".to_string(),
                a => {
                    let (what, x) = a.split_at(a.len() - 1);
                    let what = match what {
                        "Scale" => "scale",
                        "Rotate" => "rotation",
                        _ => "translation",
                    };
                    format!("{x} {what}")
                }
            };
            let whose = match s.scope {
                0 => String::new(),
                _ => format!(" of {named}"),
            };
            let along = (l.progress * 100.0).round().clamp(0.0, 100.0);
            let again = match l.repeats {
                0 => String::new(),
                n if n < 0 => ", again and again".to_string(),
                n => format!(", {n} times more"),
            };
            out.push(format!(
                "Its {axis}{whose} is changing (a script's lerp, kept in a saved game): from {} \
                 to {} over {} s, {along}% along{again}. Drawn at its end; set here, the lerp \
                 ends.",
                short(l.from),
                short(l.to),
                short(l.duration)
            ));
        }
    }
    out
}

/// What Adjust Location says when a file's `ScaleY` or `ScaleZ` differs
/// from its `ScaleX` (set by hand, or by another tool): the game scales by
/// `ScaleX` alone, which Scale shows; the other two are kept until Scale
/// is changed, which writes the three alike. `None` when none differs.
pub fn scale_note(a: &AdjustLocation) -> Option<String> {
    if a.uneven == 0 || !a.shaped {
        return None;
    }
    let which = if a.objects.len() == 1 {
        let s = a.visual.scale;
        format!(
            "Its file has ScaleX {}, ScaleY {} and ScaleZ {}",
            short(s.x),
            short(s.y),
            short(s.z)
        )
    } else if a.uneven == 1 {
        "1 of these objects has a ScaleY or ScaleZ in its file other than its ScaleX".into()
    } else {
        format!(
            "{} of these objects have a ScaleY or ScaleZ in their file other than their ScaleX",
            a.uneven
        )
    };
    Some(format!(
        "{which}: the game scales a model by ScaleX alone, which Scale shows. ScaleY and \
         ScaleZ are kept as they are until Scale is changed, which writes the three alike."
    ))
}

/// Objects moved or turned (by their places in the area's model, where
/// they would stand and how they would turn), and visual transforms given.
pub(crate) type Previewed = (Vec<(usize, Vec3, f32)>, Vec<(usize, mg_area::VisualTransform)>);

/// What the area's view shows while Adjust Location is changed, before OK
/// or Apply (a builder's report: only Apply moved them): each object it
/// moves or turns (of the area model's `objects`), and each one's visual
/// transform where that was changed. Nothing for the start location
/// ([`previewed_start`]).
pub(crate) fn previewed(a: &AdjustLocation, objects: &[mg_area::AreaObject]) -> Previewed {
    let mut moved = Vec::new();
    let mut visual = Vec::new();
    if a.start {
        return (moved, visual);
    }
    // (Where each is among the area's objects: looked up, not found by
    // going through them for every object of a large selection.)
    let places: std::collections::HashMap<(ObjectKind, usize), usize> = match a.objects.len() {
        0..=4 => Default::default(),
        _ => objects.iter().enumerate().map(|(at, o)| ((o.kind, o.index), at)).collect(),
    };
    for &(kind, index) in &a.objects {
        let at = match places.is_empty() {
            true => objects.iter().position(|o| o.kind == kind && o.index == index),
            false => places.get(&(kind, index)).copied(),
        };
        let Some(i) = at else { continue };
        let o = &objects[i];
        // (As `apply_location` sets them.)
        let mut p = o.position;
        for axis in 0..3 {
            if a.changed[axis] {
                p[axis] = a.position[axis];
            }
        }
        let rotation = if a.changed[3] { a.bearing.to_radians() } else { o.rotation };
        if p != o.position || rotation != o.rotation {
            moved.push((i, p, rotation));
        }
        if a.visual_changed && o.takes_visual_transform() {
            visual.push((i, a.visual));
        }
    }
    (moved, visual)
}

/// Where the start location shows while Adjust Location is changed for
/// it, before OK or Apply: `at` (where it is, facing) with the window's
/// changed parts.
pub(crate) fn previewed_start(
    a: &AdjustLocation,
    (mut at, mut facing): (Vec3, f32),
) -> (Vec3, f32) {
    for axis in 0..3 {
        if a.changed[axis] {
            at[axis] = a.position[axis];
        }
    }
    if a.changed[3] {
        facing = a.bearing.to_radians() + std::f32::consts::FRAC_PI_2;
    }
    (at, facing)
}

/// Sets the changed parts of the start location (a command each, as its
/// marker's drag and turn are).
fn apply_start(app: &mut Moonglow, a: &AdjustLocation) {
    if a.changed[..3].iter().any(|c| *c) {
        crate::area_view::move_start_location(app, a.position);
    }
    if a.changed[3] {
        let facing = a.bearing.to_radians() + std::f32::consts::FRAC_PI_2;
        crate::area_view::turn_start_location(app, facing);
    }
}

/// A number as short as it reads (`0.5`, `1`, `1.25`).
fn short(v: f32) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Sets the changed parts of the location on each object (one command).
fn apply_location(app: &mut Moonglow, a: &AdjustLocation) {
    if a.start {
        apply_start(app, a);
        return;
    }
    let git = ResKey::new(a.area, ResType::GIT);
    let (Some(view), Some(ws)) = (app.area_views.get(&a.area), app.ws.as_mut()) else { return };
    let Some(model) = &view.model else { return };
    let Ok(doc) = ws.doc(&git) else { return };
    let mut edits = Vec::new();
    for &(kind, index) in &a.objects {
        let (Some(o), Some(s)) =
            (model.object(kind, index), doc.root.list(kind.list()).and_then(|l| l.get(index)))
        else {
            continue;
        };
        let mut p = o.position;
        for i in 0..3 {
            if a.changed[i] {
                p[i] = a.position[i];
            }
        }
        let rotation = if a.changed[3] { a.bearing.to_radians() } else { o.rotation };
        edits.extend(mg_area::edit::move_edits(git, o, s, p, rotation));
        if (a.visual_changed || a.speed_changed) && o.takes_visual_transform() {
            // (What wasn't changed here, each its own: a speed alone
            // changed leaves the object's transform as it was.)
            let v = match a.visual_changed {
                true => a.visual,
                false => o.visual.unwrap_or_default(),
            };
            let speed = a.speed_changed.then_some(a.speed);
            edits.extend(mg_area::edit::visual_transform_edits_with(git, o, s, v, speed));
        }
    }
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new("Adjust Location", edits)));
    }
}

/// The Find Instance window's criteria and results.
#[derive(Debug, Clone, PartialEq)]
pub struct FindInstance {
    /// Which kinds to look for, by [`ObjectKind::index`].
    pub kinds: [bool; 9],
    /// `None`: every area.
    pub area: Option<ResRef>,
    pub template: String,
    pub tag: String,
    pub results: Vec<Found>,
    /// How many times each area's GIT with a result in it had its lists
    /// rewritten as they were found (`Workspace::rewrites`): a result is
    /// a place in a list, another object's once one is put in or taken
    /// out before it.
    found_at: std::collections::HashMap<ResRef, u64>,
}

impl Default for FindInstance {
    fn default() -> FindInstance {
        FindInstance {
            kinds: [true; 9],
            area: None,
            template: String::new(),
            tag: String::new(),
            results: Vec::new(),
            found_at: Default::default(),
        }
    }
}

impl FindInstance {
    /// Searches the module, and keeps what it found.
    pub(crate) fn find(&mut self, app: &mut Moonglow) {
        self.results = search(app, self);
        let Some(ws) = app.ws.as_ref() else { return };
        self.found_at = (self.results.iter())
            .map(|r| (r.area, ws.rewrites(&ResKey::new(r.area, ResType::GIT))))
            .collect();
    }

    /// Whether objects were put into or taken out of `area` since the
    /// results were found: its results may be other objects' places.
    fn stale(&self, ws: &mg_edit::Workspace, area: ResRef) -> bool {
        self.found_at.get(&area) != Some(&ws.rewrites(&ResKey::new(area, ResType::GIT)))
    }
}

/// A placed object found.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub kind: ObjectKind,
    pub index: usize,
    pub area: ResRef,
    pub tag: String,
    pub template: String,
}

/// The objects placed in the module's areas that match `f`'s criteria
/// (tag and blueprint: case-insensitive parts).
pub(crate) fn search(app: &mut Moonglow, f: &FindInstance) -> Vec<Found> {
    let Some(ws) = app.ws.as_ref() else { return Vec::new() };
    let mut areas: Vec<ResRef> = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
    areas.sort();
    areas.retain(|a| f.area.is_none_or(|only| only == *a));
    // (Every word, case and accents aside, as a Filter finds a name.)
    let (tag, template) =
        (mg_core::text::Needle::new(&f.tag), mg_core::text::Needle::new(&f.template));
    let mut out = Vec::new();
    for area in areas {
        // (An area not open is read from the module's copy and not kept
        // as a document: a search of a world left every area's GIT in the
        // workspace until the next save.)
        let key = ResKey::new(area, ResType::GIT);
        let read;
        let git = match ws.loaded(&key) {
            Some(doc) => &doc.root,
            None => match ws.module.get(&key).and_then(|b| mg_gff::Gff::read(b).ok()) {
                Some(gff) => {
                    read = gff;
                    &read.root
                }
                None => continue,
            },
        };
        for kind in ObjectKind::ALL {
            if !f.kinds[kind.index()] {
                continue;
            }
            let field = kind.resref_field();
            for (index, s) in git.list(kind.list()).unwrap_or(&[]).iter().enumerate() {
                let t = crate::text::decode(s.string("Tag").unwrap_or_default());
                let bp = s.resref(field).map(|r| r.to_string()).unwrap_or_default();
                if tag.finds(&t) && template.finds(&bp) {
                    out.push(Found { kind, index, area, tag: t, template: bp });
                }
            }
        }
    }
    out
}

fn find_window(app: &mut Moonglow, ui: &mut Ui) {
    let Some(mut f) = app.find_instance.take() else { return };
    let mut open = true;
    let mut go = None;
    let areas: Vec<ResRef> = app.ws.as_ref().map_or_else(Vec::new, |ws| {
        let mut a: Vec<ResRef> = ws.module.keys_of(ResType::ARE).map(|k| k.resref).collect();
        a.sort();
        a
    });
    // Areas by name, when the module tree lists them so (Options ›
    // General): each asked for as it is shown (the rows in sight, the
    // list while it is open), not every area's every frame.
    let naming = std::cell::RefCell::new((&mut *app, std::collections::HashMap::new()));
    let label = |a: &ResRef| -> String {
        let (app, labels) = &mut *naming.borrow_mut();
        labels.entry(*a).or_insert_with(|| crate::tabs::area_label(app, *a)).clone()
    };
    let mut searching = false;
    crate::widgets::dialog(ui.ctx(), "Find Instance", &mut open).default_width(460.0).show(
        ui.ctx(),
        |ui| {
            ui.horizontal_wrapped(|ui| {
                crate::widgets::field_label(ui, "Search For");
                for kind in ObjectKind::ALL {
                    ui.check_box(&mut f.kinds[kind.index()], kind.label());
                }
            });
            egui::Grid::new("find").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                crate::widgets::field_label(ui, "In Area");
                let shown = f.area.map_or_else(|| "(all areas)".to_string(), |a| label(&a));
                let options: Vec<Option<ResRef>> =
                    std::iter::once(None).chain(areas.iter().copied().map(Some)).collect();
                crate::widgets::dropdown("find-area").selected_text(shown).show(
                    ui,
                    &mut f.area,
                    &options,
                    |a| a.map_or_else(|| "(all areas)".to_string(), |a| label(&a)),
                );
                ui.end_row();
                crate::widgets::field_label(ui, "From Blueprint");
                let field =
                    ui.add(mg_uikit::text_field(&mut f.template).hint_text("blueprint resref"));
                crate::widgets::autofocus(ui, &field);
                ui.end_row();
                crate::widgets::field_label(ui, "With Tag");
                ui.add(mg_uikit::text_field(&mut f.tag).hint_text("tag"));
                ui.end_row();
            });
            ui.horizontal(|ui| {
                searching = ui.button("Search").clicked();
                if ui.button("Clear").clicked() {
                    f = FindInstance::default();
                }
            });
            ui.separator();
            ui.weak(format!("{} found; double click one to go to it", f.results.len()));
            // (The rows in sight alone are laid out, in columns of set
            // widths, a long name cut: with nothing typed every placed
            // object of the module is found, tens of thousands.)
            const WIDTHS: [f32; 4] = [76.0, 130.0, 120.0, 120.0];
            let height = ui.spacing().interact_size.y;
            let cell = |ui: &mut Ui, width: f32, add: &mut dyn FnMut(&mut Ui)| {
                let layout = egui::Layout::left_to_right(egui::Align::Center);
                ui.allocate_ui_with_layout(egui::vec2(width, height), layout, |ui| {
                    ui.set_min_width(width);
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                    add(ui);
                });
            };
            ui.horizontal(|ui| {
                for (h, width) in ["Type", "Tag", "Area", "Blueprint"].into_iter().zip(WIDTHS) {
                    cell(ui, width, &mut |ui| {
                        ui.strong(h);
                    });
                }
            });
            let rows = egui::ScrollArea::vertical().max_height(260.0).auto_shrink([false, true]);
            rows.show_rows(ui, height, f.results.len(), |ui, range| {
                for at in range {
                    let r = &f.results[at];
                    ui.horizontal(|ui| {
                        if at % 2 == 1 {
                            let mut stripe = ui.available_rect_before_wrap();
                            stripe.set_height(height);
                            ui.painter().rect_filled(stripe, 0.0, ui.visuals().faint_bg_color);
                        }
                        let row = [format!("{:?}", r.kind), r.tag.clone(), label(&r.area)];
                        let texts = row.iter().map(String::as_str).chain([r.template.as_str()]);
                        let mut double = false;
                        for (text, width) in texts.zip(WIDTHS) {
                            cell(ui, width, &mut |ui| {
                                double |= ui.selectable_label(false, text).double_clicked();
                            });
                        }
                        if double {
                            go = Some(r.clone());
                        }
                    });
                }
            });
        },
    );
    if searching {
        f.find(app);
    }
    if let Some(r) = go {
        // (Its place in the area's list may be another's by now: found
        // again, to go to what the list shows.)
        if app.ws.as_ref().is_some_and(|ws| f.stale(ws, r.area)) {
            f.find(app);
            app.log.warn(format!(
                "Find Instance: objects were added to or removed from {} since the search: \
                 searched again",
                r.area
            ));
        } else {
            app.area_focus = Some((r.area, r.kind, r.index));
            app.actions.push(Action::OpenTab(Tab::Area(r.area)));
        }
    }
    if open {
        app.find_instance = Some(f);
    }
}

/// Aurora's Preview window (`TfrmPreview`): the blueprint chosen in the
/// palette, in 3D for creatures, items, placeables and doors, with its
/// name, tag, resref and comments and a few fields of its type.
pub(crate) fn preview_window(app: &mut Moonglow, ui: &mut Ui) {
    if !app.preview_window {
        return;
    }
    let mut open = true;
    let key = app.palette.selected;
    crate::widgets::dialog(ui.ctx(), "Preview", &mut open).default_size([360.0, 460.0]).show(
        ui.ctx(),
        |ui| {
            let Some(key) = key else {
                ui.weak("Choose a blueprint in the palette.");
                return;
            };
            let gff = blueprint(app, key);
            let Some(gff) = gff else {
                ui.colored_label(ui.visuals().error_fg_color, format!("{key}: not found"));
                return;
            };
            ui.horizontal_top(|ui| {
                // Items also in 2D: the inventory icon.
                if key.restype == ResType::UTI {
                    let layers = app.item_icon(ui.ctx(), &gff.root);
                    crate::images::stacked(ui, &layers, 1.0, "Item icon");
                }
                egui::Grid::new("preview-fields").num_columns(2).spacing([12.0, 4.0]).show(
                    ui,
                    |ui| {
                        for (label, value) in summary(app, key.restype, &gff.root) {
                            crate::widgets::field_label(ui, label);
                            ui.add(egui::Label::new(value).truncate());
                            ui.end_row();
                        }
                    },
                );
            });
            if crate::model_view::previewable(key.restype) {
                ui.separator();
                ui.allocate_ui(egui::vec2(ui.available_width(), 300.0), |ui| {
                    crate::model_view::ui(app, ui, crate::model_view::Source::Resource(key));
                });
            }
            if key.restype == ResType::UTM {
                egui::ScrollArea::vertical().show(ui, |ui| store_stock(app, ui, &gff.root));
            }
        },
    );
    app.preview_window = open;
}

/// A blueprint without a model (a store, a sound, a trigger, a waypoint,
/// an encounter) where the model viewer would show one: its fields, as the
/// Preview window lists them, and a store's stock by page.
pub(crate) fn summary_view(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let Some(gff) = blueprint(app, key) else {
        ui.colored_label(ui.visuals().error_fg_color, format!("{key}: not found"));
        return;
    };
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Grid::new(("summary", key)).num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            for (label, value) in summary(app, key.restype, &gff.root) {
                crate::widgets::field_label(ui, label);
                ui.label(value);
                ui.end_row();
            }
        });
        if key.restype == ResType::UTM {
            store_stock(app, ui, &gff.root);
        }
    });
}

/// A store's items, by page: icon, name, and whether it never runs out.
fn store_stock(app: &mut Moonglow, ui: &mut Ui, store: &mg_gff::Struct) {
    let pages = store.list("StoreList").unwrap_or(&[]);
    if pages.iter().all(|p| p.list("ItemList").is_none_or(<[_]>::is_empty)) {
        ui.add_space(8.0);
        ui.weak("It sells nothing.");
        return;
    }
    for (id, page) in crate::blueprint::store::STORE_PAGES {
        let items = pages.iter().find(|p| p.id == id).and_then(|p| p.list("ItemList"));
        let Some(items) = items.filter(|l| !l.is_empty()) else { continue };
        ui.add_space(8.0);
        ui.strong(format!("{page} ({})", items.len()));
        // (Only the rows in sight read their item: a store of hundreds
        // read every blueprint each frame. The others take a row's room.)
        let row = crate::images::ICON_MAX.y + ui.spacing().item_spacing.y;
        for entry in items {
            let room = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(1.0, row));
            if !ui.is_rect_visible(room) {
                ui.add_space(row);
                continue;
            }
            let resref = entry.resref("InventoryRes").unwrap_or(ResRef::EMPTY);
            // A whole item (placed stores hold them) or its blueprint.
            let item = match entry.integer("BaseItem") {
                Some(_) => Some(entry.clone()),
                None => blueprint(app, ResKey::new(resref, ResType::UTI)).map(|g| g.root),
            };
            let icon = item.as_ref().map(|i| app.item_icon(ui.ctx(), i)).unwrap_or_default();
            let name = item
                .as_ref()
                .and_then(|i| i.locstring("LocalizedName"))
                .and_then(|ls| app.game.as_deref()?.locstring(ls))
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| resref.to_string());
            ui.horizontal(|ui| {
                crate::images::icon_box(ui, &icon, crate::images::ICON_MAX, &name);
                ui.label(&name).on_hover_text(resref.to_string());
                if entry.integer("Infinite").unwrap_or(0) != 0 {
                    ui.weak("infinite");
                }
            });
        }
    }
}

/// A blueprint from the module, else the game.
pub(crate) fn blueprint(app: &Moonglow, key: ResKey) -> Option<mg_gff::Gff> {
    let data = app
        .ws
        .as_ref()
        .and_then(|w| w.module.get(&key).map(<[u8]>::to_vec))
        .or_else(|| app.game.as_deref()?.resman.get(&key).ok().map(|d| d.into_owned()))?;
    mg_gff::Gff::read(&data).ok()
}

/// The Preview window's fields for a blueprint (as Aurora's).
fn summary(app: &Moonglow, t: ResType, s: &mg_gff::Struct) -> Vec<(&'static str, String)> {
    let game = app.game.as_deref();
    let text = |label: &str| s.string(label).map(crate::text::decode);
    let int = |label: &str| s.integer(label).map(|v| v.to_string()).unwrap_or_default();
    let yes =
        |label: &str| if s.integer(label).unwrap_or(0) != 0 { "yes" } else { "no" }.to_string();
    let resref_field = ObjectKind::resref_field_of(t);
    // (A creature by its first and last names, as the module tree and
    // `mg` name it.)
    let mut out = vec![
        ("Name", crate::tree::blueprint_name(game, s)),
        ("Tag", text("Tag").unwrap_or_default()),
        ("Blueprint ResRef", s.resref(resref_field).map(|r| r.to_string()).unwrap_or_default()),
    ];
    match t {
        ResType::UTC => {
            out.push((
                "Challenge Rating",
                s.float("ChallengeRating").map(|v| v.to_string()).unwrap_or_default(),
            ));
            out.push(("Faction", int("FactionID")));
        }
        ResType::UTD => {
            out.push(("Trap Type", int("TrapType")));
            out.push(("Faction", int("Faction")));
            out.push(("Destination Tag", text("LinkedTo").unwrap_or_default()));
            out.push(("Locked", yes("Locked")));
        }
        ResType::UTE => {
            out.push(("Difficulty", int("DifficultyIndex")));
            out.push(("Spawn Option", int("SpawnOption")));
            out.push(("Faction", int("Faction")));
        }
        ResType::UTI => {
            let cost = game.map(|g| g.item_cost(&mg_rules::ItemValue::from_gff(s)));
            out.push(("Total Cost", cost.map(|c| c.to_string()).unwrap_or_default()));
        }
        ResType::UTP => {
            out.push(("Faction", int("Faction")));
            out.push(("Trap Type", int("TrapType")));
            out.push(("Locked", yes("Locked")));
        }
        ResType::UTS => {
            out.push(("Volume", int("Volume")));
            out.push(("Active", yes("Active")));
        }
        ResType::UTT => {
            out.push(("Trigger Type", int("Type")));
            out.push(("Destination Tag", text("LinkedTo").unwrap_or_default()));
            out.push(("Faction", int("Faction")));
            out.push(("Trap Type", int("TrapType")));
        }
        _ => {}
    }
    out.push(("Comments", text("Comment").unwrap_or_default()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two placeables, the second turned a quarter, and the window as it
    /// opens on them.
    fn two() -> (Vec<mg_area::AreaObject>, AdjustLocation) {
        let placed = |x: f32, turned: f32| {
            let mut s = mg_gff::Struct::new(0);
            for (label, v) in [("X", x), ("Y", 20.0), ("Z", 0.0), ("Bearing", turned)] {
                s.set(label, mg_gff::Value::Float(v));
            }
            s
        };
        let objects: Vec<mg_area::AreaObject> = [(10.0, 0.0), (14.0, 1.5)]
            .into_iter()
            .enumerate()
            .map(|(i, (x, b))| mg_area::AreaObject::placed(ObjectKind::Placeable, i, &placed(x, b)))
            .collect();
        let mut a = adjust_start(ResRef::from_str("area").unwrap(), (Vec3::ZERO, 0.0));
        a.start = false;
        a.objects = vec![(ObjectKind::Placeable, 0), (ObjectKind::Placeable, 1)];
        a.position = objects[0].position;
        (objects, a)
    }

    /// Only what was changed is shown changed, on each object, as Apply
    /// sets it: Y alone moves both north and leaves their X and turns.
    #[test]
    fn the_view_shows_what_the_window_changes_on_each_object() {
        let (objects, mut a) = two();
        assert_eq!(previewed(&a, &objects), (Vec::new(), Vec::new()), "nothing changed yet");
        a.position.y = 25.0;
        a.changed[1] = true;
        let (moved, visual) = previewed(&a, &objects);
        assert!(visual.is_empty());
        let want: Vec<(usize, Vec3, f32)> = objects
            .iter()
            .enumerate()
            .map(|(i, o)| (i, Vec3::new(o.position.x, 25.0, 0.0), o.rotation))
            .collect();
        assert_eq!(moved, want);
        // The bearing too: each turned to it.
        a.bearing = 90.0;
        a.changed[3] = true;
        let (moved, _) = previewed(&a, &objects);
        assert!(moved.iter().all(|m| (m.2 - 90f32.to_radians()).abs() < 1e-6), "{moved:?}");
        // A visual transform, given to each that takes one.
        a.visual.scale = Vec3::splat(2.0);
        a.visual_changed = true;
        let (_, visual) = previewed(&a, &objects);
        assert_eq!(visual.iter().map(|v| v.0).collect::<Vec<_>>(), [0, 1]);
        // An object no longer there (deleted while the window was open).
        a.objects.push((ObjectKind::Placeable, 7));
        assert_eq!(previewed(&a, &objects).0.len(), 2);
        // The start location: none of the objects'.
        a.start = true;
        assert_eq!(previewed(&a, &objects), (Vec::new(), Vec::new()));
    }

    /// The start location shows where the window puts it: its changed
    /// parts alone, its bearing as the way its arrow faces.
    #[test]
    fn the_start_location_shows_where_the_window_puts_it() {
        let area = ResRef::from_str("area").unwrap();
        let at = (Vec3::new(5.0, 6.0, 0.0), std::f32::consts::FRAC_PI_2);
        let mut a = adjust_start(area, at);
        assert_eq!(previewed_start(&a, at), at, "nothing changed");
        a.position.x = 9.0;
        a.changed[0] = true;
        a.bearing = 90.0;
        a.changed[3] = true;
        let (to, facing) = previewed_start(&a, at);
        assert_eq!(to, Vec3::new(9.0, 6.0, 0.0));
        assert!((facing - std::f32::consts::PI).abs() < 1e-6, "west: {facing}");
    }

    /// The review's A26: what a window lists from disk is read as it
    /// opens and kept while it is shown, not read every frame; a window
    /// closed and opened again reads anew.
    #[test]
    fn what_a_window_reads_from_disk_is_kept_while_it_is_shown() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("kept");
        let made = std::cell::Cell::new(0);
        let shown = |show: bool| {
            let out = ctx.run_ui(Default::default(), |ui| {
                if show {
                    let v = kept_while_shown(ui.ctx(), id, || {
                        made.set(made.get() + 1);
                        made.get()
                    });
                    assert_eq!(v, made.get());
                }
            });
            { out }.textures_delta.clear();
        };
        for _ in 0..5 {
            shown(true);
        }
        assert_eq!(made.get(), 1, "read once while shown");
        shown(false);
        shown(false);
        shown(true);
        shown(true);
        assert_eq!(made.get(), 2, "and again when opened again");
    }
}
