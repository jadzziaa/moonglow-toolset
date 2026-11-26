//! The canvas: its toolbar, the window's chrome and the window's body.
use super::*;
use std::borrow::Cow;

pub(in crate::nui_view) fn canvas(
    ui: &mut Ui,
    v: &mut Value,
    s: &Settings,
    state: &mut State,
    assets: &mut skin::Assets,
) {
    if state.preview_scale == 0.0 {
        state.preview_scale = 1.0;
    }
    track_gestures(ui, v, state);
    let mut runtime = session(crate::nui_view::trusted(ui.ctx(), state), v, s, state);
    // What is shown in the window's place: a simulation's window, the window
    // as a gesture under way leaves it, or a variant inside its window.
    // Otherwise it is the window itself, and nothing is copied.
    let (mut other, shown_settings) = match &runtime {
        Some(r) if state.preview_interactive => (Some(Cow::Borrowed(&r.doc)), &r.settings),
        _ => (None, s),
    };
    if state.resize.is_some() || state.draw_drag.is_some() || state.window_move.is_some() {
        let mut shown = other.map_or_else(|| v.clone(), Cow::into_owned);
        if let Some(resize) = &state.resize {
            resize.apply(&mut shown);
        }
        if let Some(drag) = &state.draw_drag {
            drag.apply(&mut shown);
        }
        if let Some(m) = &state.window_move {
            m.apply(&mut shown);
        }
        other = Some(Cow::Owned(shown));
    }
    let context = layouts::canvas_context(other.as_deref().unwrap_or(v), shown_settings, state);
    let context_prefix = context.map(|(context, prefix)| {
        other = Some(Cow::Owned(context));
        prefix
    });
    let empty_rows = Default::default();
    let row_values = runtime.as_ref().map_or(&empty_rows, |r| &r.row_values);
    let closed = state.preview_interactive && runtime.as_ref().is_some_and(|r| r.closed);
    let authored = v;
    let s = shown_settings;
    let size = window_size(other.as_deref().unwrap_or(authored), s);
    // (The whole room: what the canvas says stands over its corners, M4's
    // U19, not in rows over and under it.)
    let available = ui.available_size().max(vec2(80.0, 80.0));
    if state.screen_size.x <= 0.0 {
        state.screen_size = vec2(1920.0, 1080.0);
    }
    // The client caps UI scale at its screen height over 720 unless
    // ui.unconstrain-scale is set (NWN EE 8193.37: 150% on a 993-high screen
    // draws at 1.38).
    let ui_scale = if state.screen_preview {
        state.preview_scale.min(state.screen_size.y / 720.0)
    } else {
        state.preview_scale
    };
    let extent = if state.screen_preview { state.screen_size / ui_scale } else { size };
    let zoom = if state.zoom == 0.0 {
        ((available.x - 40.0) / (extent.x * ui_scale))
            .min((available.y - 48.0) / (extent.y * ui_scale))
            .clamp(0.1, 1.0)
    } else {
        state.zoom
    };
    let scale = state.resize.as_ref().map_or(zoom * ui_scale, |r| r.scale);
    let mut stage = Stage {
        authored: &mut *authored,
        shown: other.as_deref(),
        s,
        assets,
        state: &mut *state,
        closed,
        row_values,
        context_prefix: context_prefix.as_deref(),
        size,
        extent,
        scale,
        ui_scale,
        available,
        changes: Vec::new(),
        selected_bounds: None,
        dropped: None,
        unsupported: BTreeSet::new(),
        close_requested: false,
        collapse_requested: None,
    };
    let room = egui::ScrollArea::both()
        .id_salt("nui-canvas")
        .auto_shrink([false, false])
        .max_height(available.y)
        .show(ui, |ui| stage.draw(ui))
        .inner_rect;
    let Stage { changes, dropped, unsupported, close_requested, collapse_requested, .. } = stage;
    let last_event = (runtime.as_ref())
        .filter(|r| state.preview_interactive && !r.last_event.is_empty())
        .map(|r| r.last_event.clone());
    let said =
        Said { size, ui_scale, unsupported: &unsupported, last_event: last_event.as_deref() };
    corners(ui, room, state, assets, &said);
    if let Some((path, ty)) = dropped {
        state.selected = path;
        design::insert(authored, &mut state.selected, ty);
        if ty == "swap" {
            state.new_swap_slot = Some(state.selected.clone());
        }
    }
    release_gestures(ui, authored, state);
    if let Some(r) = &mut runtime {
        simulate(ui, r, changes, close_requested, collapse_requested);
    }
    state.runtime = runtime;
}

