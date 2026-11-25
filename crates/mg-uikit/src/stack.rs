//! Panels stacked in a dock: their arrangement ([`StackLayout`]: columns
//! side by side, each a list of sections from the top, each section the
//! panels sharing its tab bar) and what a panel dropped on it does
//! ([`StackZone`], [`StackLayout::apply`]). Plain data over the
//! application's own panel type; the docks draw it (their bars, bodies
//! and seams).
//!
//! ```
//! use mg_uikit::stack::{StackLayout, StackRules, StackZone};
//! #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//! enum P { Palette, Preview, Properties }
//! let rules = StackRules {
//!     all: &[P::Palette, P::Preview, P::Properties],
//!     most_columns: 2,
//!     default: || StackLayout { columns: vec![vec![vec![P::Palette, P::Preview, P::Properties]]] },
//! };
//! let one = (rules.default)();
//! // Properties dropped under the first section: a section of its own.
//! let two = one.apply(&rules, P::Properties, StackZone::Below { col: 0, sec: 0 });
//! assert_eq!(two.columns, vec![vec![vec![P::Palette, P::Preview], vec![P::Properties]]]);
//! ```

use egui::{Pos2, Rect};
use serde::{Deserialize, Serialize};

/// Columns side by side, from the left; each a list of sections from the
/// top; each section the panels in its tab bar, in order. Serialized as
/// the bare nesting (`{"columns": [[["Palette", "Preview"]], ...]}`), as
/// the toolset's area Right Dock has kept it since 3.x.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, bound(serialize = "P: Serialize", deserialize = "P: Deserialize<'de>"))]
pub struct StackLayout<P> {
    pub columns: Vec<Vec<Vec<P>>>,
}

impl<P> Default for StackLayout<P> {
    fn default() -> Self {
        StackLayout { columns: Vec::new() }
    }
}

/// What an application's stack holds and allows: every panel it must
/// show (one missing is added to the last column's first section), at
/// most `most_columns` columns (those past it join the last), and the
/// layout where nothing is left (an empty or unreadable one kept).
#[derive(Clone, Copy, Debug)]
pub struct StackRules<'a, P> {
    pub all: &'a [P],
    pub most_columns: usize,
    pub default: fn() -> StackLayout<P>,
}

/// Where a panel dropped on a stack goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StackZone {
    /// Into the section's tab bar at `index` (the dragged panel's own
    /// section: moved there among its tabs; another's: joins it).
    Tab { col: usize, sec: usize, index: usize },
    /// Into the section, its last tab.
    Into { col: usize, sec: usize },
    /// A section of its own above the section.
    Above { col: usize, sec: usize },
    /// A section of its own below the section.
    Below { col: usize, sec: usize },
    /// A column of its own (or the top of that column where there are
    /// already as many as allowed): the first, or the last.
    Column { last: bool },
}

impl<P: Copy + PartialEq> StackLayout<P> {
    /// Where `p` is: its column, section and place in the tab bar.
    pub fn find(&self, p: P) -> Option<(usize, usize, usize)> {
        for (c, col) in self.columns.iter().enumerate() {
            for (s, sec) in col.iter().enumerate() {
                if let Some(i) = sec.iter().position(|&q| q == p) {
                    return Some((c, s, i));
                }
            }
        }
        None
    }

