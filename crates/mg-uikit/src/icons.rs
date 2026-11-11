//! The drawn icon set: 194 icons in a 24 by 24 box, strokes 1.5 wide with
//! round caps and joins, no fill, drawn at 16 px (the spec's section 8).
//!
//! The set is `icons/icons.json` (each name's SVG markup), made into Rust
//! at build time (`build.rs`): [`Icon`], one variant a name, and each
//! icon's strokes flattened to polylines. [`Icon::paint`] draws one with
//! egui's painter at any size; [`icon`], [`icon_button`], [`icon_toggle`],
//! [`icon_text_button`] and [`icon_text_toggle`] are the widgets.

use egui::{Color32, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, WidgetInfo, WidgetType};

use crate::metrics;
use crate::theme::TokensExt as _;
use crate::tokens::Tokens;

/// One stroke of an icon, in its 24 by 24 box.
#[derive(Debug)]
struct IconShape {
    /// The last point joins the first.
    closed: bool,
    /// Filled as well as stroked.
    filled: bool,
    points: &'static [[f32; 2]],
}

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

fn strokes(icon: Icon) -> &'static [IconShape] {
    SHAPES[icon as usize]
}

/// How sharp a corner of a stroke is before it is rounded with a dot (the
/// cosine of the turn): a turn of more than 60°.
const SHARP: f32 = 0.5;

impl Icon {
    /// The icon's strokes as polylines in its 24 by 24 box (`(points,
    /// closed)`), for tests and other painters.
    pub fn polylines(self) -> impl Iterator<Item = (&'static [[f32; 2]], bool)> {
        strokes(self).iter().map(|s| (s.points, s.closed))
    }

    /// Paints the icon into `rect` (its 24 box scaled to the square in the
    /// middle of it) in `color`: strokes 1.5 units wide, scaled, never
    /// under one physical pixel, with round caps and corners; the square's
    /// corner on a physical pixel, so that a 16 px icon is crisp.
    pub fn paint(self, painter: &egui::Painter, rect: Rect, color: Color32) {
        record(painter.ctx(), self, rect);
        let ppp = painter.pixels_per_point();
        let side = rect.width().min(rect.height());
        let corner = rect.center() - Vec2::splat(side / 2.0);
        let corner = Pos2::new((corner.x * ppp).round() / ppp, (corner.y * ppp).round() / ppp);
        let scale = side / 24.0;
        let width = (metrics::ICON_STROKE * scale).max(1.0 / ppp);
        let stroke = Stroke::new(width, color);
        let at = |p: &[f32; 2]| corner + Vec2::new(p[0], p[1]) * scale;
        // (The strokes with as many points as the size drawn needs: a
        // tenth of a physical pixel off at most. The set's curves are cut
        // fine enough for 32 px, and at 16 px each icon was hundreds of
        // vertices.)
        let level = simplify_level(scale * ppp);
        let mut shapes = Vec::new();
        for s in simplified(self, level).iter() {
            let points: Vec<Pos2> = s.points.iter().map(at).collect();
            if s.filled && points.len() > 2 {
                shapes.push(Shape::convex_polygon(points.clone(), color, Stroke::NONE));
            }
            // Round caps and sharp corners: a dot as wide as the stroke
            // (a path of `h.01`, a dot, is all cap).
            for &i in &s.dots {
                shapes.push(Shape::circle_filled(points[i], width / 2.0, color));
            }
            if s.closed {
                shapes.push(Shape::closed_line(points, stroke));
            } else {
                shapes.push(Shape::line(points, stroke));
            }
        }
        painter.extend(shapes);
    }
}

/// A stroke as painted at a size: its points, and those that get a round
/// dot (its ends, its sharp corners).
#[derive(Debug)]
struct Painted {
    points: Vec<[f32; 2]>,
    closed: bool,
    filled: bool,
    dots: Vec<usize>,
}

/// How finely the strokes are kept for `px` physical pixels a unit of the
/// 24 box: the tolerance's step (a 64th of a unit), at most a tenth of a
/// pixel.
fn simplify_level(px: f32) -> u8 {
    let tolerance = 0.1 / px.max(0.01);
    (tolerance * 64.0).clamp(0.0, 64.0) as u8
}

