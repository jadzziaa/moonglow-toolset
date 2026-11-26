//! Rows, columns and groups (with a group's scrolling), and lists.
use super::*;
use std::fmt::Write as _;

/// The most rows of a list drawn at once (from the first in its view), and
/// laid out off the screen (from its first).
const ROWS_DRAWN: usize = 512;

/// For each of `children`, the largest width any control after it has of
/// its own (-1: none has one): what a row's last button stretches to
/// (`Renderer::children`). One pass from the end. (Each child looked at
/// all those after it, every frame: 300 rows in a column were 45,000
/// lookups, each with its bind's.)
pub(super) fn widths_after(children: &[Value], s: &Settings, row: Option<usize>) -> Vec<f32> {
    let mut after = vec![-1.0; children.len()];
    let mut widest = -1.0_f32;
    for (i, child) in children.iter().enumerate().rev() {
        after[i] = widest;
        widest = widest.max(num(&child["width"], s, row, -1.0));
    }
    after
}

impl Renderer<'_> {
    /// A row, a column or a group: its children side by side or one under
    /// another, in what the group's bars leave.
    pub(super) fn container(&mut self, ui: &mut Ui, c: &mut Control<'_>) {
        let Control { s, a, scale, node, ty, row, body, tint, .. } = *c;
        let p = &c.p;
        if ty == "group" && flag(&node["border"], s, row, true) {
            // The window's root group has its border 4 points out, in the
            // window's inset (the stage lets it draw there).
            if c.source_path == "/root" {
                a.paint(
                    ui.painter(),
                    "window.border_image",
                    body.expand(4.0 * scale),
                    scale,
                    true,
                    tint,
                );
            } else {
                a.paint(p, "window.border_image", body, scale, true, tint);
            }
        }
        let mut inner = if ty == "group" { body.shrink(4.0 * scale) } else { body };
        let mut scrolled = Vec2::ZERO;
        if ty == "group" {
            scrolled = self.group_scroll(ui, c, &mut inner);
        }
        if let Some(children) = node["children"].as_array() {
            self.children(ui, c, children, inner, scrolled);
        }
    }

    /// A group's scrollbars: which of them show, what they take of `inner`,
    /// and how far the group is scrolled.
    fn group_scroll(&mut self, ui: &mut Ui, c: &Control<'_>, inner: &mut Rect) -> Vec2 {
        let Control {
            s,
            a,
            scale,
            node,
            row,
            depth,
            body,
            path,
            source_path,
            disabled,
            editable,
            ..
        } = *c;
        let p = &c.p;
        let mut scrolled = Vec2::ZERO;
        // Nuklear scrolls a group until its content's far edge (with
        // the group's 2-point padding each side) meets the end of the
        // bar's track, which falls short of the view by the bar's two
        // end buttons (NWN EE 8193.37, np_group: 374 + 4 − (124 − 36)
        // = 290 down).
        let content = self.cell(&node["children"][0], row, depth + 1) + Vec2::splat(4.0 * scale);
        let scroll = val(&node["scrollbars"], s, row).as_i64().unwrap_or(4);
        // x: the vertical bar's width; y: the horizontal bar's height.
        let bars = vec2(
            a.number("window.scrollbar_size_x", 18.0),
            a.number("window.scrollbar_size_y", 18.0),
        ) * scale;
        let mut on = [matches!(scroll, 1 | 3), matches!(scroll, 2 | 3)];
        if scroll == 4 {
            on[0] = content.x > inner.width();
            on[1] = content.y > inner.height() - if on[0] { bars.y } else { 0.0 };
            on[0] = content.x > inner.width() - if on[1] { bars.x } else { 0.0 };
        }
        if on[0] {
            inner.max.y -= bars.y;
        }
        if on[1] {
            inner.max.x -= bars.x;
        }
        let key = format!("{path}/{row:?}");
        for axis in [0, 1] {
            let horizontal = axis == 0;
            // A bar's end buttons are as long as the bar is thick.
            let thick = if horizontal { bars.y } else { bars.x };
            let max = (content[axis] - (inner.size()[axis] - 2.0 * thick)).max(0.0);
            let stored =
                if horizontal { &self.state.group_scroll_x } else { &self.state.group_scroll_y };
            // Logical units keep the visible content stable when zoom changes.
            let mut offset = stored.get(&key).copied().unwrap_or(0.0) * scale;
            if on[axis] {
                let mut area = body;
                if horizontal && on[1] {
                    area.max.x -= bars.x;
                } else if !horizontal && on[0] {
                    area.max.y -= bars.y;
                }
                let inset = 2.0 * scale;
                let mut strip = area.shrink(inset);
                strip.min[1 - axis] = strip.max[1 - axis] - thick;
                let scroll_response = ui.interact(
                    strip,
                    ui.id().with(("group-scroll", axis, source_path, row)),
                    Sense::click_and_drag(),
                );
                scroll_response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Slider,
                        !disabled,
                        format!(
                            "Scroll group {} {path}",
                            if horizontal { "horizontally" } else { "vertically" }
                        ),
                    )
                });
                if !disabled && editable {
                    let bar = Bar { strip, axis, button: thick, max };
                    offset = scroll_by(ui, &scroll_response, body, &bar, offset);
                }
                offset = offset.clamp(0.0, max);
                self.scrollbar(
                    p,
                    area,
                    (inner.size()[axis] - 2.0 * thick) / content[axis].max(1.0),
                    offset / max.max(1.0),
                    horizontal,
                );
            } else {
                offset = 0.0;
            }
            scrolled[axis] = offset;
            if horizontal {
                self.state.group_scroll_x.insert(key.clone(), offset / scale);
            } else {
                self.state.group_scroll_y.insert(key.clone(), offset / scale);
            }
        }
        scrolled
    }

    /// The children of a row, column or group, each in the room the client
    /// gives it.
    fn children(
        &mut self,
        ui: &mut Ui,
        c: &Control<'_>,
        children: &[Value],
        inner: Rect,
        scrolled: Vec2,
    ) {
        let Control { s, scale, ty, row, depth, source_path, .. } = *c;
        let p = &c.p;
        if children.is_empty() && self.state.guides() {
            p.rect_stroke(inner, 0, Stroke::new(1.0, GOLD.gamma_multiply(0.5)), StrokeKind::Inside);
            c.label(inner, "+ Drop a control", Align2::CENTER_CENTER, false);
        }
        let gap = self.gap_px(4.0);
        let make = || {
            let total_gap = children
                .iter()
                .take(children.len().saturating_sub(1))
                .map(|child| self.gap_px(child_gap(ty, child)))
                .sum::<f32>();
            let sizes: Vec<_> = children.iter().map(|n| self.cell(n, row, depth + 1)).collect();
            let fills: Vec<_> = children
                .iter()
                .map(|n| (fills_width(n, s, row), fills_height(n, s, row)))
                .collect();
            let fixed = sizes.iter().zip(&fills).filter(|(_, f)| !f.0).map(|(v, _)| v.x).sum();
            let fixed_height =
                sizes.iter().zip(&fills).filter(|(_, f)| !f.1).map(|(v, _)| v.y).sum();
            Plan {
                total_gap,
                flex: fills.iter().filter(|f| f.0).count(),
                height_flex: fills.iter().filter(|f| f.1).count(),
                fixed,
                fixed_height,
                // (A column's alone: the rows that reach are a column's.)
                widths_after: match ty {
                    "col" => widths_after(children, s, row),
                    _ => Vec::new(),
                },
                sizes,
                fills,
            }
        };
        let plan = plan(c.node, row, depth, &self.naturals, make);
        let Plan { sizes, fills, total_gap, fixed, flex, fixed_height, height_flex, widths_after } =
            &*plan;
        let (total_gap, fixed, fixed_height, flex, height_flex) =
            (*total_gap, *fixed, *fixed_height, *flex, *height_flex);
        // A row's last button without a width stretches to a control with
        // a width of its own after the row in its column, never before it
        // (NWN EE 8193.37: one button under an image 200 wide is 196, two
        // under 400 are 150 and 242; three under 400, or one under 100,
        // stay 150).
        let reach = match &self.reach {
            Some((path, right)) if ty == "row" && path == source_path => Some(*right),
            _ => None,
        };
        let share = ((inner.width() - fixed - total_gap) / flex.max(1) as f32).max(0.0);
        let height_share =
            ((inner.height() - fixed_height - total_gap) / height_flex.max(1) as f32).max(0.0);
        let mut cursor = inner.min - scrolled;
        let old_clip = ui.clip_rect();
        // Only a group (like the window and a list) clips what is
        // inside it; a row or a column doesn't: draw layers in it
        // reach past it in the client (np_scissor2).
        if ty == "group" {
            ui.set_clip_rect(inner.intersect(old_clip));
        }
        // Past what is in sight (below it in a column, right of it in a
        // row), nothing more is drawn (`draw_node` draws nothing out of
        // the clip), so the rest are not laid out, unless where they go is
        // asked for (`mg nui layout`): a group's column of 1,000 rows laid
        // every one out each frame.
        let clip = ui.clip_rect();
        let mut child_path = format!("{source_path}/children/");
        let base = child_path.len();
        let past = |cursor: egui::Pos2| match ty {
            "col" => cursor.y > clip.bottom(),
            "row" => cursor.x > clip.right(),
            _ => false,
        };
        for (i, child) in children.iter().enumerate() {
            if past(cursor) && !self.assets.recording() {
                break;
            }
            let mut size = sizes[i];
            let (fills_width, fills_height) = fills[i];
            if ty == "row" && fills_width {
                size.x = share;
            } else if ty != "row" && fills_width {
                size.x = if ty == "group" { inner.width().max(size.x) } else { inner.width() };
            }
            if ty == "group" && matches!(child["type"].as_str(), Some("col" | "row")) {
                // A group's layout fills it, or runs past it and scrolls.
                size.y = inner.height().max(size.y);
            } else if fills_height {
                size.y = if ty == "row" || ty == "group" {
                    inner.height()
                } else {
                    // Never less than its own: the window scrolls instead.
                    height_share.max(size.y)
                };
            } else if ty == "row" && num(&child["height"], s, row, -1.0) < 0.0 {
                // A control's own height gives way to its row's.
                size.y = size.y.min(inner.height());
            }
            if num(&child["aspect"], s, row, 0.0) > 0.0 && num(&child["height"], s, row, -1.0) < 0.0
            {
                size.y = size.x / num(&child["aspect"], s, row, 1.0);
            }
            if let Some(right) = reach
                && i + 1 == children.len()
                && !fills_width
                && num(&child["width"], s, row, -1.0) < 0.0
            {
                size.x = size.x.max(right - cursor.x);
            }
            // (No more than `natural` holds it to: an aspect of a millionth
            // is a control hundreds of millions of points high, and a
            // scrollbar as long.)
            size = size.min(self.scaled(Vec2::splat(LARGEST)));
            // (One string for every child's path, not one made for each.)
            child_path.truncate(base);
            let _ = write!(child_path, "{i}");
            let after = widths_after.get(i).copied().unwrap_or(-1.0);
            self.reach = (ty == "col" && child["type"] == "row" && after >= 0.0).then(|| {
                let padding = layout_padding(child, s, row) * scale;
                (child_path.clone(), cursor.x + self.px(after) - padding)
            });
            self.node(ui, child, &child_path, Rect::from_min_size(cursor, size), row, depth + 1);
            self.reach = None;
            if ty == "row" {
                cursor.x += size.x + gap;
            } else {
                cursor.y += size.y + self.gap_px(child_gap(ty, child));
            }
        }
        ui.set_clip_rect(old_clip);
    }

    pub(super) fn list(&mut self, ui: &mut Ui, c: &mut Control<'_>) {
        let Control {
            s,
            a,
            scale,
            node,
            row,
            depth,
            body,
            tint,
            path,
            source_path,
            disabled,
            editable,
            ..
        } = *c;
        let p = &c.p;
        if flag(&node["border"], s, row, true) {
            a.paint(p, "window.border_image", body, scale, true, tint);
        }
        let mut inner = body.shrink(4.0 * scale);
        let height =
            self.scaled(vec2(0.0, num(&node["row_height"], s, row, 25.0).clamp(1.0, 4096.0))).y;
        let stride = height + 4.0 * scale;
        let count = val(&node["row_count"], s, row);
        let count =
            count.as_array().map_or_else(|| count.as_u64().unwrap_or(0) as usize, |v| v.len());
        // Explicit Y/BOTH reserves a scrollbar even if all rows fit.
        let scrolling = matches!(val(&node["scrollbars"], s, row).as_i64().unwrap_or(2), 2 | 3);
        // A list's horizontal bar is drawn but has nothing to move:
        // cells past its width are cut (NWN EE 8193.37, np_list2).
        let mut y_body = body;
        if matches!(val(&node["scrollbars"], s, row).as_i64(), Some(1 | 3)) {
            let height = a.number("window.scrollbar_size_y", 18.0) * scale;
            let mut area = body;
            if scrolling {
                area.max.x -= a.number("window.scrollbar_size_x", 18.0) * scale;
            }
            self.scrollbar(p, area, 1.0, 0.0, true);
            inner.max.y -= height;
            y_body.max.y -= height;
        }
        let content_height = (count as f32 * stride - 4.0 * scale).max(0.0);
        // The client scrolls a list by whole rows, its last stop leaving
        // as many rows on top as fit with 36 points to spare (NWN EE
        // 8193.37: 10 rows in 100 end at row 9, 40 in 200 at row 35,
        // 40 in 300 at row 32).
        let fit = ((inner.height() - 36.0 * scale) / stride).floor().max(0.0);
        let max_scroll = ((count as f32 - fit).max(0.0) * stride).max(0.0);
        let scroll_key = format!("{path}/{row:?}");
        // (Kept in points, as a group's and a text's: the same rows show
        // at another zoom or UI scale.)
        let kept = self.state.list_scroll.get(&scroll_key).copied().unwrap_or(0.0);
        let mut offset = (kept * scale).clamp(0.0, max_scroll);
        if scrolling {
            let width = a.number("window.scrollbar_size_x", 18.0) * scale;
            inner.max.x -= width;
            let track = Rect::from_min_max(
                pos2(inner.right(), inner.top()),
                y_body.max - vec2(4.0, 4.0) * scale,
            );
            let scroll_response = ui.interact(
                track,
                ui.id().with(("list-scroll", source_path, row)),
                Sense::click_and_drag(),
            );
            scroll_response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Slider,
                    !disabled,
                    format!("Scroll list {path}"),
                )
            });
            if !disabled && editable {
                // (The track is what is between the native end buttons, so
                // that its bottom reaches the final row.)
                let bar = Bar { strip: track, axis: 1, button: width, max: max_scroll };
                offset = scroll_by(ui, &scroll_response, body, &bar, offset);
            }
            offset = offset.clamp(0.0, max_scroll);
            self.scrollbar(
                p,
                y_body,
                inner.height() / content_height.max(1.0),
                offset / max_scroll.max(1.0),
                false,
            );
        } else {
            offset = 0.0;
        }
        self.state.list_scroll.insert(scroll_key, offset / scale);
        if let Some(cells) = node["row_template"].as_array() {
            if cells.is_empty() && self.state.guides() {
                c.label(
                    inner,
                    "Empty list · Add controls to the row template",
                    Align2::CENTER_CENTER,
                    true,
                );
            }
            let widths = list_widths(cells, inner.width(), scale, |w| self.scaled(vec2(w, 0.0)).x);
            let old_clip = ui.clip_rect();
            ui.set_clip_rect(inner.intersect(old_clip));
            let mut first = (offset / stride).floor() as usize;
            // Native list rows include a four-unit gap and do not
            // expose the next row's border when only that gap fits.
            let mut end = count
                .min(((offset + inner.height() + 4.0 * scale) / stride).floor().max(0.0) as usize);
            // Where it goes asked for (`mg nui layout`): the rows out of
            // its view too, laid out where scrolling brings them, drawn
            // clipped to nothing. A list in another list's row keeps to
            // the rows in its view, as it is drawn (its rows are there
            // once for each row of the other), and all a window's lists
            // together to what `record_rows` gives them.
            if a.recording() {
                if row.is_none() {
                    (first, end) = (0, count);
                }
                let wanted = end.saturating_sub(first).min(ROWS_DRAWN);
                let taken = a.record_rows(source_path, row.is_some(), wanted, count);
                end = first.saturating_add(taken);
            }
            for r in first..end.min(first.saturating_add(ROWS_DRAWN)) {
                let mut x = inner.left();
                for (i, cell) in cells.iter().enumerate() {
                    let width = widths[i];
                    self.node(
                        ui,
                        &cell[0],
                        &format!("{source_path}/row_template/{i}/0"),
                        Rect::from_min_size(
                            pos2(x, inner.top() + r as f32 * stride - offset),
                            vec2(width, height),
                        ),
                        Some(r),
                        depth + 1,
                    );
                    x += width + 4.0 * scale;
                }
            }
            ui.set_clip_rect(old_clip);
        }
    }
}