/// The window alone, as the canvas draws it with nothing selected and no
/// guides, its top left at the top left of `ui`, at `state`'s UI scale
/// (`mg nui render` and `layout`): no toolbar, no zoom, no input taken.
/// Returns the kinds of control it could not draw.
pub(in crate::nui_view) fn still(
    ui: &mut Ui,
    v: &mut Value,
    s: &Settings,
    state: &mut State,
    assets: &skin::Assets,
) -> BTreeSet<String> {
    if assets.recording() {
        // (A pass egui discards draws the window again: one frame's worth.)
        assets.start_record();
    }
    let size = window_size(v, s);
    let scale = state.preview_scale;
    let space = Rect::from_min_size(ui.max_rect().min, size * scale);
    let empty_rows = Default::default();
    let mut stage = Stage {
        authored: v,
        shown: None,
        s,
        assets,
        state,
        closed: false,
        row_values: &empty_rows,
        context_prefix: None,
        size,
        extent: size,
        scale,
        ui_scale: scale,
        available: size * scale,
        changes: Vec::new(),
        selected_bounds: None,
        dropped: None,
        unsupported: BTreeSet::new(),
        close_requested: false,
        collapse_requested: None,
    };
    let p = ui.painter().with_clip_rect(space.intersect(ui.clip_rect()));
    stage.window(ui, &p, space, space);
    stage.unsupported
}

/// A resize or a move of the window under way: dropped when the window
/// changed under it or on Esc, and following the pointer otherwise.
fn track_gestures(ui: &Ui, v: &Value, state: &mut State) {
    if state.resize.as_ref().is_some_and(|r| r.source != *v)
        || ui.input(|i| i.key_pressed(egui::Key::Escape))
    {
        state.resize = None;
    }
    if state.window_move.as_ref().is_some_and(|r| r.source != *v)
        || ui.input(|i| i.key_pressed(egui::Key::Escape))
    {
        state.window_move = None;
    }
    if let Some(m) = &mut state.window_move
        && let Some(pointer) = ui.input(|i| i.pointer.latest_pos())
    {
        m.update(pointer);
    }
    if let Some(resize) = &mut state.resize
        && let Some(pointer) = ui.input(|i| i.pointer.latest_pos())
    {
        resize.move_to(pointer);
    }
}

/// The simulation of Interact: a new one when the window or its settings
/// changed, with the size the client would give it.
fn session(
    generation: Option<u64>,
    v: &Value,
    s: &Settings,
    state: &mut State,
) -> Option<interaction::Session> {
    let mut runtime = state.runtime.take();
    if state.preview_interactive && runtime.as_mut().is_none_or(|r| !r.matches(generation, v, s)) {
        runtime = Some(interaction::Session::new(v, s, generation));
        state.forget_scrolls();
    }
    if state.preview_interactive
        && let Some(runtime) = &mut runtime
    {
        runtime.constrain_geometry();
    }
    runtime
}