type Cache = std::collections::HashMap<(Icon, u8), std::sync::Arc<Vec<Painted>>>;

/// `icon`'s strokes simplified to `level` ([`simplify_level`]), worked out
/// once a size (the set is the program's own: a cache of it alone).
fn simplified(icon: Icon, level: u8) -> std::sync::Arc<Vec<Painted>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<Cache>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Ok(cache) = cache.lock()
        && let Some(found) = cache.get(&(icon, level))
    {
        return found.clone();
    }
    let tolerance = f32::from(level) / 64.0;
    let made: Vec<Painted> = strokes(icon)
        .iter()
        .map(|s| {
            let points = if s.points.len() > 2 && !s.filled {
                douglas_peucker(s.points, tolerance, s.closed)
            } else {
                s.points.to_vec()
            };
            let dots = dots(&points, s.closed);
            Painted { points, closed: s.closed, filled: s.filled, dots }
        })
        .collect();
    let made = std::sync::Arc::new(made);
    if let Ok(mut cache) = cache.lock() {
        cache.insert((icon, level), made.clone());
    }
    made
}

/// The points of a stroke that get a dot: an open one's ends, and every
/// corner sharper than [`SHARP`].
fn dots(points: &[[f32; 2]], closed: bool) -> Vec<usize> {
    let n = points.len();
    let v = |a: [f32; 2], b: [f32; 2]| Vec2::new(b[0] - a[0], b[1] - a[1]).normalized();
    (0..n)
        .filter(|&i| {
            let end = !closed && (i == 0 || i == n - 1);
            let corner = (closed || (i > 0 && i < n - 1)) && n > 1 && {
                let prev = points[(i + n - 1) % n];
                let next = points[(i + 1) % n];
                v(prev, points[i]).dot(v(points[i], next)) < SHARP
            };
            end || corner
        })
        .collect()
}

/// `points` with those dropped that lie within `tolerance` of the line
/// their neighbors kept make (Douglas and Peucker's); a closed stroke
/// keeps its first point and the one farthest from it.
fn douglas_peucker(points: &[[f32; 2]], tolerance: f32, closed: bool) -> Vec<[f32; 2]> {
    if tolerance <= 0.0 || points.len() < 3 {
        return points.to_vec();
    }
    fn distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
        let (ab, ap) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
        let len = (ab[0] * ab[0] + ab[1] * ab[1]).sqrt();
        if len < 1e-6 {
            return (ap[0] * ap[0] + ap[1] * ap[1]).sqrt();
        }
        (ab[0] * ap[1] - ab[1] * ap[0]).abs() / len
    }
    fn keep(points: &[[f32; 2]], tolerance: f32, out: &mut Vec<bool>, from: usize, to: usize) {
        if to <= from + 1 {
            return;
        }
        let (mut far, mut at) = (0.0, from);
        for i in from + 1..to {
            let d = distance(points[i], points[from], points[to]);
            if d > far {
                (far, at) = (d, i);
            }
        }
        if far > tolerance {
            out[at] = true;
            keep(points, tolerance, out, from, at);
            keep(points, tolerance, out, at, to);
        }
    }
    let n = points.len();
    let mut kept = vec![false; n];
    kept[0] = true;
    if closed {
        // (Split at the point farthest from the first: two open halves.)
        let d = |p: [f32; 2]| (p[0] - points[0][0]).powi(2) + (p[1] - points[0][1]).powi(2);
        let far = (1..n).max_by(|&a, &b| d(points[a]).total_cmp(&d(points[b]))).unwrap_or(0);
        kept[far] = true;
        keep(points, tolerance, &mut kept, 0, far);
        let mut ring: Vec<[f32; 2]> = points[far..].to_vec();
        ring.push(points[0]);
        let mut back = vec![false; ring.len()];
        keep(&ring, tolerance, &mut back, 0, ring.len() - 1);
        for (i, k) in back.iter().enumerate().take(ring.len() - 1).skip(1) {
            if *k {
                kept[far + i] = true;
            }
        }
    } else {
        kept[n - 1] = true;
        keep(points, tolerance, &mut kept, 0, n - 1);
    }
    points.iter().zip(kept).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

