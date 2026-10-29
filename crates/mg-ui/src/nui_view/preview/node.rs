//! One control on the canvas: what every kind shares (its place, the
//! pointer, its state and tint), and the kind's own drawing after that.
use super::*;

pub(super) struct Renderer<'a> {
    pub(super) s: &'a Settings,
    pub(super) assets: &'a skin::Assets,
    /// Points to the canvas's pixels: the UI scale times the zoom.
    pub(super) scale: f32,
    /// The game's UI scale.
    pub(super) ui_scale: f32,
    pub(super) state: &'a mut State,
    pub(super) dropped: &'a mut Option<(String, &'static str)>,
    pub(super) unsupported: &'a mut BTreeSet<String>,
    pub(super) window_font: &'a str,
    pub(super) changes: &'a mut Vec<interaction::Change>,
    pub(super) row_values: &'a std::collections::BTreeMap<(String, usize), Value>,
    pub(super) selected_bounds: &'a mut Option<Rect>,
    pub(super) ancestor_disabled: bool,
    pub(super) context_prefix: Option<&'a str>,
}

/// A control being drawn: where, in which state and with what. The kinds'
/// own drawing (`Renderer::button`, `list`...) starts from this.
pub(super) struct Control<'a> {
    pub s: &'a Settings,
    pub a: &'a skin::Assets,
    pub scale: f32,
    pub node: &'a Value,
    pub ty: &'a str,
    /// The control's place in the window being edited, and in what is shown
    /// (the two differ where a variant is shown inside its window).
    pub path: &'a str,
    pub source_path: &'a str,
    pub row: Option<usize>,
    pub depth: usize,
    /// Inside the control's padding.
    pub body: Rect,
    pub p: Painter,
    pub response: egui::Response,
    /// The entries of Options and Tabs.
    pub choices: Vec<Rect>,
    pub editable: bool,
    pub disabled: bool,
    /// A simulation takes input for it.
    pub interactive: bool,
    pub state_key: &'static str,
    pub tint: Color32,
    pub fg: Color32,
    pub font: &'a str,
    /// What the control's input makes of its value (`Some(None)`: a click).
    pub changed: Option<Option<Value>>,
    /// The client's own correction of a value, not an input.
    pub normalization: bool,
}

impl Control<'_> {
    pub(super) fn label(&self, r: Rect, content: &str, align: Align2, wrap: bool) {
        text(
            &self.p,
            self.a,
            r,
            content,
            TextStyle { font: self.font, scale: self.scale, color: self.fg, align, wrap },
        );
    }

    pub(super) fn frame(&self, section: &str, r: Rect) {
        let (a, p, scale, tint) = (self.a, &self.p, self.scale, self.tint);
        let state_key = self.state_key;
        if !a.paint(p, &format!("{section}.{state_key}"), r, scale, true, tint) {
            // (No skin: the state still shows, a lit edge under the
            // pointer and a darker gold while pressed.)
            let (fill, edge) = match state_key {
                "hover" => (Color32::BLACK, Color32::from_rgb(230, 200, 140)),
                "active" => (Color32::from_rgb(60, 45, 20), GOLD),
                _ => (Color32::BLACK, GOLD),
            };
            p.rect_filled(r, 0, fill);
            p.rect_stroke(r, 0, Stroke::new(scale, edge * tint), StrokeKind::Inside);
        }
    }

    pub(super) fn props(&self) -> Props<'_> {
        Props { node: self.node, s: self.s, row: self.row }
    }
}

/// The skin's name for how a control looks now: `mode` is 2 under the
/// pointer and 3 while pressed.
fn look(disabled: bool, mode: usize, encouraged: bool) -> &'static str {
    if disabled {
        "normal"
    } else {
        match mode {
            2 => "hover",
            3 => "active",
            // An encouraged control breathes between its normal and hover
            // looks (NWN EE 8193.37, np_misc); a still preview shows the peak.
            _ if encouraged => "hover",
            _ => "normal",
        }
    }
}

/// The skin's section that colors a kind's text.
fn text_section(ty: &str) -> &'static str {
    match ty {
        "button" | "button_image" => "button",
        "button_select" => "selectable",
        "check" => "checkbox",
        "textedit" => "edit",
        "combo" => "combo",
        _ => "text",
    }
}