    /// The panels, from the first column's top.
    pub fn panels(&self) -> impl Iterator<Item = P> + '_ {
        self.columns.iter().flatten().flatten().copied()
    }

    /// Without `p`: the sections and columns it leaves empty dropped.
    pub fn without(&self, p: P) -> Self {
        let columns = (self.columns.iter())
            .map(|col| {
                (col.iter())
                    .map(|sec| sec.iter().copied().filter(|&q| q != p).collect::<Vec<_>>())
                    .filter(|sec| !sec.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|col| !col.is_empty())
            .collect();
        StackLayout { columns }
    }

    /// Each panel once (its first place kept), no empty section or column, at most `rules.most_columns` columns
    /// (those past it join the last), every panel of `rules.all` there
    /// (one missing goes to the last column's first section); the default
    /// where nothing is left.
    pub fn normalized(&self, rules: &StackRules<'_, P>) -> Self {
        let mut seen: Vec<P> = Vec::new();
        let mut columns: Vec<Vec<Vec<P>>> = Vec::new();
        for col in &self.columns {
            let mut clean_col = Vec::new();
            for sec in col {
                let mut clean_sec = Vec::new();
                for &p in sec {
                    if !seen.contains(&p) {
                        seen.push(p);
                        clean_sec.push(p);
                    }
                }
                if !clean_sec.is_empty() {
                    clean_col.push(clean_sec);
                }
            }
            if clean_col.is_empty() {
                continue;
            }
            match columns.len() >= rules.most_columns.max(1) {
                true => columns.last_mut().expect("a column").extend(clean_col),
                false => columns.push(clean_col),
            }
        }
        if columns.is_empty() {
            return (rules.default)();
        }
        for &p in rules.all {
            if !seen.contains(&p) {
                columns.last_mut().expect("a column")[0].push(p);
            }
        }
        StackLayout { columns }
    }

    /// `dragged` dropped at `zone`: the layout it makes, normalized. A
    /// zone that names no section, or a section that holds `dragged`
    /// alone, leaves it as it was; on an empty stack the panel is its one
    /// section. `dragged` may be another stack's (moved between docks):
    /// take it out of that one only where it is in the result ([`find`](Self::find)).
    pub fn apply(&self, rules: &StackRules<'_, P>, dragged: P, zone: StackZone) -> Self {
        let norm = self.normalized(rules);
        // (A panel of the section other than the dragged one, to find the
        // section by once the dragged one is taken out.)
        let anchor = |col: usize, sec: usize| {
            norm.columns.get(col)?.get(sec)?.iter().copied().find(|&q| q != dragged)
        };
        let mut rest = norm.without(dragged);
        match zone {
            StackZone::Tab { col, sec, index } => {
                if let Some((c, s, i)) = norm.find(dragged)
                    && (c, s) == (col, sec)
                {
                    let mut columns = norm.columns.clone();
                    let tabs = &mut columns[c][s];
                    if i != index && index < tabs.len() {
                        let p = tabs.remove(i);
                        tabs.insert(index, p);
                    }
                    return StackLayout { columns }.normalized(rules);
                }
                return self.apply(rules, dragged, StackZone::Into { col, sec });
            }
            StackZone::Column { last } => {
                let at = if last { rest.columns.len().saturating_sub(1) } else { 0 };
                match rest.columns.len() >= rules.most_columns.max(1) {
                    true => rest.columns[at].insert(0, vec![dragged]),
                    false if last => rest.columns.push(vec![vec![dragged]]),
                    false => rest.columns.insert(0, vec![vec![dragged]]),
                }
            }
            // (An empty stack, another dock's panel let go on it: its
            // one section.)
            _ if rest.columns.is_empty() => rest.columns.push(vec![vec![dragged]]),
            StackZone::Into { col, sec }
            | StackZone::Above { col, sec }
            | StackZone::Below { col, sec } => {
                let Some(anchor) = anchor(col, sec) else { return norm };
                let Some((c, s, _)) = rest.find(anchor) else { return norm };
                match zone {
                    StackZone::Into { .. } => rest.columns[c][s].push(dragged),
                    StackZone::Above { .. } => rest.columns[c].insert(s, vec![dragged]),
                    _ => rest.columns[c].insert(s + 1, vec![dragged]),
                }
            }
        }
        rest.normalized(rules)
    }

    /// Whether `dragged`, taken out, leaves room for a column of its own
    /// (fewer columns than `rules.most_columns`): where a drop at the
    /// pane's edges makes one ([`StackDrop::new_columns`]).
    pub fn room_for_column(&self, rules: &StackRules<'_, P>, dragged: P) -> bool {
        self.normalized(rules).without(dragged).columns.len() < rules.most_columns
    }
}

/// A section of a stack as drawn, for [`StackDrop`]: its place in the
/// layout, its rectangle (tab bar or header at the top), its panels and
/// its tabs' rectangles (none where a lone panel has a header).
#[derive(Clone, Debug)]
pub struct DropSection<P> {
    pub col: usize,
    pub sec: usize,
    pub rect: Rect,
    pub panels: Vec<P>,
    pub tabs: Vec<(P, Rect)>,
}

/// One dock's stack as drawn, to find where a dragged panel would land:
/// over a section's tab bar it joins the bar (its own: moves among its
/// tabs), over a section's top or bottom 30% it makes a section above or
/// below, over the middle it joins the section; with `new_columns`, the
/// pane's outer 18% (40 points at least, 40% at most) makes a column.
#[derive(Clone, Copy, Debug)]
pub struct StackDrop<'a, P> {
    pub pane: Rect,
    pub sections: &'a [DropSection<P>],
    /// The height of a section's tab bar or header.
    pub bar_height: f32,
    /// A drop at the pane's left or right edge makes a column
    /// ([`StackLayout::room_for_column`], where the dock is wide enough).
    pub new_columns: bool,
}