/// An icon, 16 px, in the current text color; not interactive.
pub fn icon(ui: &mut Ui, icon: Icon) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(metrics::ICON), Sense::hover());
    if ui.is_rect_visible(rect) {
        icon.paint(ui.painter(), rect, ui.visuals().text_color());
    }
    response
}

/// How an icon button is drawn in a state ([`paint_icon_button`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonState {
    /// At rest.
    #[default]
    Rest,
    /// Under the pointer.
    Hovered,
    /// Pressed, or holding the keyboard's focus.
    Pressed,
}

impl ButtonState {
    /// The state a widget's response is in.
    pub fn of(response: &Response) -> Self {
        if response.is_pointer_button_down_on() || response.has_focus() {
            ButtonState::Pressed
        } else if response.hovered() || response.highlighted() {
            ButtonState::Hovered
        } else {
            ButtonState::Rest
        }
    }
}

/// Paints an icon button (the mock's `.btn-icon`; a toggle's `.toggle`
/// when `on` is given) in `rect`: frameless at rest, the icon in `text-2`;
/// hovered, `raised` and the icon in `text`; pressed or focused, an accent
/// ring; on, `accent-fill` and the icon in `on-fill`.
pub fn paint_icon_button(
    painter: &egui::Painter,
    tokens: &Tokens,
    rect: Rect,
    icon: Icon,
    state: ButtonState,
    on: bool,
) {
    let t = tokens;
    let radius = metrics::RADIUS;
    let (fill, color) = match (on, state) {
        (true, ButtonState::Rest) => (t.accent_fill, t.on_fill),
        (true, _) => (t.accent_fill_hover, t.on_fill),
        (false, ButtonState::Rest) => (Color32::TRANSPARENT, t.text_2),
        (false, ButtonState::Hovered) => (t.raised, t.text),
        (false, ButtonState::Pressed) => (t.raised_hover, t.text),
    };
    let ring =
        if state == ButtonState::Pressed { Stroke::new(1.0, t.accent) } else { Stroke::NONE };
    painter.rect(rect, radius, fill, ring, egui::StrokeKind::Inside);
    let square = Rect::from_center_size(rect.center(), Vec2::splat(metrics::ICON));
    icon.paint(painter, square, color);
}

fn icon_button_ui(ui: &mut Ui, icon: Icon, tooltip: &str, on: Option<bool>) -> Response {
    let size = Vec2::splat(metrics::ICON_BUTTON);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let enabled = ui.is_enabled();
    response.widget_info(|| match on {
        Some(on) => WidgetInfo::selected(WidgetType::Button, enabled, on, tooltip),
        None => WidgetInfo::labeled(WidgetType::Button, enabled, tooltip),
    });
    if ui.is_rect_visible(rect) {
        let tokens = ui.tokens();
        let state = ButtonState::of(&response);
        paint_icon_button(ui.painter(), &tokens, rect, icon, state, on == Some(true));
    }
    response.on_hover_text(tooltip).on_disabled_hover_text(tooltip)
}

/// An icon button: 22 px square, frameless until hovered (the mock's
/// `.btn-icon`). `tooltip` is shown on hover, also while disabled, and is
/// its accessible name, by which tests and screen readers find it.
pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    icon_button_ui(ui, icon, tooltip, None)
}

/// An icon button that toggles `on` when clicked (the mock's `.toggle`
/// with an icon): `accent-fill` while on. Changed when clicked.
pub fn icon_toggle(ui: &mut Ui, on: &mut bool, icon: Icon, tooltip: &str) -> Response {
    let mut response = icon_button_ui(ui, icon, tooltip, Some(*on));
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response
}