impl Renderer<'_> {
    /// A control's size in points on the canvas as the game scales it:
    /// whole pixels at its UI scale, rounded down (NWN EE 8193.37: a row 34
    /// high is 37 at 111%, not 37.7), then zoomed.
    pub(super) fn scaled(&self, size: Vec2) -> Vec2 {
        let ui = if self.ui_scale > 0.0 { self.ui_scale } else { 1.0 };
        // (A hair over, so that a size that is whole in decimals stays so.)
        (size * ui + Vec2::splat(1e-3)).floor() * (self.scale / ui)
    }

    /// A control in the room `outer` its parent gives it: drawn, and noted
    /// where it went when that is asked for (`mg nui layout`), drawn or not.
    pub(super) fn node(
        &mut self,
        ui: &mut Ui,
        node: &Value,
        path: &str,
        outer: Rect,
        row: Option<usize>,
        depth: usize,
    ) {
        let entered = depth <= 32
            && outer.is_finite()
            && self.assets.recording()
            && self.assets.enter(
                path,
                row,
                node,
                outer.shrink(extra_margin(node, self.s, row) * self.scale),
                ui.clip_rect(),
            );
        self.draw_node(ui, node, path, outer, row, depth);
        self.assets.leave(entered);
    }

    fn draw_node(
        &mut self,
        ui: &mut Ui,
        node: &Value,
        path: &str,
        outer: Rect,
        row: Option<usize>,
        depth: usize,
    ) {
        // (A rectangle that is not finite comes of sizes no window has, such
        // as an aspect of 1e-40.) What can't be seen isn't drawn, unless
        // where it goes is asked for: then it is laid out, its controls with
        // it, and drawn clipped, to nothing.
        if depth > 32
            || !outer.is_finite()
            || !outer.is_positive()
            || (!outer.intersects(ui.clip_rect()) && !self.assets.recording())
        {
            return;
        }
        let source_path = path;
        let mapped = layouts::canvas_path(path, self.context_prefix);
        let editable = mapped.is_some();
        let path = mapped.as_deref().unwrap_or(path);
        let mut local_node;
        let node = if let Some(value) = row.and_then(|r| self.row_values.get(&(path.into(), r))) {
            local_node = node.clone();
            local_node["value"] = value.clone();
            &local_node
        } else {
            node
        };
        let ty = node["type"].as_str().unwrap_or("unknown");
        let s = self.s;
        let scale = self.scale;
        let margin = extra_margin(node, s, row) * scale;
        let rect = outer.shrink(margin);
        if !rect.is_positive() {
            return;
        }
        let live = self.state.preview_interactive && editable;
        let padding = layout_padding(node, s, row) * scale;
        let body = rect.shrink(padding);
        let choices = if matches!(ty, "options" | "tabbar") {
            choice_rects(
                body,
                val(&node["elements"], s, row).as_array().map_or(0, Vec::len),
                val(&node["direction"], s, row).as_i64() == Some(1),
                ty == "tabbar",
                scale,
            )
        } else {
            Vec::new()
        };
        let painted_bounds = choices.iter().fold(rect, |bounds, item| bounds.union(*item));
        let response = ui.interact(
            if live { painted_bounds } else { rect },
            ui.id().with(("canvas", source_path, row)),
            if !editable {
                Sense::hover()
            } else if !live || matches!(ty, "slider" | "sliderf") {
                Sense::click_and_drag()
            } else {
                Sense::click()
            },
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                format!(
                    "{} {}{}",
                    if editable { "Canvas" } else { "Context" },
                    design::node_name(node),
                    row.map_or(String::new(), |r| format!(" · row {r}"))
                ),
            )
        });
        if editable {
            self.select_and_drag(ui, node, path, &response, live);
        }
        let selected = editable && self.state.selected == path;
        if selected && self.selected_bounds.is_none() {
            *self.selected_bounds = Some(rect);
        }
        let visible = flag(&node["visible"], s, row, true);
        let parent_disabled = self.ancestor_disabled;
        // (A state asked for off the screen, by the control's ID or for all.
        // Disabled by its ID, what is in it is too, as in the game; all
        // disabled, each is, and one given another state by ID has it.)
        let still = &self.state.still_states;
        let id = node["id"].as_str();
        let in_row = id
            .zip(row)
            .and_then(|(id, r)| self.state.still_row_states.get(&(id.to_owned(), r)).copied());
        let by_id = in_row.or_else(|| id.and_then(|id| still.get(&Some(id.to_owned()))).copied());
        let asked = by_id.or_else(|| still.get(&None).copied());
        let disabled = parent_disabled
            || !flag(&node["enabled"], s, row, true)
            || (!live && selected && self.state.preview_state == 4);
        self.ancestor_disabled = disabled || by_id == Some(4);
        let disabled = disabled || asked == Some(4);
        let mode = if let Some(mode) = asked.filter(|m| *m != 4) {
            mode
        } else if !live && selected && self.state.preview_state > 0 {
            self.state.preview_state
        } else if response.is_pointer_button_down_on() {
            3
        } else if response.hovered() {
            2
        } else {
            1
        };
        let state_key = look(disabled, mode, flag(&node["encouraged"], s, row, false));
        self.assets.record_look(match state_key {
            _ if disabled => "disabled",
            "hover" => "hover",
            "active" => "pressed",
            _ => "normal",
        });
        let tint = if disabled { Color32::from_gray(115) } else { Color32::WHITE };
        let p = ui.painter().with_clip_rect(painted_bounds.intersect(ui.clip_rect()));
        let a = self.assets;
        let default_color = a.color("text.color", Color32::WHITE);
        let text_section = text_section(ty);
        let default_color = a.color(&format!("{text_section}.text_{state_key}"), default_color);
        let fg = rgba(val(&node["foreground_color"], s, row), default_color) * tint;
        let font = localized_string(&node["font"], s, row, a);
        let font = if font.is_empty() { self.window_font } else { &font };
        let interactive = live && visible && !disabled;
        let mouse = ui.input(|i| {
            [
                response.contains_pointer(),
                i.pointer.primary_down(),
                i.pointer.secondary_down(),
                i.pointer.middle_down(),
            ]
        });
        let mut c = Control {
            s,
            a,
            scale,
            node,
            ty,
            path,
            source_path,
            row,
            depth,
            body,
            p,
            response,
            choices,
            editable,
            disabled,
            interactive,
            state_key,
            tint,
            fg,
            font,
            changed: None,
            normalization: false,
        };
        if visible {
            draw::paint(ui.painter(), node, body, scale, s, a, row, true, mouse);
        }
        if interactive && c.response.clicked() {
            match ty {
                "button" | "button_image" => c.changed = Some(None),
                // A Toggle button with a literal value stays as it is in the
                // client, on or off (NWN EE 8193.37, np_bsel); a bound one flips.
                "button_select" if node["value"].get("bind").is_none() => {}
                "check" | "button_select" => {
                    c.changed = Some(Some(json!(!flag(&node["value"], s, row, false))))
                }
                _ => {}
            }
        }
        if !visible {
            if self.state.guides() {
                c.p.rect_stroke(rect, 0, Stroke::new(1.0, Color32::DARK_GRAY), StrokeKind::Inside);
                text(
                    &c.p,
                    a,
                    rect,
                    "Hidden",
                    TextStyle {
                        font,
                        scale,
                        color: Color32::GRAY,
                        align: Align2::CENTER_CENTER,
                        wrap: false,
                    },
                );
            }
        } else {
            self.control(ui, &mut c);
        }
        if visible {
            draw::paint(ui.painter(), node, body, scale, s, a, row, false, mouse);
        }
        self.ancestor_disabled = parent_disabled;
        self.finish(ui, c, rect, selected, live);
    }

    /// After a control is drawn: its input goes to the simulation, and it
    /// gets its outline and tooltip.
    fn finish(&mut self, ui: &mut Ui, c: Control<'_>, rect: Rect, selected: bool, live: bool) {
        let Control { s, a, node, path, row, disabled, .. } = c;
        if let Some(value) = c.changed
            && value.as_ref().is_none_or(|v| v != val(&node["value"], s, row))
        {
            self.changes.push(interaction::Change {
                path: path.into(),
                row,
                value,
                normalization: c.normalization,
            });
        }
        if self.state.guides()
            && (selected
                || (!live && c.response.hovered())
                || c.response.dnd_hover_payload::<design::Insert>().is_some())
        {
            ui.painter().rect_stroke(
                rect,
                0,
                Stroke::new(if selected { 2.0 } else { 1.0 }, ui.visuals().selection.bg_fill),
                StrokeKind::Inside,
            );
        }
        let tooltip = if disabled { &node["disabled_tooltip"] } else { &node["tooltip"] };
        let tooltip = localized_string(tooltip, s, row, a);
        if !tooltip.is_empty() {
            c.response.on_hover_text(tooltip);
        }
    }

    /// What the pointer does to a control that is edited: a click selects
    /// it, a drag moves it in the structure, and a layout takes what is
    /// dropped on it.
    fn select_and_drag(
        &mut self,
        ui: &mut Ui,
        node: &Value,
        path: &str,
        response: &egui::Response,
        live: bool,
    ) {
        let layout = matches!(node["type"].as_str(), Some("col" | "row" | "group" | "list"));
        if response.clicked() && !live && self.state.resize.is_none() {
            structure::select(self.state, path, ui.input(|i| i.modifiers.command));
        }
        if !live && response.drag_started() {
            egui::DragAndDrop::set_payload(
                ui.ctx(),
                structure::Drag { path: path.into(), node: node.clone() },
            );
        }
        if layout {
            structure::drag_target(ui, response, path, self.state, true);
        }
        if layout && let Some(payload) = response.dnd_release_payload::<design::Insert>() {
            *self.dropped = Some((path.into(), payload.0));
        }
    }

    /// The control's own drawing, by its kind.
    fn control(&mut self, ui: &mut Ui, c: &mut Control<'_>) {
        match c.ty {
            "col" | "row" | "group" => self.container(ui, c),
            "list" => self.list(ui, c),
            "button" | "button_image" | "button_select" => self.button(c),
            "label" => self.label(c),
            "text" => self.text_block(ui, c),
            "textedit" => self.textedit(ui, c),
            "combo" => self.combo(ui, c),
            "check" => self.check(c),
            "slider" | "sliderf" | "progress" => self.slider(ui, c),
            "image" => self.image(c),
            "options" | "tabbar" => self.choices(ui, c),
            "color_picker" => self.color_picker(ui, c),
            "chart" => self.chart(ui, c),
            "spacer" => {}
            other => {
                let (p, body, node) = (&c.p, c.body, c.node);
                self.unsupported.insert(other.into());
                p.rect_stroke(body, 0, Stroke::new(1.0, GOLD), StrokeKind::Inside);
                c.label(
                    body,
                    &format!("{} · preview in NWN", design::node_name(node)),
                    Align2::CENTER_CENTER,
                    true,
                );
            }
        }
    }

    pub(super) fn scrollbar(
        &self,
        p: &Painter,
        rect: Rect,
        fraction: f32,
        offset: f32,
        horizontal: bool,
    ) {
        let width = self.assets.number(
            if horizontal { "window.scrollbar_size_y" } else { "window.scrollbar_size_x" },
            18.0,
        ) * self.scale;
        let inset = 2.0 * self.scale;
        let strip = if horizontal {
            Rect::from_min_max(
                pos2(rect.left() + inset, rect.bottom() - width - inset),
                rect.right_bottom() - Vec2::splat(inset),
            )
        } else {
            Rect::from_min_max(
                pos2(rect.right() - width - inset, rect.top() + inset),
                rect.right_bottom() - Vec2::splat(inset),
            )
        };
        let axis = usize::from(!horizontal);
        let mut dec = strip;
        dec.max[axis] = (dec.min[axis] + width).min(strip.max[axis]);
        // Nuklear doesn't shrink the end buttons of a short bar: the second
        // is drawn over the first (NWN EE 8193.37, a one-entry combo list).
        let mut inc = strip;
        inc.min[axis] = (inc.max[axis] - width).max(strip.min[axis]);
        let mut track = strip;
        track.min[axis] = dec.max[axis];
        track.max[axis] = inc.min[axis].max(track.min[axis]);
        let section = if horizontal { "scrollh" } else { "scrollv" };
        for (property, r) in
            [("normal", track), ("dec_button.normal", dec), ("inc_button.normal", inc)]
        {
            self.assets.paint(
                p,
                &format!("{section}.{property}"),
                r,
                self.scale,
                true,
                Color32::WHITE,
            );
        }
        let mut thumb = track.shrink(self.scale * 2.0);
        let length = thumb.size()[axis];
        if length <= 0.0 {
            return;
        }
        let size = length * fraction.clamp(0.05, 1.0);
        thumb.min[axis] += (length - size) * offset.clamp(0.0, 1.0);
        thumb.max[axis] = thumb.min[axis] + size;
        self.assets.paint_tiled(
            p,
            &format!("{section}.cursor_normal"),
            thumb,
            self.scale,
            horizontal,
            Color32::WHITE,
        );
    }
}