impl<P: Copy + PartialEq> StackDrop<'_, P> {
    /// Where `dragged` lands with the pointer at `pos`, and the rectangle
    /// to outline there ([`paint_drop_preview`]); none outside the pane or
    /// over the section it is alone in.
    pub fn target(&self, dragged: P, pos: Pos2) -> Option<(StackZone, Rect)> {
        let pane = self.pane;
        if !pane.contains(pos) {
            return None;
        }
        for b in self.sections {
            let bar = b.rect.with_max_y((b.rect.top() + self.bar_height).min(b.rect.bottom()));
            if !bar.contains(pos) {
                continue;
            }
            if !b.panels.contains(&dragged) {
                return Some((StackZone::Into { col: b.col, sec: b.sec }, bar));
            }
            if b.panels.len() > 1 {
                for (index, (p, tab)) in b.tabs.iter().enumerate() {
                    if *p != dragged && tab.contains(pos) {
                        return Some((StackZone::Tab { col: b.col, sec: b.sec, index }, *tab));
                    }
                }
            }
        }
        if self.new_columns {
            let edge = (pane.width() * 0.18).max(40.0).min(pane.width() * 0.4);
            if pos.x <= pane.left() + edge {
                return Some((StackZone::Column { last: false }, pane.with_max_x(pane.center().x)));
            }
            if pos.x >= pane.right() - edge {
                return Some((StackZone::Column { last: true }, pane.with_min_x(pane.center().x)));
            }
        }
        for b in self.sections {
            if !b.rect.contains(pos) || b.panels == [dragged] {
                continue;
            }
            let (col, sec, h) = (b.col, b.sec, b.rect.height());
            let rel = (pos.y - b.rect.top()) / h.max(1.0);
            return Some(match rel {
                r if r < 0.30 => {
                    (StackZone::Above { col, sec }, b.rect.with_max_y(b.rect.top() + h * 0.5))
                }
                r if r > 0.70 => {
                    (StackZone::Below { col, sec }, b.rect.with_min_y(b.rect.top() + h * 0.5))
                }
                _ => (StackZone::Into { col, sec }, b.rect),
            });
        }
        None
    }
}

/// Where a dragged panel lands among several docks ([`DockDrop::target`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropTarget<D> {
    pub dock: D,
    pub zone: StackZone,
    pub preview: Rect,
}

/// The docks a dragged panel may be dropped on, each its [`StackDrop`]:
/// an application that keeps panels in their own dock registers that
/// dock alone (a drop over any other lands nowhere); one that moves them
/// between docks registers each, applies the zone to the new dock's
/// layout and, where the panel is in what that makes, takes it out of
/// the old one's ([`StackLayout::without`]). Such docks' [`StackRules`]
/// name in `all` none of the panels they may give away (`&[]`): a panel
/// named there is put back by [`StackLayout::normalized`].
#[derive(Debug)]
pub struct DockDrop<'a, D, P> {
    docks: Vec<(D, StackDrop<'a, P>)>,
}

impl<D, P> Default for DockDrop<'_, D, P> {
    fn default() -> Self {
        DockDrop { docks: Vec::new() }
    }
}

impl<'a, D: Copy, P: Copy + PartialEq> DockDrop<'a, D, P> {
    pub fn new() -> Self {
        Self::default()
    }

    /// `dock` takes drops where `stack` says.
    pub fn dock(mut self, dock: D, stack: StackDrop<'a, P>) -> Self {
        self.docks.push((dock, stack));
        self
    }

    /// The first registered dock whose pane holds `pos`, and where in it
    /// `dragged` lands.
    pub fn target(&self, dragged: P, pos: Pos2) -> Option<DropTarget<D>> {
        let (dock, stack) = self.docks.iter().find(|(_, s)| s.pane.contains(pos))?;
        let (zone, preview) = stack.target(dragged, pos)?;
        Some(DropTarget { dock: *dock, zone, preview })
    }
}