fn icon_text_button_ui(ui: &mut Ui, icon: Icon, text: &str, selected: Option<bool>) -> Response {
    let id = ui.next_auto_id().with("mg-uikit-icon");
    let side = Vec2::splat(metrics::ICON);
    let mut button = egui::Button::new((egui::Atom::custom(id, side), text));
    if let Some(selected) = selected {
        button = button.selected(selected);
    }
    let atoms = button.atom_ui(ui);
    if let Some(rect) = atoms.rect(id) {
        let r = &atoms.response;
        let visuals = ui.style().interact_selectable(r, selected == Some(true));
        icon.paint(ui.painter(), rect, visuals.text_color());
    }
    atoms.response
}

/// A button with an icon and words (`Save`), egui's own button: its
/// accessible name is the words alone.
pub fn icon_text_button(ui: &mut Ui, icon: Icon, text: &str) -> Response {
    icon_text_button_ui(ui, icon, text, None)
}

/// How far a word button's leading icon reaches into its padding (the
/// mock's `.btn .ic { margin-left: -2px }`).
const ICON_INSET: f32 = 2.0;

/// How an [`IconButton`] is labelled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Label {
    /// The icon and the words.
    #[default]
    Both,
    /// The icon alone (the words its accessible name; its tooltip is the
    /// caller's).
    IconOnly,
    /// The words alone.
    WordsOnly,
}

/// A button labelled with an icon, its words, or both (the mock's word
/// button with a leading icon, or `.btn-icon`), as a widget: added with
/// `ui.add` or `ui.add_enabled` (a disabled one fades whole, icon and
/// words). Its accessible name is the words alone however it is labelled,
/// so `get_by_label("Save")` finds it with or without the words shown.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use mg_uikit::{Icon, IconButton};
/// let save = ui.add(IconButton::new(Icon::Save, "Save").small());
/// let undo = ui.add(IconButton::new(Icon::Undo, "Undo").icon_only(true));
/// # let _ = (save, undo);
/// # });
/// ```
#[derive(Debug)]
#[must_use = "add it with ui.add"]
pub struct IconButton {
    icon: Icon,
    text: egui::WidgetText,
    label: Label,
    small: bool,
    frame: Option<bool>,
    selected: Option<bool>,
    min_height: Option<f32>,
    fill: Option<Color32>,
    sense: Sense,
    gap: Option<f32>,
}

impl IconButton {
    /// `icon` before `text`.
    pub fn new(icon: Icon, text: impl Into<egui::WidgetText>) -> Self {
        IconButton {
            icon,
            text: text.into(),
            label: Label::Both,
            small: false,
            frame: None,
            selected: None,
            min_height: None,
            fill: None,
            sense: Sense::click(),
            gap: None,
        }
    }

    /// Labelled so ([`Label`]).
    pub fn label(mut self, label: Label) -> Self {
        self.label = label;
        self
    }

    /// The icon alone where `only` (the words its accessible name), else
    /// the icon and the words.
    pub fn icon_only(self, only: bool) -> Self {
        self.label(if only { Label::IconOnly } else { Label::Both })
    }

    /// No padding above and below the words (egui's `Button::small`).
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    /// Whether a frame is drawn while it is not hovered (egui's
    /// `Button::frame_when_inactive`; an icon alone has none at rest).
    pub fn frame_when_inactive(mut self, frame: bool) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Down (a toggle's on: `accent-fill`, the icon in `on-fill`).
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// The room between the icon and the words (the theme's
    /// `icon_spacing`, the mock's 6 px, where not given): a crowded
    /// toolbar's 4.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    /// What it senses (a click, egui's button's; a click and a drag for a
    /// button held down to repeat).
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// A toggle's look (egui's `Button::selectable`): down and filled while
    /// `on`, frameless until hovered while off.
    pub fn selectable(self, on: bool) -> Self {
        self.selected(on).frame_when_inactive(on)
    }

    /// Filled with `fill` (egui's `Button::fill`: a main action's
    /// `accent-fill`), its words and icon in `on-fill` when the fill is
    /// the theme's accent.
    pub fn fill(mut self, fill: Color32) -> Self {
        self.fill = Some(fill);
        self
    }

    /// At least so high (a row of buttons that keep one height).
    pub fn min_height(mut self, height: f32) -> Self {
        self.min_height = Some(height);
        self
    }

    /// The width it takes in `ui` (to decide whether a row fits with the
    /// words before laying it out).
    pub fn width(&self, ui: &Ui) -> f32 {
        if self.label == Label::IconOnly {
            return ui.spacing().interact_size.y.clamp(metrics::ICON, metrics::ICON_BUTTON);
        }
        let galley = self.text.clone().into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Button,
        );
        let pad = 2.0 * ui.spacing().button_padding.x;
        match self.label {
            Label::WordsOnly => pad + galley.size().x,
            _ => {
                let gap = self.gap.unwrap_or(ui.spacing().icon_spacing);
                pad + metrics::ICON - ICON_INSET + gap + galley.size().x
            }
        }
    }