/// A resize or a move of the window ends when the pointer lets go: one edit.
fn release_gestures(ui: &Ui, authored: &mut Value, state: &mut State) {
    let released = ui.input(|i| i.pointer.any_released());
    if released
        && let Some(resize) = state.resize.take()
        && resize.source == *authored
        && resize.changed()
    {
        resize.apply(authored);
    }
    if released
        && let Some(m) = state.window_move.take()
        && m.source == *authored
        && m.position != m.initial
    {
        m.apply(authored);
    }
}

/// What the frame's input does in the simulation.
fn simulate(
    ui: &Ui,
    r: &mut interaction::Session,
    changes: Vec<interaction::Change>,
    close_requested: bool,
    collapse_requested: Option<bool>,
) {
    if let Some(collapsed) = collapse_requested {
        r.collapse(collapsed);
        ui.ctx().request_repaint();
    }
    if close_requested {
        r.dispatch("close", "", None, 0);
        r.closed = true;
        ui.ctx().request_repaint();
    }
    if !changes.is_empty() {
        ui.ctx().request_repaint();
    }
    for change in changes {
        r.apply(change);
    }
}

/// What the canvas says of what it shows, at its corners.
struct Said<'a> {
    size: Vec2,
    ui_scale: f32,
    unsupported: &'a BTreeSet<String>,
    last_event: Option<&'a str>,
}

