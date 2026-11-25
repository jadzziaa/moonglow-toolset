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
    /// alone, leaves it as it was.
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
                let at = if last { rest.columns.len().min(1) } else { 0 };
                match rest.columns.len() >= 2 {
                    true => rest.columns[at].insert(0, vec![dragged]),
                    false if last => rest.columns.push(vec![vec![dragged]]),
                    false => rest.columns.insert(0, vec![vec![dragged]]),
                }
            }
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
/// between docks registers each, takes the panel out of its old dock's
/// layout and applies the zone to the new one's.
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
        // (And the layouts: out of one, into the other.)
        let from = cols(&[&[&[Palette, Preview], &[Properties]]]).without(Properties);
        let open = StackRules::<P> { all: &[], most_columns: 2, default: StackLayout::default };
        let to = StackLayout::default().apply(&open, Properties, StackZone::Column { last: true });
        assert_eq!((from, to), (cols(&[&[&[Palette, Preview]]]), cols(&[&[&[Properties]]])));
    }

    #[test]
    fn room_for_a_column_counts_the_rest() {
        assert!(cols(&[&[&[Palette, Preview, Properties]]]).room_for_column(&RULES, Properties));
        assert!(two().room_for_column(&RULES, Properties));
        assert!(!two().room_for_column(&RULES, Palette));
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