    fn button(&self, atoms: egui::Atoms<'static>) -> egui::Button<'static> {
        let mut button = egui::Button::new(atoms);
        if self.small {
            button = button.small();
        }
        if let Some(frame) = self.frame {
            button = button.frame_when_inactive(frame);
        }
        if let Some(selected) = self.selected {
            button = button.selected(selected);
        }
        if let Some(h) = self.min_height {
            button = button.min_size(Vec2::new(0.0, h));
        }
        if let Some(fill) = self.fill {
            button = button.fill(fill);
        }
        button.sense(self.sense)
    }

    /// Shown as a menu's button: the menu opens under it when clicked (as
    /// `ui.menu_button`; inside a menu, a submenu with a chevron at the
    /// right).
    pub fn menu<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> egui::InnerResponse<Option<R>> {
        use egui::containers::menu::{MenuConfig, SubMenu};
        if egui::containers::menu::is_in_menu(ui) {
            let response = submenu_row(ui, self);
            let inner = SubMenu::default().show(ui, &response, add_contents);
            return egui::InnerResponse::new(inner.map(|i| i.inner), response);
        }
        let response = ui.add(self);
        // (The menu's own config, as egui's `MenuButton` gives it: not a
        // bar's, whose rows would not open submenus.)
        let found = MenuConfig::find(ui);
        let config = MenuConfig::new().close_behavior(found.close_behavior).style(found.style);
        let inner = egui::Popup::menu(&response)
            .close_behavior(config.close_behavior)
            .style(config.style.clone())
            .info(
                egui::UiStackInfo::new(egui::UiKind::Menu)
                    .with_tag_value(MenuConfig::MENU_CONFIG_TAG, config),
            )
            .show(add_contents);
        egui::InnerResponse::new(inner.map(|i| i.inner), response)
    }
}

impl egui::Widget for IconButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let text = self.text.text().to_string();
        match self.label {
            Label::IconOnly => {
                // (22 px, or as high as the rows it stands in where they
                // are lower: a tree's arrow.)
                let side = ui.spacing().interact_size.y.clamp(metrics::ICON, metrics::ICON_BUTTON);
                let size = Vec2::splat(side);
                let (rect, response) = ui.allocate_exact_size(size, self.sense);
                let enabled = ui.is_enabled();
                response.widget_info(|| match self.selected {
                    Some(on) => WidgetInfo::selected(WidgetType::Button, enabled, on, &text),
                    None => WidgetInfo::labeled(WidgetType::Button, enabled, &text),
                });
                if ui.is_rect_visible(rect) {
                    let tokens = ui.tokens();
                    let on = self.selected == Some(true);
                    if crate::controls::is_read_only(ui) {
                        // (Quiet, as a read-only form's controls: nothing
                        // lit under the pointer, never the accent; on, a
                        // `raised` ground and `text-2`, off `text-3`.)
                        let (fill, color) = match on {
                            true => (tokens.raised, tokens.text_2),
                            false => (Color32::TRANSPARENT, tokens.text_3),
                        };
                        let painter = ui.painter();
                        painter.rect_filled(rect, metrics::RADIUS, fill);
                        let square =
                            Rect::from_center_size(rect.center(), Vec2::splat(metrics::ICON));
                        self.icon.paint(painter, square, color);
                    } else if let Some(fill) = self.fill {
                        // (A main action's fill, kept with its icon alone:
                        // the icon in `on-fill` on it.)
                        let painter = ui.painter();
                        painter.rect_filled(rect, metrics::RADIUS, fill);
                        let square =
                            Rect::from_center_size(rect.center(), Vec2::splat(metrics::ICON));
                        self.icon.paint(painter, square, tokens.on_fill);
                    } else {
                        let state = ButtonState::of(&response);
                        paint_icon_button(ui.painter(), &tokens, rect, self.icon, state, on);
                    }
                }
                response
            }
            Label::WordsOnly => {
                let mut atoms = egui::Atoms::default();
                atoms.push_right(self.text.clone());
                ui.add(self.button(atoms))
            }
            Label::Both => {
                // (The mock's `.btn .ic { margin-left: -2px }`: the icon
                // 2 px into the padding, 6 px from the button's edge, the
                // words 6 px after it.)
                let id = ui.next_auto_id().with("mg-uikit-icon");
                let room = Vec2::new(metrics::ICON - ICON_INSET, metrics::ICON);
                let atoms: egui::Atoms<'static> =
                    (egui::Atom::custom(id, room), self.text.clone()).into_atoms();
                let spacing = ui.spacing().icon_spacing;
                if let Some(gap) = self.gap {
                    ui.spacing_mut().icon_spacing = gap;
                }
                let atoms_ui = self.button(atoms).atom_ui(ui);
                ui.spacing_mut().icon_spacing = spacing;
                if let Some(rect) = atoms_ui.rect(id) {
                    let r = &atoms_ui.response;
                    let visuals = ui.style().interact_selectable(r, self.selected == Some(true));
                    let square = Rect::from_center_size(
                        rect.center() - Vec2::new(ICON_INSET / 2.0, 0.0),
                        Vec2::splat(metrics::ICON),
                    );
                    self.icon.paint(ui.painter(), square, visuals.text_color());
                }
                atoms_ui.response
            }
        }
    }
}

