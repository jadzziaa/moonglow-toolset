//! The area viewer's dialogs: Adjust Location (`TdlgLocation`: an exact
//! position, bearing and visual transform for the selected objects); and
//! Find Instance's search (`TdlgFindInstance`: placed objects across the
//! module by type, area, blueprint and tag; a double click goes to one),
//! whose form and rows are Search Results' since M4's U28.

use egui::Ui;
use glam::Vec3;
use mg_area::ObjectKind;
use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_resman::ResKey;

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

/// Which parts of an object's [`mg_area::VisualTransform`] were changed in
/// Adjust Location: only those parts are applied across a multi-object
/// selection so editing Scale does not overwrite another object's rotation
/// or translation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VisualChanged {
    pub scale: bool,
    pub rotate: [bool; 3],
    pub translate: [bool; 3],
}

impl VisualChanged {
    /// Whether any visual transform field was edited.
    pub fn any(self) -> bool {
        self.scale || self.rotate.iter().any(|&c| c) || self.translate.iter().any(|&c| c)
    }

    /// Marks every component as changed (for callers/tests setting a full transform).
    pub fn all() -> Self {
        Self { scale: true, rotate: [true; 3], translate: [true; 3] }
    }
}

/// Applies only the components marked in `changed` from `edited` onto `base`.
fn merge_visual(
    mut base: mg_area::VisualTransform,
    edited: mg_area::VisualTransform,
    changed: VisualChanged,
) -> mg_area::VisualTransform {
    if changed.scale {
        base.scale = edited.scale;
    }
    for i in 0..3 {
        if changed.rotate[i] {
            base.rotate[i] = edited.rotate[i];
        }
        if changed.translate[i] {
            base.translate[i] = edited.translate[i];
        }
    }
    base
}

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
    /// The visual transform (EE), and which of its fields were changed.
    pub visual: mg_area::VisualTransform,
    pub visual_changed: VisualChanged,
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

impl AdjustLocation {
    /// Whether any field has unapplied edits.
    pub fn has_changes(&self) -> bool {
        self.changed.iter().any(|&c| c) || self.visual_changed.any() || self.speed_changed
    }

    /// Marks all fields as clean after saving.
    pub(crate) fn clear_changes(&mut self) {
        self.changed = [false; 4];
        self.visual_changed = VisualChanged::default();
        self.speed_changed = false;
    }

    /// Whether no object and no start location is currently targeted.
    pub(crate) fn is_empty(&self) -> bool {
        !self.start && self.objects.is_empty()
    }
}

/// An empty Adjust Location state kept when the dialog stays open after
/// deselecting in the area view.
fn empty_adjust(area: ResRef) -> AdjustLocation {
    AdjustLocation {
        area,
        objects: Vec::new(),
        position: Vec3::ZERO,
        bearing: 0.0,
        changed: [false; 4],
        visual: mg_area::VisualTransform::default(),
        visual_changed: VisualChanged::default(),
        speed: 1.0,
        speed_changed: false,
        kept: Vec::new(),
        shaped: false,
        fixed: 0,
        fixed_shaped: 0,
        uneven: 0,
        start: false,
    }
}

/// Refreshes clean Adjust Location values from the current area state so
/// dragging or turning the object in the view updates the open dialog live.
fn sync_live(app: &mut Moonglow, a: &mut AdjustLocation) {
    if a.has_changes() || a.is_empty() {
        return;
    }
    if a.start {
        if let Some(at) = crate::area_view::start_location(app, a.area) {
            let refreshed = adjust_start(a.area, at);
            a.position = refreshed.position;
            a.bearing = refreshed.bearing;
        }
    } else if let Some(view) = app.area_views.get(&a.area)
        && view.selection == a.objects
        && let Some(refreshed) = adjust(view)
    {
        a.position = refreshed.position;
        a.bearing = refreshed.bearing;
        a.visual = refreshed.visual;
        a.speed = refreshed.speed;
        a.kept = refreshed.kept;
        a.shaped = refreshed.shaped;
        a.fixed = refreshed.fixed;
        a.fixed_shaped = refreshed.fixed_shaped;
        a.uneven = refreshed.uneven;
    }
}