/// What stands over the canvas's corners (M4's U19, the NUI board's
/// `vp-tl` and `vp-br`), over the window's room and not in rows of their
/// own: at the top left "Game preview · UI scale 100% · State: Follow
/// pointer" (the window's size and where its resources come from in its
/// tip; in Interact what the canvas does then), the resources' issues under
/// it, or with View's Resources where each picture and font came from; at
/// the bottom left what is not drawn here and Interact's last event; at the
/// bottom right the zoom, Fit and 100%. Each on a chip of the bar's ground,
/// so that it reads in both themes over the game's black; only the zoom
/// takes the pointer, and only where it stands: the canvas beside them
/// takes its clicks.
fn corners(ui: &mut Ui, room: Rect, state: &mut State, assets: &skin::Assets, said: &Said<'_>) {
    let t = mg_uikit::tokens(ui.ctx());
    let pad = 8.0;
    let inner = room.shrink(pad);
    let chip = |ui: &mut Ui, at: Rect, add: &mut dyn FnMut(&mut Ui)| {
        let mut child = ui.new_child(
            egui::UiBuilder::new().max_rect(at).layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(room.intersect(ui.clip_rect()));
        egui::Frame::new()
            .fill(t.bar.gamma_multiply(0.92))
            .corner_radius(3)
            .inner_margin(egui::Margin::symmetric(6, 2))
            .show(&mut child, |ui| add(ui))
            .response
    };
    let scale = (said.ui_scale * 100.0).round() as u32;
    let label = match (state.preview_interactive, assets.loaded) {
        (true, _) => format!("Interact · UI scale {scale}%"),
        (false, true) => {
            format!(
                "Game preview · UI scale {scale}% · State: {}",
                super::super::STATES[state.preview_state]
            )
        }
        (false, false) => format!("Preview · game skin unavailable · UI scale {scale}%"),
    };
    let tip = match state.preview_interactive {
        true => {
            "Controls take values here (an edit starts over), and handlers' code runs only in the game"
                .to_string()
        }
        false => format!(
            "{} × {}: game resources, local layout preview",
            said.size.x as u32, said.size.y as u32
        ),
    };
    let top = chip(ui, inner, &mut |ui| {
        ui.label(egui::RichText::new(&label).small().color(t.text_2)).on_hover_text(&tip);
    });
    let mut below = top.rect.bottom() + 4.0;
    if state.preview_info {
        let at = Rect::from_min_max(egui::pos2(inner.left(), below), inner.max);
        chip(ui, at, &mut |ui| {
            ui.set_max_width(360.0_f32.min(inner.width() - 12.0));
            egui::ScrollArea::vertical().id_salt("nui-resource-info").max_height(120.0).show(
                ui,
                |ui| {
                    for (name, origin) in &assets.origins {
                        ui.small(format!("{name} — {origin}"));
                    }
                    for issue in &assets.issues {
                        ui.colored_label(t.error, issue);
                    }
                    ui.small(
                        "Stock bevel: 4 source pixels. Font rasterization and layout are local; \
                     compare final output in NWN.",
                    );
                },
            );
        });
    } else if !assets.issues.is_empty() {
        let at = Rect::from_min_max(egui::pos2(inner.left(), below), inner.max);
        let words = match assets.issues.len() {
            1 => "1 resource issue · see View › Resources".to_owned(),
            n => format!("{n} resource issues · see View › Resources"),
        };
        below = chip(ui, at, &mut |ui| {
            ui.label(egui::RichText::new(&words).small().color(t.error));
        })
        .rect
        .bottom();
    }
    let _ = below;
    // (Bottom left: what is not drawn here, Interact's last event.)
    let mut lines = Vec::new();
    if !said.unsupported.is_empty() {
        let kinds = said.unsupported.iter().cloned().collect::<Vec<_>>().join(", ");
        lines.push((format!("Not drawn here (see it in the game): {kinds}"), t.warn));
    }
    if let Some(event) = said.last_event {
        lines.push((event.to_owned(), t.text_2));
    }
    let zoom_width = zoom_width(ui);
    if !lines.is_empty() {
        let high = 20.0 * lines.len() as f32 + 4.0;
        let at = Rect::from_min_max(
            egui::pos2(inner.left(), inner.bottom() - high),
            egui::pos2((inner.right() - zoom_width - pad).max(inner.left()), inner.bottom()),
        );
        chip(ui, at, &mut |ui| {
            for (words, color) in &lines {
                ui.add(
                    egui::Label::new(egui::RichText::new(words).small().color(*color)).truncate(),
                );
            }
        });
    }
    // (Bottom right: the zoom.)
    let size = egui::vec2(zoom_width, mg_uikit::metrics::CONTROL_HEIGHT);
    let at = Rect::from_min_size(inner.max - size, size);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(at)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let mut zoom = state.zoom;
    mg_uikit::segmented(&mut child, "Zoom", &mut zoom, &ZOOMS);
    state.zoom = zoom;
}

/// The zoom's choices: Fit, and the window at its size.
const ZOOMS: [(f32, &str, &str); 2] =
    [(0.0, "Fit", "Fit the window in the canvas"), (1.0, "100%", "The window at its size")];

/// The zoom's width, as the kit lays out its choices.
fn zoom_width(ui: &Ui) -> f32 {
    form::segmented_width(ui, egui::Id::new("nui-zoom-switch-w"), &[ZOOMS[0].1, ZOOMS[1].1])
}

/// One frame of the canvas: the window as it is shown, and what drawing it
/// gathers (clicks in a simulation, a control dropped, the selection's place).
pub(super) struct Stage<'a> {
    authored: &'a mut Value,
    /// What is shown when it is not the window itself.
    shown: Option<&'a Value>,
    s: &'a Settings,
    assets: &'a skin::Assets,
    state: &'a mut State,
    /// A simulation whose window was closed.
    closed: bool,
    row_values: &'a std::collections::BTreeMap<(String, usize), Value>,
    context_prefix: Option<&'a str>,
    size: Vec2,
    extent: Vec2,
    /// Points to the canvas's pixels: the UI scale times the zoom.
    scale: f32,
    /// The game's UI scale, whose sizes are whole pixels.
    ui_scale: f32,
    available: Vec2,
    changes: Vec<interaction::Change>,
    selected_bounds: Option<Rect>,
    dropped: Option<(String, &'static str)>,
    unsupported: BTreeSet<String>,
    close_requested: bool,
    collapse_requested: Option<bool>,
}

/// The window's frame: where it is, its title bar and what the bar holds.
pub(super) struct Chrome<'a> {
    rect: Rect,
    header: Rect,
    screen: Rect,
    title: &'a str,
    font: &'a str,
    border: bool,
    collapsed: bool,
    collapsible: bool,
    closable: bool,
}