/// Outlines where a dragged panel would land: the accent's tint and rim,
/// over every window.
pub fn paint_drop_preview(ctx: &egui::Context, rect: Rect) {
    let accent = crate::tokens(ctx).accent;
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("dock-drop-preview"));
    let painter = ctx.layer_painter(layer);
    painter.rect_filled(rect, 2.0, accent.gamma_multiply(0.22));
    painter.rect_stroke(rect, 2.0, egui::Stroke::new(2.0, accent), egui::StrokeKind::Inside);
}

/// The least height of a stacked section where the column has the room
/// ([`stack_heights`]): its bar and a few rows.
pub const SECTION_LEAST: f32 = 80.0;

/// The heights of stacked sections in `room` (the 1-point lines between
/// them left out) by their `shares`: each its share of the room, at
/// least its own least (`leasts`; [`SECTION_LEAST`] past their end) where
/// the room has that for it and those after it, each least held to an
/// even part of a short column's room, the last taking what is left so
/// that they fill it in whole points.
pub fn stack_heights(room: f32, shares: &[f32], leasts: &[f32]) -> Vec<f32> {
    let count = shares.len();
    if count == 0 {
        return Vec::new();
    }
    let total: f32 = shares.iter().sum::<f32>().max(f32::EPSILON);
    let leasts: Vec<f32> = (0..count)
        .map(|i| leasts.get(i).copied().unwrap_or(SECTION_LEAST).min(room / count as f32))
        .collect();
    let mut heights = Vec::with_capacity(count);
    let mut left = room;
    for (i, share) in shares.iter().enumerate() {
        let after: f32 = leasts[i + 1..].iter().sum();
        let least = leasts[i];
        let h = match i + 1 == count {
            true => left,
            false => (room * share / total).round().clamp(least, (left - after).max(least)),
        };
        heights.push(h.max(0.0));
        left -= h;
    }
    heights
}

/// The rectangles of sections `heights` tall stacked in `column` from its
/// top, a 1-point line between each two ([`stack_seams`] draws them).
pub fn stack_rects(column: Rect, heights: &[f32]) -> Vec<Rect> {
    let mut y = column.top();
    (heights.iter())
        .map(|h| {
            let r = Rect::from_x_y_ranges(column.x_range(), y..=y + h);
            y += h + 1.0;
            r
        })
        .collect()
}

/// A seam between stacked sections dragged ([`stack_seams`]): the line
/// under section `index`, and every section's height with that one and
/// the one under it changed (their sum kept, each at least
/// [`SECTION_LEAST`] or half of it).
#[derive(Clone, Debug, PartialEq)]
pub struct SeamDrag {
    pub index: usize,
    pub heights: Vec<f32>,
}

/// Draws the lines between the sections at `rects` and makes each a seam
/// to drag (3 points either side; a drag alone, never the keyboard's
/// Tab), except those `fixed` names by the section above it; called after
/// the sections' own widgets, so that the seams are over them. The seam
/// being dragged, if any: the application keeps the shares it makes.
pub fn stack_seams(
    ui: &egui::Ui,
    rects: &[Rect],
    salt: impl std::hash::Hash + std::fmt::Debug + Copy,
    fixed: impl Fn(usize) -> bool,
) -> Option<SeamDrag> {
    let line_color = crate::tokens(ui.ctx()).line;
    let mut drag = None;
    for i in 0..rects.len().saturating_sub(1) {
        let top = rects[i].bottom();
        let line = Rect::from_x_y_ranges(rects[i].x_range(), top..=top + 1.0);
        ui.painter().rect_filled(line, 0.0, line_color);
        if fixed(i) {
            continue;
        }
        let grip = line.expand2(egui::vec2(0.0, 3.0));
        let seam = ui.interact(grip, egui::Id::new((salt, "seam", i)), egui::Sense::DRAG);
        if seam.hovered() || seam.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        let (Some(at), true) = (seam.interact_pointer_pos(), seam.dragged()) else { continue };
        let pair = rects[i].height() + rects[i + 1].height();
        let least = SECTION_LEAST.min(pair / 2.0);
        let upper = (at.y - rects[i].top()).clamp(least, pair - least);
        let mut heights: Vec<f32> = rects.iter().map(Rect::height).collect();
        heights[i] = upper;
        heights[i + 1] = pair - upper;
        drag = Some(SeamDrag { index: i, heights });
    }
    drag
}

