//! A panel's pages as a rail of icons at its left edge (the mock's
//! `.rail`, Blender's Properties): 32 px of `bar` with a 1 px `line` at its
//! right; from the top, 4 px in and 2 px apart, 24 px page buttons
//! (frameless in `text-2` at rest, `raised` and `text` under the pointer,
//! `accent-fill` and `on-fill` for the page shown), each with its page's
//! icon. The Properties panel's pages (M4's U6); U9's page list for forms
//! is its wider sibling.
//!
//! The rail is one group named "Pages" (or its own name); each page a
//! button named by its page's name, said selected or not, its tooltip the
//! name. Where the rail is shorter than its pages (a section dragged
//! low), it scrolls under the wheel, and the page shown is kept in sight.
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! use mg_uikit::pages::PageRail;
//! use mg_uikit::Icon;
//! let mut page = 0;
//! let changed = PageRail::new("properties-pages")
//!     .page(Icon::Info, "Basic")
//!     .page(Icon::Inventory, "Inventory")
//!     .page(Icon::Script, "Scripts")
//!     .show(ui, &mut page)
//!     .changed;
//! # let _ = changed;
//! # });
//! ```

use egui::{Rect, Response, Sense, Stroke, Ui, Vec2, WidgetInfo, WidgetType, pos2, vec2};

use crate::icons::{ButtonState, Icon};
use crate::metrics;
use crate::theme::TokensExt as _;

/// The rail's accessible name where it is not given one.
pub const PAGES: &str = "Pages";
/// The rail's width, its line included (`.rail`: `width: 32px`).
pub const PAGE_RAIL: f32 = 32.0;
/// A page button's side (`.rail .tool`: 24 by 24).
pub const PAGE_BUTTON: f32 = 24.0;
/// The room above the first page (`.rail`: `padding: 4px 0`).
pub const RAIL_PADDING: f32 = 4.0;
/// The gap between the pages (`.rail`: `gap: 2px`).
pub const RAIL_GAP: f32 = 2.0;

/// What a [`PageRail`] did.
#[derive(Debug)]
pub struct RailOutput {
    /// The rail's response (its whole strip).
    pub response: Response,
    /// Whether a page was clicked and the page changed.
    pub changed: bool,
    /// Each page's response, in order.
    pub pages: Vec<Response>,
}

/// A panel's pages as icons at its left edge (module comment).
#[derive(Clone, Debug)]
#[must_use = "show it with .show(ui, &mut page)"]
pub struct PageRail {
    id: egui::Id,
    name: String,
    pages: Vec<(Icon, String)>,
}

impl PageRail {
    /// A rail named "Pages", its scroll kept by `id_salt`.
    pub fn new(id_salt: impl egui::AsId) -> Self {
        PageRail { id: egui::Id::new(id_salt), name: PAGES.to_string(), pages: Vec::new() }
    }

    /// Named `name` for a screen reader (and tests) in place of "Pages".
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// A page, after those given: its icon and its name.
    pub fn page(mut self, icon: Icon, name: impl Into<String>) -> Self {
        self.pages.push((icon, name.into()));
        self
    }

    /// The height its pages take, its padding included.
    pub fn height(&self) -> f32 {
        let n = self.pages.len() as f32;
        2.0 * RAIL_PADDING + n * PAGE_BUTTON + (n - 1.0).max(0.0) * RAIL_GAP
    }

    /// Draws the rail down the height of `ui` at its left ([`PAGE_RAIL`]
    /// wide, its line included); `page` is the index of the page shown,
    /// set to the one clicked.
    pub fn show(self, ui: &mut Ui, page: &mut usize) -> RailOutput {
        let t = ui.tokens();
        let top = ui.max_rect().top();
        let left = ui.cursor().min.x;
        let rect =
            Rect::from_min_max(pos2(left, top), pos2(left + PAGE_RAIL, ui.max_rect().bottom()));
        let response = ui.interact(rect, self.id.with("mg-uikit-rail"), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, t.bar);
        ui.painter().vline(rect.right() - 0.5, rect.y_range(), Stroke::new(1.0, t.line));
        let inner = Rect::from_min_max(rect.min, pos2(rect.right() - 1.0, rect.bottom()));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        child.set_clip_rect(inner.intersect(ui.clip_rect()));
        let mut changed = false;
        let mut pages = Vec::new();
        let shown = *page;
        let name = self.name.clone();
        // (Where they do not all fit, they scroll; the page shown kept in
        // sight as it changes.)
        let kept = child.ctx().data(|d| d.get_temp::<usize>(self.id.with("shown")));
        let mut in_sight = false;
        egui::ScrollArea::vertical()
            .id_salt(self.id.with("scroll"))
            .auto_shrink([false, false])
            .min_scrolled_height(0.0)
            .max_height(inner.height())
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(&mut child, |ui| {
                crate::icons::name_group(ui, &name);
                // (The padding as room taken, which the scroll counts.)
                ui.spacing_mut().item_spacing.y = RAIL_GAP;
                ui.allocate_exact_size(vec2(PAGE_BUTTON, RAIL_PADDING - RAIL_GAP), Sense::hover());
                for (i, (icon, name)) in self.pages.iter().enumerate() {
                    let r = page_ui(ui, *icon, name, i == shown);
                    if r.clicked() && i != *page {
                        *page = i;
                        changed = true;
                    }
                    if i == shown {
                        in_sight = ui.clip_rect().contains_rect(r.rect);
                        if kept != Some(shown) && !in_sight {
                            r.scroll_to_me_animation(None, egui::style::ScrollAnimation::none());
                        }
                    }
                    pages.push(r);
                }
                ui.allocate_exact_size(vec2(PAGE_BUTTON, RAIL_PADDING - RAIL_GAP), Sense::hover());
            });
        // (Noted once it is in sight: until then each frame brings it in.)
        if in_sight {
            child.ctx().data_mut(|d| d.insert_temp(self.id.with("shown"), shown));
        }
        ui.advance_cursor_after_rect(rect);
        RailOutput { response, changed, pages }
    }
}