use egui::IntoAtoms as _;

/// A submenu's row: the button (its icon and words, as asked) with a
/// chevron at its right, lit while its submenu is open.
fn submenu_row(ui: &mut Ui, button: IconButton) -> Response {
    use egui::containers::menu::{MenuState, SubMenu};
    let my_id = ui.next_auto_id();
    let open = MenuState::from_ui(ui, |state, _| {
        state.open_item == Some(SubMenu::id_from_widget_id(my_id))
    });
    let inactive = ui.style().visuals.widgets.inactive;
    if open {
        ui.style_mut().visuals.widgets.inactive = ui.style().visuals.widgets.open;
    }
    let response = submenu_button(ui, button);
    ui.style_mut().visuals.widgets.inactive = inactive;
    response
}

/// A menu's row that opens a submenu: `button`'s words (and icon, unless
/// [`Label::WordsOnly`]) and a chevron at the right, for
/// `egui::containers::menu::SubMenu::show` (the crate's in place of
/// egui's `SubMenuButton`, whose arrow is a font's glyph). Its
/// accessible name is the words alone.
pub fn submenu_button(ui: &mut Ui, button: IconButton) -> Response {
    let chevron = ui.next_auto_id().with("mg-uikit-chevron");
    let icon = ui.next_auto_id().with("mg-uikit-icon");
    let side = Vec2::splat(metrics::ICON);
    let mut atoms = egui::Atoms::default();
    let with_icon = button.label != Label::WordsOnly;
    if with_icon {
        atoms.push_right(egui::Atom::custom(icon, side));
    }
    atoms.push_right(button.text.clone());
    atoms.push_right(egui::Atom::grow());
    atoms.push_right(egui::Atom::custom(chevron, Vec2::splat(metrics::ICON_SMALL)));
    let atoms_ui = button.button(atoms).atom_ui(ui);
    let r = &atoms_ui.response;
    let color = ui.style().interact(r).text_color();
    if with_icon && let Some(rect) = atoms_ui.rect(icon) {
        button.icon.paint(ui.painter(), rect, color);
    }
    if let Some(rect) = atoms_ui.rect(chevron) {
        crate::controls::paint_chevron(ui.painter(), rect.center(), 0.0, color);
    }
    atoms_ui.response
}