#[derive(Clone)]
struct AdjustWinState {
    rect: egui::Rect,
    seen_selection: (bool, Vec<(ObjectKind, usize)>),
    /// The revision of the area view's model its numbers were last
    /// refreshed from ([`sync_live`]): not again until the model changes
    /// (a drag shows in the view's posed copy, not in its model, until it
    /// is let go and written).
    synced: Option<u64>,
}

impl Default for AdjustWinState {
    fn default() -> Self {
        Self { rect: egui::Rect::NOTHING, seen_selection: (false, Vec::new()), synced: None }
    }
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
        visual_changed: VisualChanged::default(),
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
pub fn adjust(view: &crate::area_view::AreaView) -> Option<AdjustLocation> {
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
        visual_changed: VisualChanged::default(),
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
}

fn adjust_window(app: &mut Moonglow, ui: &mut Ui) {
    let state_id = egui::Id::new("mg-adjust-win-state");
    let Some(mut a) = app.adjust.take() else {
        ui.ctx().data_mut(|d| d.remove_temp::<AdjustWinState>(state_id));
        return;
    };
    let prev_state: Option<AdjustWinState> = ui.ctx().data(|d| d.get_temp(state_id));
    // (An area another builder holds, in a project that keeps others'
    // areas read-only: its writes are refused, so nothing is applied or
    // saved on the way, and the edit stays in the dialog until Cancel.)
    let read_only = crate::claims::read_only(app, a.area);
    let mut open = true;
    let footed = crate::widgets::dialog(ui.ctx(), "Adjust Location", &mut open)
        .resizable(false)
        .collapsible(false)
        .show_foot(ui.ctx(), |ui| {
            let empty = a.is_empty();
            if empty {
                ui.weak("Select an object to adjust its location.");
            } else {
                if a.objects.len() > 1 {
                    ui.weak(format!("{} objects: what you change is set on each", a.objects.len()));
                }
                if a.start {
                    ui.weak("The module's start location");
                }
            }
            ui.add_enabled_ui(!empty, |ui| {
                egui::Grid::new("adjust").num_columns(4).spacing([12.0, 6.0]).show(ui, |ui| {
                    // (Two fields to a row: X and Y, then Z and the bearing.)
                    for row in [[0, 1], [2, 3]] {
                        for i in row {
                            if i < 3 {
                                crate::widgets::field_label(ui, ["X", "Y", "Z"][i]);
                                // (A step of an arrow: a tenth of a meter.)
                                let r = crate::widgets::spin_draft(
                                    ui,
                                    &mut a.position[i],
                                    FAR,
                                    0.1,
                                    |d| d.speed(0.05).max_decimals(3),
                                );
                                a.changed[i] |= r.changed();
                            } else {
                                crate::widgets::field_label(ui, "Bearing (°)")
                                    .on_hover_text("0 faces north, 90 west (as Aurora shows it)");
                                let r = crate::widgets::spin_draft(
                                    ui,
                                    &mut a.bearing,
                                    0.0..=360.0,
                                    1.0,
                                    |d| d.speed(1.0).max_decimals(2),
                                );
                                a.changed[3] |= r.changed();
                            }
                        }
                        ui.end_row();
                    }
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
                        let r =
                            crate::widgets::spin_draft(ui, &mut scale, 0.01..=100.0, 0.1, |d| {
                                d.speed(0.01).max_decimals(2)
                            });
                        if r.changed() {
                            a.visual.scale = Vec3::splat(scale);
                            a.visual_changed.scale = true;
                            // (Written alike from now on.)
                            a.uneven = 0;
                        }
                        crate::widgets::field_label(ui, "Animation speed").on_hover_text(
                            "How fast the model's animations play: 1 as made, 2 twice as fast",
                        );
                        let r =
                            crate::widgets::spin_draft(ui, &mut a.speed, 0.0..=100.0, 0.1, |d| {
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
                            crate::widgets::field_label(ui, format!("{axis} rotation"))
                                .on_hover_text(turns[i]);
                            let turn = &mut a.visual.rotate[i];
                            let r = crate::widgets::spin_draft(ui, turn, FAR, 1.0, |d| {
                                d.speed(1.0).max_decimals(1)
                            });
                            a.visual_changed.rotate[i] |= r.changed();
                            crate::widgets::field_label(ui, format!("{axis} translation"));
                            let to = &mut a.visual.translate[i];
                            let r = crate::widgets::spin_draft(ui, to, FAR, 0.1, |d| {
                                d.speed(0.01).max_decimals(2)
                            });
                            a.visual_changed.translate[i] |= r.changed();
                            ui.end_row();
                        }
                    });
                }
                if a.shaped && !a.kept.is_empty() {
                    ui.add_space(crate::widgets::SECTION_GAP);
                    ui.scope(|ui| {
                        crate::widgets::fit_width(ui, 340.0);
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
                        crate::widgets::fit_width(ui, 340.0);
                        ui.add(egui::Label::new(egui::RichText::new(note).weak()).wrap());
                    });
                }
                if let Some(note) = static_note(&a) {
                    ui.add_space(crate::widgets::SECTION_GAP);
                    ui.colored_label(ui.visuals().warn_fg_color, note);
                }
                if let Some(why) = &read_only {
                    ui.add_space(crate::widgets::SECTION_GAP);
                    let note = format!("{why}: read-only here, so what you change is not saved.");
                    ui.scope(|ui| {
                        crate::widgets::fit_width(ui, 340.0);
                        let text = egui::RichText::new(note).color(ui.visuals().warn_fg_color);
                        ui.add(egui::Label::new(text).wrap());
                    });
                }
            });
            let can = !empty && read_only.is_none();
            let foot = crate::widgets::Foot::new()
                .main("OK", can)
                .tip("Apply changes and close (clicking outside also applies)")
                .button("Apply", can)
                .tip("Apply changes and keep open")
                .cancel()
                .tip("Discard unapplied changes (Esc)");
            ((), foot)
        });
    let (cancel, done) = match &footed {
        Some(f) if f.pressed.main() => (false, Some(true)),
        Some(f) if f.pressed.clicked("Apply") => (false, Some(false)),
        Some(f) => (f.pressed.cancelled(), None),
        None => (false, None),
    };
    if cancel || !open {
        ui.ctx().data_mut(|d| d.remove_temp::<AdjustWinState>(state_id));
        return;
    }
    if let Some(close) = done {
        if !a.is_empty() && read_only.is_none() {
            apply_location(app, &a);
            a.clear_changes();
        }
        if close {
            ui.ctx().data_mut(|d| d.remove_temp::<AdjustWinState>(state_id));
            return;
        }
    }
    let cur_selection =
        app.area_views.get(&a.area).map(|v| (v.start_is_selected(), v.selection.clone()));
    let rect = footed.as_ref().map_or(egui::Rect::NOTHING, |f| f.response.rect);
    let model_now = app.area_views.get(&a.area).and_then(|v| v.revision);
    let mut synced = None;
    if let Some(prev) = prev_state {
        let selection_changed = cur_selection.as_ref().is_some_and(|cur| {
            *cur != prev.seen_selection && (cur.0 != a.start || cur.1 != a.objects)
        });
        // (Not saved where the write would be refused: kept, as its note
        // says.)
        let held = read_only.is_some() && a.has_changes() && !a.is_empty();
        let pressed_outside = a.has_changes()
            && !a.is_empty()
            && !held
            && !egui::Popup::is_any_open(ui.ctx())
            && ui.ctx().input(|i| {
                i.pointer.primary_pressed()
                    && i.pointer.press_origin().is_some_and(|p| !prev.rect.contains(p))
            });
        if selection_changed && held {
            // (The dialog stays on what it was editing.)
        } else if selection_changed {
            if a.has_changes() && !a.is_empty() {
                apply_location(app, &a);
            }
            let area = a.area;
            a = match cur_selection.as_ref() {
                Some((true, _)) => crate::area_view::start_location(app, area)
                    .map_or_else(|| empty_adjust(area), |at| adjust_start(area, at)),
                Some((false, sel)) if !sel.is_empty() => {
                    app.area_views.get(&area).and_then(adjust).unwrap_or_else(|| empty_adjust(area))
                }
                _ => empty_adjust(area),
            };
            ui.ctx().request_repaint();
        } else if pressed_outside {
            apply_location(app, &a);
            a.clear_changes();
        } else if !a.has_changes() && !a.is_empty() {
            if a.start || model_now.is_none() || model_now != prev.synced {
                sync_live(app, &mut a);
            }
            synced = model_now;
        }
    }
    let seen_selection = cur_selection.unwrap_or_else(|| (a.start, a.objects.clone()));
    ui.ctx().data_mut(|d| d.insert_temp(state_id, AdjustWinState { rect, seen_selection, synced }));
    app.adjust = Some(a);
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
         offered. Uncheck Static (Properties) to scale, tilt or move a model.{kept}"
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
        if a.visual_changed.any() && o.takes_visual_transform() {
            let base = o.visual.unwrap_or_default();
            visual.push((i, merge_visual(base, a.visual, a.visual_changed)));
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
    // (A trigger moved in X or Y goes back on the ground.)
    let on = |p: glam::Vec2, near: f32| view.ground().and_then(|g| g.height(p, near));
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
        edits.extend(mg_area::edit::move_edits_on(git, o, s, p, rotation, Some(&on)));
        if (a.visual_changed.any() || a.speed_changed) && o.takes_visual_transform() {
            // (What wasn't changed here, each its own: a speed alone
            // changed leaves the object's transform as it was, and editing
            // one visual field leaves the others intact on each object.)
            let base = o.visual.unwrap_or_default();
            let v = if a.visual_changed.any() {
                merge_visual(base, a.visual, a.visual_changed)
            } else {
                base
            };
            let speed = a.speed_changed.then_some(a.speed);
            edits.extend(mg_area::edit::visual_transform_edits_with(git, o, s, v, speed));
        }
    }
    if !edits.is_empty() {
        app.actions.push(Action::Apply(Command::new("Adjust Location", edits)));
    }
}

/// Find Instance's criteria and results (M4's U28: Search Results' form
/// and rows, `search_results`; before it a window of its own).
#[derive(Debug, Clone, PartialEq)]
pub struct FindInstance {
    /// Which kinds to look for, by [`ObjectKind::index`].
    pub kinds: [bool; 9],
    /// `None`: every area.
    pub area: Option<ResRef>,
    pub template: String,
    pub tag: String,
    pub results: Vec<Found>,
    /// Whether a search was made (its results, none among them, are
    /// shown; before, what fills the panel is said).
    pub searched: bool,
    /// How many times each area's GIT had its lists rewritten as the search
    /// began (`Workspace::rewrites`): a result is a place in a list,
    /// another object's once one is put in or taken out before it.
    found_at: std::collections::HashMap<ResRef, u64>,
    /// Counted up as the results change (Search Results' order is made
    /// again only then).
    pub(crate) revision: u64,
}

impl Default for FindInstance {
    fn default() -> FindInstance {
        FindInstance {
            kinds: [true; 9],
            area: None,
            template: String::new(),
            tag: String::new(),
            results: Vec::new(),
            searched: false,
            found_at: Default::default(),
            revision: 0,
        }
    }
}

impl FindInstance {
    /// Whether objects were put into or taken out of `area` since the
    /// search began: its results may be other objects' places.
    pub(crate) fn stale(&self, ws: &mg_edit::Workspace, area: ResRef) -> bool {
        self.found_at.get(&area) != Some(&ws.rewrites(&ResKey::new(area, ResType::GIT)))
    }