/// A page's button: 24 px square, painted in its state's tokens.
fn page_ui(ui: &mut Ui, icon: Icon, name: &str, on: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(PAGE_BUTTON), Sense::click());
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, on, name));
    if ui.is_rect_visible(rect) {
        let t = ui.tokens();
        let state = if enabled { ButtonState::of(&response) } else { ButtonState::Rest };
        let (fill, color) = match (on, state) {
            (true, ButtonState::Rest) => (t.accent_fill, t.on_fill),
            (true, _) => (t.accent_fill_hover, t.on_fill),
            (false, ButtonState::Rest) => (egui::Color32::TRANSPARENT, t.text_2),
            (false, _) => (t.raised, t.text),
        };
        let ring = if response.has_focus() { Stroke::new(1.0, t.accent) } else { Stroke::NONE };
        let painter = ui.painter();
        painter.rect(rect, metrics::RADIUS, fill, ring, egui::StrokeKind::Inside);
        let square = Rect::from_center_size(rect.center(), vec2(metrics::ICON, metrics::ICON));
        icon.paint(painter, square, color);
    }
    response.on_hover_text(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};

    #[test]
    fn the_rail_s_pages_are_named_buttons_at_its_measures() {
        let mut page = 1;
        let mut clicked = false;
        let mut h = Harness::builder().with_size(vec2(200.0, 300.0)).build_ui(|ui| {
            crate::apply(ui.ctx(), crate::Theme::Dark);
            let out = PageRail::new("rail")
                .page(Icon::Info, "Basic")
                .page(Icon::Inventory, "Inventory")
                .page(Icon::Lock, "Lock")
                .show(ui, &mut page);
            clicked |= out.changed;
        });
        h.run();
        let _ = h.get_by_label(PAGES);
        let basic = h.get_by_label("Basic").rect();
        let inventory = h.get_by_label("Inventory").rect();
        assert_eq!(basic.size(), Vec2::splat(PAGE_BUTTON), "24 by 24");
        assert_eq!(inventory.top() - basic.bottom(), RAIL_GAP, "2 apart");
        // (The harness's panel keeps 8 round its ui.)
        assert_eq!(basic.top() - 8.0, RAIL_PADDING, "4 under the top");
        assert_eq!(basic.center().x - 8.0, (PAGE_RAIL - 1.0) / 2.0, "centered in the rail");
        assert_eq!(
            h.get_by_label("Inventory").accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::True)
        );
        assert_eq!(
            h.get_by_label("Basic").accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::False)
        );
        h.get_by_label("Lock").click();
        h.run();
        drop(h);
        assert!(clicked, "a click changes the page");
        assert_eq!(page, 2);
    }

    #[test]
    fn a_rail_shorter_than_its_pages_scrolls_to_the_page_shown() {
        let mut page = 4;
        // (The rail in a room 50 high at 100 down: two pages fit.)
        let room = egui::Rect::from_min_size(pos2(20.0, 100.0), vec2(100.0, 50.0));
        let mut h = Harness::builder().with_size(vec2(200.0, 300.0)).build_ui(|ui| {
            crate::apply(ui.ctx(), crate::Theme::Dark);
            let mut part = ui.new_child(egui::UiBuilder::new().max_rect(room));
            part.set_clip_rect(room);
            let _ = PageRail::new("short")
                .page(Icon::Info, "Basic")
                .page(Icon::Inventory, "Inventory")
                .page(Icon::Lock, "Lock")
                .page(Icon::Trap, "Trap")
                .page(Icon::Script, "Scripts")
                .show(&mut part, &mut page);
        });
        h.run();
        let scripts = h.get_by_label("Scripts").rect();
        assert!(room.contains_rect(scripts), "in sight: {scripts:?}");
        let basic = h.get_by_label("Basic").rect();
        assert!(!room.intersects(basic), "the first scrolled out: {basic:?}");
    }
}