/// `ui.menu_button(text, …)` with the crate's chevron in place of egui's
/// glyph when it opens a submenu (words alone, as egui's).
pub fn menu_button<R>(
    ui: &mut Ui,
    text: impl Into<egui::WidgetText>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    IconButton::new(Icon::ChevronRight, text).label(Label::WordsOnly).menu(ui, add_contents)
}

/// An icon (16 px, in `color`, or the text color) and words after it, as a
/// label (not interactive): a warning's or a note's mark. Its accessible
/// name is the words.
pub fn icon_label(
    ui: &mut Ui,
    icon: Icon,
    color: Option<Color32>,
    text: impl Into<egui::WidgetText>,
) -> Response {
    // (Top-aligned: words that wrap to a second line keep the icon at the
    // first.)
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = ui.spacing().icon_spacing;
        let text = egui::Label::new(text).wrap();
        let line = ui.text_style_height(&egui::TextStyle::Body);
        let (rect, _) = ui
            .allocate_exact_size(Vec2::new(metrics::ICON, line.max(metrics::ICON)), Sense::hover());
        if ui.is_rect_visible(rect) {
            let color = color.unwrap_or_else(|| ui.visuals().text_color());
            let square = Rect::from_center_size(rect.center(), Vec2::splat(metrics::ICON));
            icon.paint(ui.painter(), square, color);
        }
        ui.add(text)
    })
    .inner
}

/// Names `ui` for a screen reader (and tests): a group, `name`, of the
/// widgets it holds (a row of icons that name kinds the window shows
/// elsewhere too, say). Its widgets are found within it by their own
/// names.
pub fn name_group(ui: &Ui, name: &str) {
    ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
        node.set_role(egui::accesskit::Role::Group);
        node.set_label(name);
    });
}

/// The id under which the icons painted are kept while they are recorded.
fn recording_id() -> egui::Id {
    egui::Id::new("mg-uikit-icons-drawn")
}

/// The icons painted in a pass, while recorded ([`record_icons`]).
#[derive(Default)]
struct Drawn {
    pass: u64,
    icons: Vec<(Icon, Rect)>,
}

type Recording = std::sync::Arc<std::sync::Mutex<Drawn>>;

/// Has `ctx` keep the icons it paints, for [`icons_drawn`] (tests: which
/// icon a widget drew, and where).
pub fn record_icons(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(recording_id(), Recording::default()));
}

/// The icons the last pass of `ctx` painted, and where, while recorded
/// ([`record_icons`]; else none).
pub fn icons_drawn(ctx: &egui::Context) -> Vec<(Icon, Rect)> {
    let recording = ctx.data(|d| d.get_temp::<Recording>(recording_id()));
    recording.map(|r| r.lock().map(|d| d.icons.clone()).unwrap_or_default()).unwrap_or_default()
}

/// Keeps `icon`, painted in `rect`, where `ctx` records them.
fn record(ctx: &egui::Context, icon: Icon, rect: Rect) {
    let Some(recording) = ctx.data(|d| d.get_temp::<Recording>(recording_id())) else {
        return;
    };
    let pass = ctx.cumulative_pass_nr();
    if let Ok(mut drawn) = recording.lock() {
        if drawn.pass != pass {
            *drawn = Drawn { pass, icons: Vec::new() };
        }
        drawn.icons.push((icon, rect));
    }
}