impl Stage<'_> {
    /// The canvas under the toolbar: the screen, and the window on it.
    fn draw(&mut self, ui: &mut Ui) {
        let (extent, scale, available) = (self.extent, self.scale, self.available);
        let (space, background) = ui.allocate_exact_size(
            (extent * scale + vec2(40.0, 48.0)).max(available),
            Sense::click(),
        );
        self.state.preview_rect = Some(space.intersect(ui.clip_rect()));
        if self.context_prefix.is_none() && background.clicked() && self.state.resize.is_none() {
            self.state.selected.clear();
        }
        let p = ui.painter().with_clip_rect(space.intersect(ui.clip_rect()));
        p.rect_filled(space, 0, Color32::from_rgb(22, 24, 27));
        let screen = Rect::from_center_size(space.center(), extent * scale);
        if self.state.screen_preview {
            p.rect_stroke(screen, 0, Stroke::new(1.0, Color32::DARK_GRAY), StrokeKind::Inside);
        }
        if self.closed {
            p.text(
                space.center(),
                Align2::CENTER_CENTER,
                "Window closed · Reset to reopen",
                egui::FontId::proportional(16.0),
                Color32::LIGHT_GRAY,
            );
            return;
        }
        self.window(ui, &p, space, screen);
    }

    /// The window: its background and border, its title bar, its body and
    /// the handles of what is selected.
    fn window(&mut self, ui: &mut Ui, p: &Painter, space: Rect, screen: Rect) {
        let (s, assets, scale, size, extent) =
            (self.s, self.assets, self.scale, self.size, self.extent);
        let v: &Value = self.shown.unwrap_or(&*self.authored);
        let resizable = flag(&v["resizable"], s, None, true);
        let geometry = resolved(&v["geometry"], s);
        let collapsed = flag(&v["collapsed"], s, None, false);
        let collapsible = self.authored["collapsed"].as_bool() != Some(false);
        let closable = flag(&v["closable"], s, None, true);
        // `false` is how a script asks for no title (and, with collapsing
        // and closing off too, no title bar), not the word "false".
        let title = if *resolved(&v["title"], s) == false {
            String::new()
        } else {
            localized_string(&v["title"], s, None, assets)
        };
        let font = string(&v["font"], s, None);
        let header_height =
            if title.is_empty() && !collapsible && !closable { 0.0 } else { 33.0 * scale };
        let shown_size = if collapsed { vec2(size.x, header_height / scale) } else { size };
        let rect = match self.state.resize.as_ref().filter(|r| r.path.is_empty()) {
            Some(resize) => Rect::from_min_size(space.min + resize.anchor, shown_size * scale),
            None if self.state.screen_preview => {
                let x = num(&geometry["x"], s, None, -1.0);
                let y = num(&geometry["y"], s, None, -1.0);
                let offset = vec2(
                    if x < 0.0 { (extent.x - shown_size.x) / 2.0 } else { x },
                    if y < 0.0 { (extent.y - shown_size.y) / 2.0 } else { y },
                );
                Rect::from_min_size(screen.min + offset * scale, shown_size * scale)
            }
            None => Rect::from_center_size(space.center(), shown_size * scale),
        };
        let border = flag(&v["border"], s, None, true);
        if !flag(&v["transparent"], s, None, false) {
            p.rect_filled(rect, 0, assets.color("window.background", Color32::BLACK));
        }
        if border && !assets.paint(p, "window.border_image", rect, scale, true, Color32::WHITE) {
            p.rect_stroke(rect, 0, Stroke::new(scale, GOLD), StrokeKind::Inside);
        }
        let header = Rect::from_min_size(rect.min, vec2(rect.width(), header_height));
        if header_height > 0.0 {
            self.title_bar(
                ui,
                p,
                &Chrome {
                    rect,
                    header,
                    screen,
                    title: &title,
                    font: &font,
                    border,
                    collapsed,
                    collapsible,
                    closable,
                },
            );
        }
        if self.context_prefix.is_none() && self.state.selected.is_empty() && self.state.guides() {
            p.rect_stroke(
                rect.expand(2.0),
                0,
                Stroke::new(1.5, ui.visuals().selection.bg_fill),
                StrokeKind::Outside,
            );
        }
        let inner = Rect::from_min_max(
            header.left_bottom() + vec2(8.0, 8.0) * scale,
            rect.right_bottom() - vec2(8.0, 8.0) * scale,
        );
        assets.record_window(rect, inner);
        self.body(ui, inner, &font);
        if !collapsed && resizable {
            let corner = rect.right_bottom();
            p.add(egui::Shape::convex_polygon(
                vec![corner - vec2(16.0 * scale, 0.0), corner, corner - vec2(0.0, 16.0 * scale)],
                assets.color("window.scaler", GOLD),
                Stroke::NONE,
            ));
        }
        if self.context_prefix.is_none() && self.state.selected.is_empty() {
            self.selected_bounds = (!collapsed).then_some(rect);
        }
        interaction::resize_handles(
            ui,
            self.authored,
            self.state,
            self.selected_bounds,
            scale,
            space.min,
        );
        if self.context_prefix.is_none() {
            // Where the selected control draws: inside its padding.
            let body = self
                .selected_bounds
                .zip(self.authored.pointer(&self.state.selected))
                .map(|(r, n)| r.shrink(layout_padding(n, s, None) * scale));
            draw::canvas_handles(ui, self.authored, self.state, body, scale, s);
        }
    }

    /// The 33-point title bar: the title, the close and collapse buttons,
    /// and the drag that moves the window on the screen.
    fn title_bar(&mut self, ui: &mut Ui, p: &Painter, chrome: &Chrome<'_>) {
        let (assets, scale) = (self.assets, self.scale);
        let Chrome { rect, header, screen, title, font, border, collapsed, collapsible, closable } =
            *chrome;
        assets.paint(p, "window.header.normal", header, scale, true, Color32::WHITE);
        // Under its 33-point title bar the client frames the body on its
        // own: two lines between them (NWN EE 8193.37, np_group).
        if border && !collapsed {
            let body = Rect::from_min_max(header.left_bottom(), rect.max);
            assets.paint(p, "window.border_image", body, scale, true, Color32::WHITE);
        }
        let mut title_rect = header.shrink2(vec2(10.0, 2.0) * scale);
        let button_count = usize::from(closable) + usize::from(collapsible);
        title_rect.max.x -= 26.0 * button_count as f32 * scale;
        if !title.is_empty() {
            assets.record_title(title);
        }
        text(
            p,
            assets,
            title_rect,
            title,
            TextStyle {
                font,
                scale,
                color: assets.color("window.header.label_normal", Color32::LIGHT_GRAY),
                align: Align2::LEFT_CENTER,
                wrap: false,
            },
        );
        if closable {
            let close = Rect::from_center_size(
                header.right_center() - vec2(17.0 * scale, 0.0),
                vec2(16.0, 16.0) * scale,
            );
            assets.paint(
                p,
                "window.header.close_button.normal",
                close,
                scale,
                false,
                Color32::WHITE,
            );
            if self.state.preview_interactive
                && ui.interact(close, ui.id().with("close-window"), Sense::click()).clicked()
            {
                self.close_requested = true;
            }
        }
        if collapsible {
            let collapse = Rect::from_center_size(
                header.right_center()
                    - vec2((17.0 + if closable { 26.0 } else { 0.0 }) * scale, 0.0),
                vec2(16.0, 16.0) * scale,
            );
            let r = ui.interact(collapse, ui.id().with("collapse-window"), Sense::click());
            r.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    true,
                    if collapsed { "Expand preview window" } else { "Collapse preview window" },
                )
            });
            assets.paint(
                p,
                "window.header.minimize_button.normal",
                collapse,
                scale,
                false,
                Color32::WHITE,
            );
            if r.clicked() && self.state.preview_interactive {
                self.collapse_requested = Some(!collapsed);
            }
        }
        let mut title_hit = header;
        title_hit.max.x -= (8.0 + 26.0 * button_count as f32) * scale;
        let movable = self.context_prefix.is_none()
            && self.state.screen_preview
            && !self.authored["geometry"]["bind"].is_string();
        let r = ui.interact(
            title_hit,
            ui.id().with("nui-window"),
            if movable { Sense::click_and_drag() } else { Sense::click() },
        );
        if movable
            && r.drag_started()
            && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        {
            self.state.window_move = Some(interaction::WindowMove {
                source: self.authored.clone(),
                origin,
                initial: (rect.min - screen.min) / scale,
                position: (rect.min - screen.min) / scale,
                scale,
            });
        }
        r.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Canvas window")
        });
        if self.context_prefix.is_none() && r.clicked() && self.state.resize.is_none() {
            self.state.selected.clear();
        }
    }

    /// The window's controls, inside its 8-point inset.
    fn body(&mut self, ui: &mut Ui, inner: Rect, font: &str) {
        let (s, scale) = (self.s, self.scale);
        let v: &Value = self.shown.unwrap_or(&*self.authored);
        // A group at the root draws its border 4 points into the inset on
        // every side, its controls where they would be without that
        // (8193.37-17: measured on the client's screenshots; `container`).
        let root_group = v["root"]["type"] == "group";
        let reach = if root_group { inner.expand(4.0 * scale) } else { inner };
        // What the last pass worked out of the window, where it is the
        // window itself (not a copy shown in its place), at the same
        // scale, and known since not to have changed (`State::measured`).
        let at = (self.state.window_gen, scale.to_bits(), self.ui_scale.to_bits());
        let trusted = crate::nui_view::trusted(ui.ctx(), self.state) == Some(self.state.window_gen);
        let kept = match self.state.measured.take() {
            Some((was, kept)) if was == at && trusted && self.shown.is_none() => {
                self.state.carried += 1;
                kept.carry()
            }
            _ => Measured::default(),
        };
        let keep = self.shown.is_none();
        let kept = ui.scope_builder(egui::UiBuilder::new().max_rect(reach), |ui| {
            ui.set_clip_rect(reach.intersect(ui.clip_rect()));
            let mut renderer = Renderer {
                s,
                assets: self.assets,
                scale,
                ui_scale: self.ui_scale,
                state: &mut *self.state,
                dropped: &mut self.dropped,
                unsupported: &mut self.unsupported,
                window_font: font,
                changes: &mut self.changes,
                row_values: self.row_values,
                selected_bounds: &mut self.selected_bounds,
                ancestor_disabled: !flag(&v["accepts_input"], s, None, true),
                context_prefix: self.context_prefix,
                reach: None,
                naturals: Naturals::new(kept),
            };
            if !flag(&v["collapsed"], s, None, false) {
                let desired =
                    renderer.scaled(natural(&v["root"], s, None, 0, Some(&renderer.naturals)));
                let mut root = inner;
                // A column made only of Groups has no child width advice.
                // Its padding alone must not collapse the entire canvas.
                let inset = 2.0 * layout_padding(&v["root"], s, None) * scale;
                if desired.x > inset {
                    root.max.x = (root.left() + desired.x).min(inner.right());
                }
                renderer.node(ui, &v["root"], "/root", root, None, 0);
            }
            renderer.naturals.into_inner()
        });
        if keep {
            self.state.measured = Some((at, kept.inner));
        }
    }
}
