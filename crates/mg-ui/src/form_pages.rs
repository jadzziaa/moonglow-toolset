//! A form window's pages (M4's U9; the Creature board): the page list at
//! its left (`mg_uikit::pages::PageList`, or in a window too narrow for it
//! the rail of icons, `PageRail`) and the room beside it, where the pages
//! stand on one scroll (`mg_uikit::pages::PageScroll`). The blueprint
//! windows, Module Properties and Area Properties.

use egui::{Rect, Ui, UiBuilder, pos2, vec2};
use mg_uikit::pages::{PAGE_LIST, PAGE_RAIL, PageList, PageRail, PageScroll};

/// The width of a window's room under which its pages are listed as a
/// rail of icons rather than words: under it the list's 150 points (and
/// a page's margins) would leave the form less than the width it is laid
/// out narrow under (`blueprint::NARROW_FORM`), where the rail's 32 leave
/// it its window's look.
pub(crate) const LIST_UNDER: f32 = PAGE_LIST + MARGINS + crate::blueprint::NARROW_FORM;

/// The page's margins left and right (`mg_uikit::pages::PAGE_MARGIN`).
const MARGINS: f32 =
    (mg_uikit::pages::PAGE_MARGIN.left + mg_uikit::pages::PAGE_MARGIN.right) as f32;

/// Whether the pages in a room `body` wide (beside the list) are laid out
/// narrow: judged on the page's own width inside its margins, as
/// [`LIST_UNDER`] is (`blueprint::NARROW_FORM`).
pub(crate) fn narrow(body: f32) -> bool {
    body - MARGINS < crate::blueprint::NARROW_FORM
}

/// What [`list`] gave: the room for the pages beside the list, and
/// whether a page was chosen there (to be brought to the top, though it
/// is already the page at the top).
pub(crate) struct Listed {
    pub body: Ui,
    pub chosen: bool,
}