/// A button with an icon and words that toggles `on` when clicked
/// (`accent-fill` while on, egui's selected button). Changed when clicked.
pub fn icon_text_toggle(ui: &mut Ui, on: &mut bool, icon: Icon, text: &str) -> Response {
    let mut response = icon_text_button_ui(ui, icon, text, Some(*on));
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_set_has_194_icons_named_as_in_the_file() {
        assert_eq!(Icon::ALL.len(), 194);
        let json = include_str!("../icons/icons.json");
        for icon in Icon::ALL {
            assert_eq!(Icon::from_name(icon.name()), Some(icon));
            assert!(json.contains(&format!("\"{}\":", icon.name())), "{}", icon.name());
        }
        assert_eq!(Icon::from_name("no-such-icon"), None);
        assert_eq!(Icon::ChevronDown.name(), "chevron-down");
        assert_eq!(Icon::from_name("eye-off"), Some(Icon::EyeOff));
    }

    /// Every icon has strokes, and they stay inside the 24 box with their
    /// width (0.75 at each side).
    #[test]
    fn every_icon_has_strokes_inside_its_box() {
        let half = metrics::ICON_STROKE / 2.0;
        for icon in Icon::ALL {
            let polys: Vec<_> = icon.polylines().collect();
            assert!(!polys.is_empty(), "{} has no strokes", icon.name());
            for (points, _) in polys {
                assert!(!points.is_empty(), "{}: an empty stroke", icon.name());
                for &[x, y] in points {
                    let inside = |v: f32| (half - 0.001..=24.0 - half + 0.001).contains(&v);
                    assert!(inside(x) && inside(y), "{}: ({x}, {y}) outside", icon.name());
                }
            }
        }
    }

    /// Painted at 16 px, a stroke keeps the points a tenth of a pixel
    /// needs (a circle's hundred and twenty are a few dozen), its ends,
    /// and each original point within the tolerance of what is kept; at
    /// 32 px and 200% more of them.
    #[test]
    fn strokes_are_simplified_for_the_size_painted() {
        let small = simplify_level(16.0 / 24.0);
        let large = simplify_level(32.0 / 24.0 * 2.0);
        assert!(small > large && large > 0, "{small} {large}");
        let count =
            |level| -> usize { simplified(Icon::Info, level).iter().map(|s| s.points.len()).sum() };
        let all: usize = Icon::Info.polylines().map(|(p, _)| p.len()).sum();
        assert!(count(small) * 3 < all, "{} of {all}", count(small));
        assert!(count(large) >= count(small));
        let near = |p: [f32; 2], line: &[[f32; 2]], closed: bool| {
            let n = line.len();
            let segments = if closed { n } else { n - 1 };
            (0..segments).any(|i| {
                let (a, b) = (line[i], line[(i + 1) % n]);
                let (ab, ap) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
                let len2 = ab[0] * ab[0] + ab[1] * ab[1];
                let t = if len2 > 0.0 {
                    ((ap[0] * ab[0] + ap[1] * ab[1]) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (dx, dy) = (ap[0] - t * ab[0], ap[1] - t * ab[1]);
                (dx * dx + dy * dy).sqrt() <= f32::from(small) / 64.0 + 1e-3
            })
        };
        for icon in Icon::ALL {
            for (s, (orig, closed)) in simplified(icon, small).iter().zip(icon.polylines()) {
                if !closed {
                    assert_eq!(s.points.first(), orig.first(), "{}", icon.name());
                    assert_eq!(s.points.last(), orig.last(), "{}", icon.name());
                }
                for p in orig {
                    assert!(
                        s.points.len() < 2 || near(*p, &s.points, closed),
                        "{}: {p:?}",
                        icon.name()
                    );
                }
            }
        }
    }

    /// The build read the file as the tests' copy of the parser does.
    #[test]
    fn the_build_s_strokes_are_the_parser_s() {
        let json = include_str!("../icons/icons.json");
        for icon in [Icon::Ability, Icon::Info, Icon::Settings, Icon::Grid] {
            let start = json.find(&format!("\"{}\": \"", icon.name())).unwrap();
            let markup = &json[start + icon.name().len() + 5..];
            let markup = &markup[..markup.find("\",\n").or_else(|| markup.find("\"\n")).unwrap()];
            let markup = markup.replace("\\\"", "\"");
            let parsed = crate::svg::parse_icon(&markup).unwrap();
            let built: Vec<_> = icon.polylines().collect();
            assert_eq!(parsed.len(), built.len(), "{}", icon.name());
            for (p, (points, closed)) in parsed.iter().zip(built) {
                assert_eq!(p.closed, closed);
                assert_eq!(p.points.len(), points.len());
                for (a, b) in p.points.iter().zip(points) {
                    assert!((a[0] - b[0]).abs() < 0.001 && (a[1] - b[1]).abs() < 0.001);
                }
            }
        }
    }
}