    /// A search's results, in place of the last one's; `at`, each area's
    /// lists' rewrites as it began.
    pub(crate) fn set_results(
        &mut self,
        results: Vec<Found>,
        at: std::collections::HashMap<ResRef, u64>,
    ) {
        self.results = results;
        self.found_at = at;
        self.searched = true;
        self.revision += 1;
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

/// What a search looks for, sent to its job.
#[derive(Debug, Clone)]
struct Criteria {
    kinds: [bool; 9],
    area: Option<ResRef>,
    template: String,
    tag: String,
}

/// The objects placed in `module`'s areas that match `f`'s criteria
/// (tag and blueprint: case-insensitive parts), an area at a time,
/// `step` told how far it is (and whether to go on: none found where it
/// says no).
fn search_in(
    module: &mg_module::Module,
    f: &Criteria,
    step: &mut dyn FnMut(usize, usize) -> bool,
) -> Option<Vec<Found>> {
    let mut areas: Vec<ResRef> = module.keys_of(ResType::ARE).map(|k| k.resref).collect();
    areas.sort();
    areas.retain(|a| f.area.is_none_or(|only| only == *a));
    // (Every word, case and accents aside, as a Filter finds a name.)
    let (tag, template) =
        (mg_core::text::Needle::new(&f.tag), mg_core::text::Needle::new(&f.template));
    let mut out = Vec::new();
    let total = areas.len();
    for (n, area) in areas.into_iter().enumerate() {
        if !step(n, total) {
            return None;
        }
        // (The module's copy, read and let go: a search of a world kept
        // no area's GIT as a document.)
        let key = ResKey::new(area, ResType::GIT);
        let Some(gff) = module.get(&key).and_then(|b| mg_gff::Gff::read(b).ok()) else {
            continue;
        };
        for kind in ObjectKind::ALL {
            if !f.kinds[kind.index()] {
                continue;
            }
            let field = kind.resref_field();
            for (index, s) in gff.root.list(kind.list()).unwrap_or(&[]).iter().enumerate() {
                let t = crate::text::decode(s.string("Tag").unwrap_or_default());
                let bp = s.resref(field).map(|r| r.to_string()).unwrap_or_default();
                if tag.finds(&t) && template.finds(&bp) {
                    out.push(Found { kind, index, area, tag: t, template: bp });
                }
            }
        }
    }
    step(total, total);
    Some(out)
}

/// Find Instance's search, as a job (Search Results' Search, Enter in its
/// fields): the module as it is, an area at a time, its progress in the
/// job's window and Cancel there; what it finds into Search Results, in
/// place of what was found before. A search while a job runs waits (the
/// log says so), as every job does.
pub(crate) fn start_find(app: &mut Moonglow) {
    let Some(f) = app.find_instance.as_ref() else { return };
    let Some(ws) = app.ws.as_ref() else { return };
    let criteria =
        Criteria { kinds: f.kinds, area: f.area, template: f.template.clone(), tag: f.tag.clone() };
    // (Each area's lists as the search begins: what a result's place is
    // of.)
    let at: std::collections::HashMap<ResRef, u64> = (ws.module.keys_of(ResType::ARE))
        .map(|k| (k.resref, ws.rewrites(&ResKey::new(k.resref, ResType::GIT))))
        .collect();
    app.start_job(
        "Find Instance",
        move |job| {
            search_in(&job.module, &criteria, &mut |done, total| {
                job.progress.step(done, total);
                !job.progress.cancelled()
            })
        },
        move |app, found| {
            let Some(found) = found else { return };
            let Some(f) = app.find_instance.as_mut() else { return };
            f.set_results(found, at);
            app.show_search(crate::search_results::Showing::Instances);
        },
    );
}

/// Goes to a placed object found (a double click on its row, Enter): its
/// area opened at it; where objects were put in or taken out of its area
/// since, its place may be another's by now, so the module is searched
/// again and the log says so.
pub(crate) fn go_to_found(app: &mut Moonglow, r: &Found) {
    let stale = match (&app.ws, &app.find_instance) {
        (Some(ws), Some(f)) => f.stale(ws, r.area),
        _ => false,
    };
    if stale {
        app.log.warn(format!(
            "Find Instance: objects were added to or removed from {} since the search: \
             searched again",
            r.area
        ));
        start_find(app);
    } else {
        app.area_focus = Some((r.area, r.kind, r.index));
        app.actions.push(Action::OpenTab(Tab::Area(r.area)));
    }
}

/// The Right Dock's Preview panel's body drawn late, after the editor
/// area, in the room `shell::panel_body` kept for it this frame (its
/// late slot, `shell::draw_late`): where an editor's tab may show the same
/// model, so that the tab keeps its view (as it did beside the Preview
/// window: a model is drawn in one place a frame, `model_view`), this one
/// saying so. Its widgets are the Right Dock's (under its pane), cut at
/// the panel's edges. (Else the body is drawn in the dock in its place:
/// [`preview_body`], so that Tab goes from the dock's tab bar into it.)
pub(crate) fn preview_late(app: &mut Moonglow, ui: &mut Ui, slot: crate::shell::LateSlot) {
    let builder = egui::UiBuilder::new()
        .max_rect(slot.rect)
        .layer_id(slot.layer)
        .accessibility_parent(slot.pane);
    let builder = crate::shell::LateSlot::salted(builder, slot.host, "preview-panel", slot.pane);
    let mut body = ui.new_child(builder);
    body.set_clip_rect(slot.rect);
    preview_body(app, &mut body, slot.rect);
}

/// How far an arrow key turns the Preview's model, about it and up or
/// down (radians: 15° and 10°).
const PREVIEW_TURN: (f32, f32) = (0.2618, 0.1745);

/// With the arrow keys the Preview's own (F6 to it), they turn `source`'s
/// model as a drag does: Left and Right about it, Up and Down over or
/// under it.
fn turn_by_keys(app: &mut Moonglow, ui: &Ui, source: &crate::model_view::Source) {
    use crate::palette_view::{Keys, keys_are};
    if keys_are(ui.ctx(), Keys::Preview)
        && !ui.ctx().egui_wants_keyboard_input()
        && let Some(view) = app.model_views.get_mut(source)
    {
        let (yaw, pitch) = ui.input(|i| {
            let k = |key| f32::from(u8::from(i.key_pressed(key)));
            use egui::Key::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp};
            (k(ArrowRight) - k(ArrowLeft), k(ArrowUp) - k(ArrowDown))
        });
        let o = &mut view.orbit;
        o.yaw -= yaw * PREVIEW_TURN.0;
        o.pitch = (o.pitch + pitch * PREVIEW_TURN.1).clamp(-1.45, 1.45);
    }
}

/// What a blueprint's or placed object's own Preview says where its model
/// has a tab or window of its own (Pop Out).
pub const POPPED_OUT: &str = "The preview is in a window of its own.";

/// A blueprint's or placed object's own Preview panel's body (M4's U11,
/// the Creature board's Right Dock: what the Appearance page and a
/// placeable's or a door's Basic page showed beside their fields), in
/// `ui`, `rect`: its model's header row over its 3D view, flush with the
/// panel's edges, the info line in the panel's tab bar where it is
/// `titled` (else over the header, as the last frame drew the model). Pop Out, in the header's More, shows
/// the model in a tab or window of its own; while it is there, the panel
/// says so and offers to bring it back. The arrow keys turn the model as
/// the palette's Preview's do.
pub(crate) fn own_preview_body(
    app: &mut Moonglow,
    ui: &mut Ui,
    rect: egui::Rect,
    source: crate::model_view::Source,
    titled: bool,
) {
    use crate::palette_view::{Keys, keys_outline};
    keys_outline(ui, rect, Keys::Preview);
    let tab = source.tab();
    let popped = app.open_models.contains(&tab) || app.dock.find_tab(&tab).is_some();
    if popped {
        egui::Frame::new()
            .inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 4 })
            .show(ui, |ui| {
                ui.weak(POPPED_OUT);
                let bring =
                    ui.button("Bring Back").on_hover_text("Close its window and show it here");
                if bring.clicked() {
                    app.actions.push(crate::Action::CloseTab(tab));
                }
            });
        return;
    }
    turn_by_keys(app, ui, &source);
    // (Untitled where it shares a tab bar under 1400: its info line over
    // its header, in the panel's margin.)
    if !titled && let Some(info) = crate::model_view::info(app, &source) {
        let margin = egui::Margin { left: 8, right: 8, top: 4, bottom: 2 };
        egui::Frame::new().inner_margin(margin).show(ui, |ui| {
            ui.add(egui::Label::new(egui::RichText::new(info).weak()).truncate());
        });
    }
    let shown =
        crate::model_view::Shown { info: false, pop_out: true, restore: false, paused: false };
    if crate::model_view::panel(app, ui, source, shown) {
        app.actions.push(crate::Action::OpenTab(tab));
    }
}