/// The seam between two columns at `x` in `pane` (a drag alone, 3 points
/// left of the line and 4 right, from its top to its bottom): the first
/// column's width dragged to this frame, each column at least `least`
/// (the first less where the pane has not the room), the line between
/// them 1 point.
pub fn column_seam(ui: &egui::Ui, id: egui::Id, pane: Rect, x: f32, least: f32) -> Option<f32> {
    let grip = Rect::from_x_y_ranges(x - 3.0..=x + 4.0, pane.y_range());
    let seam = ui.interact(grip, id, egui::Sense::DRAG);
    if seam.hovered() || seam.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    let at = seam.interact_pointer_pos().filter(|_| seam.dragged())?;
    let most = (pane.width() - 1.0 - least).max(least);
    Some((at.x - pane.left()).clamp(least.min(most), most).round())
}

/// What a dock keeps of its arrangement for one kind of editor, by the
/// panels' names (a name a later version does not know is passed by):
/// the panel in front of each section, the sections' shares of the
/// height as their seams were dragged, the first column's width where the
/// dock stands in two, and the panel in front where the sections fold
/// into one tab bar. Serialized with what is not kept left out. The
/// toolset keeps the same fields in its own record (with more beside
/// them) through [`keep_front`] and [`keep_shares`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeptLayout {
    /// The panel in front of each section, from the top (`""`: none
    /// chosen, its first).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fronts: Vec<String>,
    /// Each section's share of the height, from the top, in thousandths
    /// of an equal share (1000 each, or none kept: equal).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shares: Vec<u16>,
    /// The first column's width where the dock stands in two.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u16>,
    /// The panel in front of the one tab bar the sections fold into.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folded: Option<String>,
}

impl KeptLayout {
    /// The name of the panel kept in front of section `index`.
    pub fn front(&self, index: usize) -> Option<&str> {
        self.fronts.get(index).map(String::as_str).filter(|n| !n.is_empty())
    }

    /// `name` chosen in front of section `index` (`folded`: chosen in the
    /// one tab bar, that bar's front too).
    pub fn set_front(&mut self, index: usize, name: &str, folded: bool) {
        if folded {
            self.folded = Some(name.to_string());
        }
        keep_front(&mut self.fronts, index, name);
    }

    /// Section `index`'s share as kept (at least 1), for [`stack_heights`].
    pub fn share(&self, index: usize) -> Option<f32> {
        self.shares.get(index).map(|s| f32::from((*s).max(1)))
    }

    /// The heights a seam's drag made ([`SeamDrag::heights`]), kept as
    /// shares (see [`keep_shares`]).
    pub fn keep_shares(&mut self, sections: usize, heights: &[f32], own: &[Option<usize>]) {
        keep_shares(&mut self.shares, sections, heights, own);
    }