/// The pages' list at `ui`'s left, named `name` (for a screen reader and
/// the tests: "Pages"), the page at the top `top` (set to the one
/// chosen), marking the pages in view as the scroll kept by `id` last
/// showed them; the rest of `ui` for the pages.
pub(crate) fn list(
    ui: &mut Ui,
    id: egui::Id,
    name: &str,
    pages: &[&str],
    top: &mut usize,
) -> Listed {
    let room = ui.available_rect_before_wrap();
    let rail = room.width() < LIST_UNDER;
    let width = if rail { PAGE_RAIL } else { PAGE_LIST };
    let left = Rect::from_min_size(room.min, vec2(width, room.height()));
    let mut list_ui = ui.new_child(UiBuilder::new().id_salt("form-pages-list").max_rect(left));
    let chosen = match rail {
        true => {
            let mut rail = PageRail::new(id.with("rail")).name(name);
            for p in pages {
                rail = rail.page(crate::properties_panel::page_icon(p), *p);
            }
            rail.show(&mut list_ui, top).changed
        }
        false => {
            let seen = PageScroll::seen(ui.ctx(), id).map(|s| s.pages_in_view).unwrap_or_default();
            let mut list = PageList::new(id.with("list")).name(name).pages_in_view(seen);
            for p in pages {
                list = list.page(crate::properties_panel::page_icon(p), *p);
            }
            list.show(&mut list_ui, top).changed
        }
    };
    let right = Rect::from_min_max(pos2(left.right(), room.top()), room.max);
    let body = ui.new_child(
        UiBuilder::new()
            .id_salt("form-pages-body")
            .max_rect(right)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    ui.advance_cursor_after_rect(room);
    Listed { body, chosen }
}

/// The least width of a column of pages side by side, its margins
/// included (M4's U10; the WideCreature board's five columns of 480 in
/// its room beside the Right Dock, the coordinator's ruling).
pub(crate) const PAGE_COLUMN: f32 = 480.0;

/// How many columns of pages a room `body` wide (beside the list) stands
/// side by side: as many of at least [`PAGE_COLUMN`] as fit with the lines
/// between them, where four or more do (at 3440 by 1440 every page of a
/// creature in view at once); else one, a page across the room, its
/// sections in columns of their own (`mg_uikit::form::FormColumns`). (From
/// four: a docked editor up to about 2300 wide keeps a placeable's or a
/// door's model beside its Basic page's fields, which a column has no
/// room for.)
pub(crate) fn page_columns(body: f32) -> usize {
    let line = mg_uikit::pages::PAGE_COLUMN_LINE;
    let fit = ((body + line) / (PAGE_COLUMN + line)).floor() as usize;
    if fit >= 4 { fit } else { 1 }
}

/// The scroll of `pages` kept by `id`, each with its icon and name, in
/// `columns` columns ([`page_columns`]), no more than there are pages; it
/// brings the page asked for to the top where `bring`.
pub(crate) fn scroll(
    id: egui::Id,
    pages: &[&str],
    sideways: bool,
    bring: bool,
    columns: usize,
) -> PageScroll {
    // (No more columns than pages: a waypoint's five in five, not six with
    // one empty; and fewer than four, one page across, as `page_columns`.)
    let columns = match columns.min(pages.len()) {
        n if n >= 4 => n,
        _ => 1,
    };
    let mut scroll =
        PageScroll::new(id).sideways(sideways && columns == 1).bring(bring).columns(columns);
    for p in pages {
        scroll = scroll.page(crate::properties_panel::page_icon(p), *p);
    }
    scroll
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The room beside the list a window `width` wide leaves its pages.
    fn body_at(width: f32) -> f32 {
        let mut body = 0.0;
        let mut h = egui_kittest::Harness::builder()
            .with_size(egui::vec2(width + 16.0, 400.0))
            .build_ui(|ui| {
                let mut top = 0;
                let listed = list(ui, egui::Id::new("t"), "Pages", &["Basic", "Scripts"], &mut top);
                body = listed.body.available_width();
            });
        h.run();
        drop(h);
        body
    }

    /// From [`LIST_UNDER`] the list of words, under it the rail; at
    /// [`LIST_UNDER`] the page beside the list, inside its margins, is
    /// just wide enough not to be laid out narrow.
    #[test]
    fn the_list_gives_way_to_the_rail_where_the_form_would_be_narrow() {
        // (The list's 150, the page's margins 16 and 12, the narrow form's
        // 400: 578.)
        let at = 150.0 + 28.0 + crate::blueprint::NARROW_FORM;
        assert_eq!(body_at(at), at - PAGE_LIST, "the list from 578");
        assert!(!narrow(body_at(at)), "its pages laid out as a window's");
        assert!(narrow(body_at(at) - 1.0), "a point less would be narrow");
        assert_eq!(body_at(at - 1.0), at - 1.0 - PAGE_RAIL, "the rail under it");
        assert!(!narrow(body_at(at - 1.0)), "beside the rail, not narrow either");
    }

    /// A page is laid out narrow by its own width, inside its margins: a
    /// room of 410 beside the list leaves it 382.
    #[test]
    fn narrow_is_judged_inside_the_page_s_margins() {
        assert!(narrow(410.0), "382 inside");
        assert!(!narrow(crate::blueprint::NARROW_FORM + 28.0), "400 inside");
        assert!(narrow(crate::blueprint::NARROW_FORM + 27.0));
    }

    /// Pages stand side by side where four columns of at least 480 fit:
    /// the WideCreature board's room beside its Right Dock, about 2400,
    /// five; today's room at 3440 without one, about 2960, six; under
    /// four, one page across.
    #[test]
    fn pages_stand_side_by_side_where_four_columns_fit() {
        assert_eq!(PAGE_COLUMN, 480.0);
        assert_eq!(page_columns(2408.0), 5, "the board's five");
        assert_eq!(page_columns(2960.0), 6);
        assert_eq!(page_columns(4.0 * 480.0 + 3.0), 4, "four and the lines between them");
        assert_eq!(page_columns(4.0 * 480.0 + 2.0), 1, "under four, one page across");
        assert_eq!(page_columns(1500.0), 1);
    }

    /// The list marks the pages in view, which with pages side by side
    /// need not follow one another: pages between two in view, themselves
    /// out of view, are not marked (the review's B6).
    #[test]
    fn the_list_marks_the_pages_in_view_not_those_between() {
        use egui_kittest::kittest::Queryable as _;
        let pages = ["Basic", "Statistics", "Appearance", "Classes", "Skills", "Feats"];
        let heights = [100.0, 900.0, 100.0, 100.0, 900.0, 100.0];
        // (The page at the top kept from frame to frame, as a host keeps it.)
        let at = std::rc::Rc::new(std::cell::Cell::new(0));
        let kept = at.clone();
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(2200.0, 316.0)).build_ui(
            move |ui| {
                mg_uikit::apply(ui.ctx(), mg_uikit::Theme::Dark);
                let id = egui::Id::new("marks");
                let mut top = kept.get();
                let mut listed = list(ui, id, "Pages", &pages, &mut top);
                scroll(id, &pages, false, listed.chosen, 4).show(
                    &mut listed.body,
                    &mut top,
                    |ui, i| {
                        ui.allocate_exact_size(
                            egui::vec2(10.0, heights[i] - 48.0),
                            egui::Sense::hover(),
                        );
                    },
                );
                kept.set(top);
            },
        );
        h.run();
        h.get_by_label("Skills").click();
        h.run();
        let seen = PageScroll::seen(&h.ctx, egui::Id::new("marks")).unwrap();
        // (Classes, its column's last, reaches down the view.)
        assert_eq!(seen.pages_in_view, vec![1, 3, 4, 5], "all but Basic and Appearance in view");
        let t = mg_uikit::tokens::Tokens::DARK;
        let marks: Vec<egui::Rect> = (h.output().shapes.iter())
            .filter_map(|c| match &c.shape {
                egui::Shape::Rect(r)
                    if r.fill == t.line_soft && r.rect.width() == mg_uikit::pages::IN_VIEW_MARK =>
                {
                    Some(r.rect)
                }
                _ => None,
            })
            .collect();
        let marked = |n: &str| {
            let row = h.get_by_label(n).rect();
            marks.iter().any(|m| row.contains(m.center()))
        };
        assert!(marked("Statistics") && marked("Classes"), "those in view marked: {marks:?}");
        assert!(!marked("Appearance"), "Appearance between them, out of view, not marked");
    }
}
