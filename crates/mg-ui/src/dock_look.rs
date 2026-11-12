//! The document tabs' look over egui_dock's tabs (M3's T9): each tab's
//! kind icon, modified dot and close button, the lines between the tabs
//! and along the top of each pane's tab in front (`mg_uikit::dock`, its
//! `dock` feature, so that Moonglow Viewer takes them from the same
//! place), the close buttons' clicks, the Split Editor button at the right
//! end of the focused group's tab bar, and a tab bar of more tabs than it
//! holds kept scrolled to its tab in front. Each tab's title is laid out
//! by `tabs::prepare` before the dock is drawn.

use std::collections::{HashMap, HashSet};

use mg_uikit::dock::DocTabMark;
use mg_uikit::icons::ButtonState;

use crate::{Action, Moonglow, Tab, tabs};

/// The Split Editor button's tip.
pub(crate) const SPLIT_TIP: &str = "Move the tab in front to an editor group of its own";

impl Moonglow {
    /// After the dock is drawn: each tab's parts painted, its close button
    /// where it shows (on the tab in front of its pane, always, and on the
    /// tab under the pointer), named for its tab ("Close map_m1q1a") and
    /// pressed to close it as its menu's Close does (asking first where
    /// it has unsaved changes), and the Split Editor button.
    pub(crate) fn doc_tab_parts(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let pointer = ctx.pointer_hover_pos();
        let fronts: HashSet<Tab> = (self.dock.iter_leaves())
            .filter_map(|(_, l)| l.tabs.get(l.active.0).cloned())
            .collect();
        let mut marks = Vec::new();
        let mut close = None;
        let mut rects = HashMap::new();
        for (tab, rect, layer, shown) in self.tab_buttons.clone() {
            rects.insert(tab.clone(), (rect, shown));
            let Some(look) = self.doc_tabs.get(&tab) else { continue };
            // (The tab being dragged has its parts where it is dragged.)
            if self.dragged_tab.as_ref() == Some(&tab) {
                continue;
            }
            let active = fronts.contains(&tab);
            // (Under the pointer where nothing lies over it: a window over
            // the main surface's tabs hides their close buttons.)
            let hovered = pointer.is_some_and(|p| {
                shown.contains(p) && ctx.layer_id_at(p).is_none_or(|l| l == layer)
            });
            let parts = mg_uikit::shell::doc_tab_parts(rect, look.title.width, look.modified);
            let mut state = None;
            if tab.closeable() && (active || hovered) && shown.contains_rect(parts.close) {
                let id = egui::Id::new(("doc-tab-close", &tab));
                let builder = egui::UiBuilder::new().layer_id(layer).max_rect(parts.close);
                let r = ui
                    .scope_builder(builder, |ui| {
                        ui.set_clip_rect(shown);
                        // (Registered after its tab: it takes the clicks.)
                        ui.interact(parts.close, id, egui::Sense::click())
                    })
                    .inner;
                let name = format!("Close {}", look.name);
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &name));
                state = Some(if r.is_pointer_button_down_on() {
                    ButtonState::Pressed
                } else if r.hovered() || r.has_focus() {
                    ButtonState::Hovered
                } else {
                    ButtonState::Rest
                });
                let r = r.on_hover_text("Close this tab");
                if r.clicked() || r.middle_clicked() {
                    close = Some(tab.clone());
                }
            }
            marks.push(DocTabMark {
                tab,
                rect,
                layer,
                shown,
                icon: look.icon,
                title_width: look.title.width,
                modified: look.modified,
                active,
                close: state,
            });
        }
        // (The focused group's front tab has the accent line, the one
        // shown as the window opens too, before egui_dock has a focus.)
        let focused = self.dock.focused_leaf().or_else(|| self.focused_group());
        mg_uikit::dock::doc_tab_marks(&ctx, &self.dock, focused, &marks);
        if let Some(tab) = close {
            self.actions.push(Action::CloseTabs(vec![tab]));
        }
        self.tab_rects = rects;
        self.split_button(ui);
    }

    /// Split Editor at the right end of the focused group's tab bar (the
    /// mock's `.doctabs .tools`), in the room egui_dock leaves there
    /// ([`tabs::SPLIT_ROOM`]); none where the group has one tab (the boards
    /// draw none beside a group's only tab, nor on the other groups' bars).
    fn split_button(&mut self, ui: &mut egui::Ui) {
        if !self.can_split() {
            return;
        }
        let Some(path) = self.focused_group() else { return };
        let Ok(leaf) = self.dock.leaf(path) else { return };
        let bar =
            egui::Rect::from_x_y_ranges(leaf.rect.x_range(), leaf.rect.top()..=leaf.viewport.top());
        // (Not in a bar narrower than its room: the least window's editor
        // area, both docks open, is a sliver until T14's yields.)
        if !bar.is_positive() || bar.width() < tabs::SPLIT_ROOM {
            return;
        }
        let layer = (self.tab_buttons.iter())
            .find(|(t, ..)| leaf.tabs.contains(t))
            .map_or(ui.layer_id(), |b| b.2);
        let side = mg_uikit::metrics::ICON_BUTTON.min(bar.height());
        let at = egui::Rect::from_center_size(
            egui::pos2(bar.right() - tabs::SPLIT_ROOM / 2.0, bar.center().y),
            egui::vec2(side, side),
        );
        let button =
            mg_uikit::IconButton::new(mg_uikit::Icon::Split, "Split Editor").icon_only(true);
        let pressed = ui
            .scope_builder(egui::UiBuilder::new().layer_id(layer).max_rect(at), |ui| {
                ui.set_clip_rect(at.expand(1.0));
                ui.put(at, button).on_hover_text(SPLIT_TIP).clicked()
            })
            .inner;
        if pressed {
            self.actions.push(Action::SplitEditor);
        }
    }

    /// Before the dock is drawn: a pane whose tab in front is another than
    /// at the last frame (one clicked, opened, reached by the keys) has its
    /// tab bar scrolled to show it whole, where its bar has more tabs than
    /// it holds. (egui_dock scrolls a bar by the wheel alone.) A tab not
    /// drawn yet (one just opened) is shown once it has been.
    pub(crate) fn reveal_fronts(&mut self) {
        let before = std::mem::take(&mut self.fronts_before);
        let mut now = HashSet::new();
        for (surface, node) in self.dock.iter_all_nodes_mut() {
            let egui_dock::Node::Leaf(leaf) = node else { continue };
            let Some(front) = leaf.tabs.get(leaf.active.0) else { continue };
            let Some((rect, _)) = self.tab_rects.get(front) else { continue };
            now.insert(front.clone());
            if before.contains(front) {
                continue;
            }
            // (A window's bar has egui_dock's arrow at its left.)
            let main = surface.surface.is_main();
            let left = leaf.rect.left() + if main { 0.0 } else { 24.0 };
            let right = left + tabs::tab_room(main, leaf.rect.width());
            if rect.right() > right {
                leaf.scroll -= rect.right() - right;
            } else if rect.left() < left {
                leaf.scroll += left - rect.left();
            }
        }
        self.fronts_before = now;
    }
}