    /// Nothing kept: the record can be left out.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// `name` kept in front of section `index` of `fronts`, those before it
/// with none chosen (`""`) where there were none.
pub fn keep_front(fronts: &mut Vec<String>, index: usize, name: &str) {
    if fronts.len() <= index {
        fronts.resize(index + 1, String::new());
    }
    fronts[index] = name.to_string();
}

/// The sections' `heights` (from the top, as a seam's drag made them)
/// kept in `shares` (thousandths of an equal share): each section's at
/// its own place (`own`, by the section; none: one not the kind's own,
/// not kept), `shares` at least `sections` long (1000 each where none
/// was kept).
pub fn keep_shares(shares: &mut Vec<u16>, sections: usize, heights: &[f32], own: &[Option<usize>]) {
    let total: f32 = heights.iter().sum::<f32>().max(f32::EPSILON);
    let count = heights.len() as f32;
    let need = sections.max(own.iter().flatten().copied().max().map_or(0, |m| m + 1));
    if shares.len() < need {
        shares.resize(need, 1000);
    }
    for (h, index) in heights.iter().zip(own) {
        if let Some(index) = index {
            shares[*index] = ((h / total) * 1000.0 * count).round().max(1.0) as u16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    enum P {
        Palette,
        Preview,
        Properties,
    }
    use P::*;

    fn two() -> StackLayout<P> {
        StackLayout { columns: vec![vec![vec![Palette, Preview]], vec![vec![Properties]]] }
    }

    const RULES: StackRules<'static, P> =
        StackRules { all: &[Palette, Preview, Properties], most_columns: 2, default: two };

    fn cols(c: &[&[&[P]]]) -> StackLayout<P> {
        StackLayout {
            columns: c.iter().map(|col| col.iter().map(|s| s.to_vec()).collect()).collect(),
        }
    }

    #[test]
    fn normalized_keeps_each_once_and_adds_the_missing() {
        let dirty = cols(&[&[&[Palette, Palette], &[]], &[], &[&[Preview]], &[&[Preview]]]);
        assert_eq!(dirty.normalized(&RULES), cols(&[&[&[Palette]], &[&[Preview, Properties]]]));
        assert_eq!(StackLayout::default().normalized(&RULES), two());
        // (A third column joins the second.)
        let three = cols(&[&[&[Palette]], &[&[Preview]], &[&[Properties]]]);
        assert_eq!(three.normalized(&RULES), cols(&[&[&[Palette]], &[&[Preview], &[Properties]]]));
    }

    #[test]
    fn drops_stack_split_and_join() {
        let stacked = two().apply(&RULES, Properties, StackZone::Below { col: 0, sec: 0 });
        assert_eq!(stacked, cols(&[&[&[Palette, Preview], &[Properties]]]));
        let tabbed = two().apply(&RULES, Properties, StackZone::Into { col: 0, sec: 0 });
        assert_eq!(tabbed, cols(&[&[&[Palette, Preview, Properties]]]));
        let right = tabbed.apply(&RULES, Properties, StackZone::Column { last: true });
        assert_eq!(right, two());
        let left = two().apply(&RULES, Properties, StackZone::Column { last: false });
        assert_eq!(left, cols(&[&[&[Properties]], &[&[Palette, Preview]]]));
        let above = stacked.apply(&RULES, Preview, StackZone::Above { col: 0, sec: 0 });
        assert_eq!(above, cols(&[&[&[Preview], &[Palette], &[Properties]]]));
    }

    #[test]
    fn a_tab_moves_within_its_bar_or_joins_another() {
        let tabbed = cols(&[&[&[Palette, Preview, Properties]]]);
        let moved = tabbed.apply(&RULES, Properties, StackZone::Tab { col: 0, sec: 0, index: 0 });
        assert_eq!(moved, cols(&[&[&[Properties, Palette, Preview]]]));
        let joined = two().apply(&RULES, Properties, StackZone::Tab { col: 0, sec: 0, index: 1 });
        assert_eq!(joined, cols(&[&[&[Palette, Preview, Properties]]]));
    }

    #[test]
    fn a_drop_on_nothing_or_itself_changes_nothing() {
        let alone = cols(&[&[&[Palette, Preview], &[Properties]]]);
        assert_eq!(alone.apply(&RULES, Properties, StackZone::Above { col: 0, sec: 1 }), alone);
        assert_eq!(alone.apply(&RULES, Properties, StackZone::Into { col: 4, sec: 0 }), alone);
    }

    fn r(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1))
    }

    /// Stacked: Palette and Preview tabbed over Properties, in 0..300 x 0..400.
    fn stacked_sections() -> Vec<DropSection<P>> {
        vec![
            DropSection {
                col: 0,
                sec: 0,
                rect: r(0.0, 0.0, 300.0, 200.0),
                panels: vec![Palette, Preview],
                tabs: vec![
                    (Palette, r(0.0, 0.0, 80.0, 24.0)),
                    (Preview, r(80.0, 0.0, 160.0, 24.0)),
                ],
            },
            DropSection {
                col: 0,
                sec: 1,
                rect: r(0.0, 200.0, 300.0, 400.0),
                panels: vec![Properties],
                tabs: vec![],
            },
        ]
    }

    #[test]
    fn a_drop_lands_by_bar_body_and_edge() {
        let sections = stacked_sections();
        let at = |new_columns, dragged, x, y| {
            let drop = StackDrop {
                pane: r(0.0, 0.0, 300.0, 400.0),
                sections: &sections,
                bar_height: 24.0,
                new_columns,
            };
            drop.target(dragged, Pos2::new(x, y)).map(|(z, _)| z)
        };
        assert_eq!(at(false, Properties, 200.0, 10.0), Some(StackZone::Into { col: 0, sec: 0 }));
        assert_eq!(
            at(false, Preview, 40.0, 10.0),
            Some(StackZone::Tab { col: 0, sec: 0, index: 0 })
        );
        // (Over its own tab: the section's top, as the body takes it.)
        assert_eq!(at(false, Preview, 120.0, 10.0), Some(StackZone::Above { col: 0, sec: 0 }));
        assert_eq!(at(false, Properties, 150.0, 30.0), Some(StackZone::Above { col: 0, sec: 0 }));
        assert_eq!(at(false, Properties, 150.0, 100.0), Some(StackZone::Into { col: 0, sec: 0 }));
        assert_eq!(at(false, Properties, 150.0, 300.0), None, "alone in its section");
        assert_eq!(at(false, Palette, 150.0, 390.0), Some(StackZone::Below { col: 0, sec: 1 }));
        assert_eq!(at(true, Properties, 10.0, 300.0), Some(StackZone::Column { last: false }));
        assert_eq!(at(true, Properties, 295.0, 100.0), Some(StackZone::Column { last: true }));
        assert_eq!(at(true, Properties, 400.0, 100.0), None, "outside the pane");
    }

    #[test]
    fn only_registered_docks_take_a_drop() {
        #[derive(Clone, Copy, Debug, PartialEq)]
        enum Dock {
            Right,
            Left,
        }
        let sections = stacked_sections();
        let right = StackDrop {
            pane: r(0.0, 0.0, 300.0, 400.0),
            sections: &sections,
            bar_height: 24.0,
            new_columns: false,
        };
        let one = DockDrop::new().dock(Dock::Right, right);
        // (The toolset registers its Right Dock alone: a panel let go over
        // the Left or Bottom Dock lands nowhere.)
        assert_eq!(one.target(Properties, Pos2::new(-100.0, 100.0)), None);
        let hit = one.target(Properties, Pos2::new(150.0, 100.0)).expect("over the Right Dock");
        assert_eq!((hit.dock, hit.zone), (Dock::Right, StackZone::Into { col: 0, sec: 0 }));
        // (Another dock registered: a panel it does not hold joins it.)
        let left_sections = vec![DropSection {
            col: 0,
            sec: 0,
            rect: r(-300.0, 0.0, 0.0, 400.0),
            panels: vec![],
            tabs: vec![],
        }];
        let left = StackDrop {
            pane: r(-300.0, 0.0, 0.0, 400.0),
            sections: &left_sections,
            bar_height: 24.0,
            new_columns: false,
        };
        let both = DockDrop::new().dock(Dock::Right, right).dock(Dock::Left, left);
        let hit = both.target(Properties, Pos2::new(-100.0, 10.0)).expect("over the Left Dock");
        assert_eq!((hit.dock, hit.zone), (Dock::Left, StackZone::Into { col: 0, sec: 0 }));
        // (And the layouts, the zone found applied: into the empty Left
        // Dock, then out of the Right Dock, which names nothing in `all`.)
        let open = StackRules::<P> { all: &[], most_columns: 2, default: StackLayout::default };
        let to = StackLayout::default().apply(&open, Properties, hit.zone);
        assert_eq!(to, cols(&[&[&[Properties]]]));
        assert!(to.find(Properties).is_some(), "taken out of the Right Dock only then");
        let from = cols(&[&[&[Palette, Preview], &[Properties]]]).without(Properties);
        assert_eq!(from.normalized(&open), cols(&[&[&[Palette, Preview]]]));
        // (Into a dock that holds panels: it joins the section.)
        let left = cols(&[&[&[Palette]]]);
        let joined = left.apply(&open, Properties, StackZone::Into { col: 0, sec: 0 });
        assert_eq!(joined, cols(&[&[&[Palette, Properties]]]));
        // (A dock whose rules name the panel puts it back: why `all: &[]`.)
        assert_eq!(from.normalized(&RULES), cols(&[&[&[Palette, Preview, Properties]]]));
    }

    #[test]
    fn a_column_drop_keeps_to_the_rules_count() {
        let one = StackRules { most_columns: 1, ..RULES };
        let stacked = cols(&[&[&[Palette, Preview], &[Properties]]]);
        let top = stacked.apply(&one, Properties, StackZone::Column { last: true });
        assert_eq!(top, cols(&[&[&[Properties], &[Palette, Preview]]]), "one column: its top");
        let three = StackRules { most_columns: 3, ..RULES };
        let made = two().apply(&three, Preview, StackZone::Column { last: true });
        assert_eq!(made, cols(&[&[&[Palette]], &[&[Properties]], &[&[Preview]]]));
        let full = made.apply(&three, Palette, StackZone::Column { last: true });
        assert_eq!(full, cols(&[&[&[Properties]], &[&[Preview]], &[&[Palette]]]));
        let past = cols(&[&[&[Palette]], &[&[Properties]], &[&[Preview]]]);
        let at_top = past.apply(&three, Palette, StackZone::Column { last: false });
        assert_eq!(at_top, cols(&[&[&[Palette]], &[&[Properties]], &[&[Preview]]]));
        let last_top = past.apply(&three, Properties, StackZone::Column { last: true });
        assert_eq!(
            last_top,
            cols(&[&[&[Palette]], &[&[Preview]], &[&[Properties]]]),
            "its own room"
        );
    }

    #[test]
    fn room_for_a_column_counts_the_rest() {
        assert!(cols(&[&[&[Palette, Preview, Properties]]]).room_for_column(&RULES, Properties));
        assert!(two().room_for_column(&RULES, Properties));
        assert!(!two().room_for_column(&RULES, Palette));
    }

    #[test]
    fn heights_share_the_room_each_at_least_its_least() {
        let even = [SECTION_LEAST; 3];
        assert_eq!(stack_heights(600.0, &[1000.0, 1000.0], &even), vec![300.0, 300.0]);
        assert_eq!(stack_heights(601.0, &[1000.0; 3], &even), vec![200.0, 200.0, 201.0]);
        assert_eq!(stack_heights(600.0, &[3000.0, 1000.0], &even), vec![450.0, 150.0]);
        assert_eq!(stack_heights(600.0, &[10.0, 1000.0, 10.0], &even), vec![80.0, 440.0, 80.0]);
        assert_eq!(stack_heights(100.0, &[1.0, 1.0], &even), vec![50.0, 50.0], "short: halves");
        assert_eq!(stack_heights(600.0, &[1.0, 1000.0], &[150.0, 80.0]), vec![150.0, 450.0]);
        assert!(stack_heights(100.0, &[], &[]).is_empty());
        let rects = stack_rects(r(0.0, 10.0, 50.0, 611.0), &[300.0, 300.0]);
        assert_eq!(rects, vec![r(0.0, 10.0, 50.0, 310.0), r(0.0, 311.0, 50.0, 611.0)]);
    }

    #[test]
    fn a_kept_layout_keeps_fronts_and_shares_by_place() {
        let mut kept = KeptLayout::default();
        assert!(kept.is_empty());
        kept.set_front(2, "Help", false);
        assert_eq!(kept.fronts, ["", "", "Help"]);
        assert_eq!((kept.front(0), kept.front(2), kept.front(5)), (None, Some("Help"), None));
        kept.set_front(0, "Functions", true);
        assert_eq!(kept.folded.as_deref(), Some("Functions"));
        // (Two sections dragged to 300 and 100, the first not the kind's
        // own; the kind brings three.)
        kept.keep_shares(3, &[300.0, 100.0], &[None, Some(1)]);
        assert_eq!(kept.shares, [1000, 500, 1000]);
        assert_eq!(kept.share(1), Some(500.0));
        kept.keep_shares(3, &[200.0, 200.0], &[Some(0), Some(1)]);
        assert_eq!(kept.shares, [1000, 1000, 1000]);
        let json = serde_json::to_string(&KeptLayout { column: Some(340), ..Default::default() });
        assert_eq!(json.unwrap(), r#"{"column":340}"#);
    }

    #[test]
    fn kept_as_the_bare_nesting() {
        let json = serde_json::to_string(&two()).unwrap();
        assert_eq!(json, r#"{"columns":[[["Palette","Preview"]],[["Properties"]]]}"#);
        let back: StackLayout<P> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, two());
        let empty: StackLayout<P> = serde_json::from_str("{}").unwrap();
        assert!(empty.columns.is_empty());
    }
}