/// The Right Dock's Preview panel's body (Aurora's Preview window,
/// `TfrmPreview`, and Moonglow's until M3's T6), in `ui`, `rect`: the
/// blueprint chosen in the palette, its name, tag, resref and comments and
/// a few fields of its type (an item's inventory icon beside them), in 3D
/// under them for creatures, items, placeables and doors, a store's stock
/// by page. With the arrow keys its own (F6 to the Right Dock with the
/// Preview's tab in front), they turn its model as a drag does: Left and
/// Right about it, Up and Down over or under it; a thin outline says so.
pub(crate) fn preview_body(app: &mut Moonglow, ui: &mut Ui, rect: egui::Rect) {
    use crate::palette_view::{Keys, keys_outline};
    keys_outline(ui, rect, Keys::Preview);
    let Some(key) = app.palette.selected else {
        ui.weak("Choose a blueprint in the palette.");
        return;
    };
    turn_by_keys(app, ui, &crate::model_view::Source::Resource(key));
    let Some(gff) = blueprint(app, key) else {
        ui.colored_label(ui.visuals().error_fg_color, format!("{key}: not found"));
        return;
    };
    let model = crate::model_view::previewable(key.restype);
    let store = key.restype == ResType::UTM;
    // (Its fields take at most a quarter of the panel where its model or
    // its stock is under them, scrolled where they are more: the model has
    // the room.)
    let most = if model || store { (rect.height() / 4.0).max(40.0) } else { f32::INFINITY };
    // (An item's icon beside its fields where the panel is wide enough for
    // both, else over them: the fields' names and values have the width.)
    let beside = rect.width() >= 300.0;
    let fields = |app: &mut Moonglow, ui: &mut Ui| {
        if key.restype == ResType::UTI {
            let layers = app.item_icon(ui.ctx(), &gff.root);
            crate::images::stacked(ui, &layers, 1.0, "Item icon");
        }
        egui::Grid::new("preview-fields").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            for (label, value) in summary(app, key.restype, &gff.root) {
                crate::widgets::field_label(ui, label);
                ui.add(egui::Label::new(value).truncate());
                ui.end_row();
            }
        });
    };
    egui::ScrollArea::vertical()
        .id_salt("preview-fields-list")
        .max_height(most)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            if beside {
                ui.horizontal_top(|ui| fields(app, ui));
            } else {
                fields(app, ui);
            }
        });
    // (What is under the fields in the room left, never wider than the
    // panel: what is in it may not widen it.)
    let under = |ui: &mut Ui| {
        let room = ui.available_rect_before_wrap().intersect(rect);
        ui.advance_cursor_after_rect(room);
        let mut under = ui.new_child(egui::UiBuilder::new().max_rect(room));
        under.set_clip_rect(room);
        under
    };
    if model {
        ui.separator();
        let mut view = under(ui);
        crate::model_view::ui(app, &mut view, crate::model_view::Source::Resource(key));
    }
    if store {
        ui.separator();
        let mut stock = under(ui);
        egui::ScrollArea::vertical()
            .id_salt("preview-stock-list")
            .auto_shrink([false, false])
            .show(&mut stock, |ui| store_stock(app, ui, &gff.root));
    }
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

    /// Three areas, each with two placeables ("TREE_<area><n>", from
    /// `mg_tree`) and a creature ("GUARD", from `mg_guard`).
    fn three_areas() -> mg_module::Module {
        let mut m = mg_module::Module::new();
        for area in ["a1", "a2", "a3"] {
            let are = ResKey::parse(area, ResType::ARE).unwrap();
            m.set_gff(are, &mg_gff::Gff::new(*b"ARE ")).unwrap();
            let object = |tag: String, template: &str| {
                let mut s = mg_gff::Struct::new(0);
                s.set("Tag", mg_gff::Value::String(tag.into_bytes()));
                s.set("TemplateResRef", mg_gff::Value::ResRef(template.as_bytes().to_vec()));
                s
            };
            let mut git = mg_gff::Gff::new(*b"GIT ");
            let trees = (0..2).map(|n| object(format!("TREE_{area}{n}"), "mg_tree")).collect();
            git.root.set("Placeable List", mg_gff::Value::List(trees));
            let guard = vec![object("GUARD".into(), "mg_guard")];
            git.root.set("Creature List", mg_gff::Value::List(guard));
            m.set_gff(ResKey::parse(area, ResType::GIT).unwrap(), &git).unwrap();
        }
        m
    }

    fn criteria(tag: &str, template: &str) -> Criteria {
        Criteria { kinds: [true; 9], area: None, template: template.into(), tag: tag.into() }
    }

    /// M4's U28: Find Instance's search, as its job runs it, says how far
    /// it is an area at a time and ends where Cancel says so, with nothing
    /// found; let run, it finds by area, kind and place, by tag and by
    /// blueprint.
    #[test]
    fn find_instance_s_search_says_how_far_it_is_and_stops_when_called_off() {
        let m = three_areas();
        let mut steps = Vec::new();
        let found = search_in(&m, &criteria("", ""), &mut |done, total| {
            steps.push((done, total));
            true
        })
        .expect("not called off");
        assert_eq!(steps, [(0, 3), (1, 3), (2, 3), (3, 3)], "an area at a time");
        let said: Vec<String> = found
            .iter()
            .map(|f| format!("{}:{:?}:{}:{}", f.area, f.kind, f.index, f.tag))
            .collect();
        assert_eq!(
            said,
            [
                "a1:Creature:0:GUARD",
                "a1:Placeable:0:TREE_a10",
                "a1:Placeable:1:TREE_a11",
                "a2:Creature:0:GUARD",
                "a2:Placeable:0:TREE_a20",
                "a2:Placeable:1:TREE_a21",
                "a3:Creature:0:GUARD",
                "a3:Placeable:0:TREE_a30",
                "a3:Placeable:1:TREE_a31",
            ]
        );
        let by_tag = search_in(&m, &criteria("tree_a2", ""), &mut |_, _| true).unwrap();
        assert_eq!(by_tag.len(), 2, "{by_tag:?}");
        let by_blueprint = search_in(&m, &criteria("", "GUARD"), &mut |_, _| true).unwrap();
        assert_eq!(by_blueprint.len(), 3, "{by_blueprint:?}");
        let mut only = criteria("", "");
        only.area = Some(ResRef::from_str("a3").unwrap());
        only.kinds[ObjectKind::Creature.index()] = false;
        let in_one = search_in(&m, &only, &mut |_, _| true).unwrap();
        assert_eq!(
            in_one.iter().map(|f| f.tag.as_str()).collect::<Vec<_>>(),
            ["TREE_a30", "TREE_a31"]
        );
        // Called off as the second area begins: nothing, and no area after.
        let mut asked = 0;
        let off = search_in(&m, &criteria("", ""), &mut |done, _| {
            asked += 1;
            done < 1
        });
        assert_eq!(off, None, "called off");
        assert_eq!(asked, 2, "no area read after it was called off");
    }

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
        let (mut objects, mut a) = two();
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
        a.visual_changed.scale = true;
        let (_, visual) = previewed(&a, &objects);
        assert_eq!(visual.iter().map(|v| v.0).collect::<Vec<_>>(), [0, 1]);
        // Editing Scale alone preserves distinct rotations and translations on each object.
        objects[1].visual = Some(mg_area::VisualTransform {
            scale: Vec3::ONE,
            rotate: Vec3::new(15.0, 25.0, 35.0),
            translate: Vec3::new(1.0, 2.0, 3.0),
        });
        let (_, visual) = previewed(&a, &objects);
        assert_eq!(visual[1].1.scale, Vec3::splat(2.0));
        assert_eq!(visual[1].1.rotate, Vec3::new(15.0, 25.0, 35.0));
        assert_eq!(visual[1].1.translate, Vec3::new(1.0, 2.0, 3.0));
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
