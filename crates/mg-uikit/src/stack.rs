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

    /// Each panel once (its first place kept), only those `rules` know,
    /// no empty section or column, at most `rules.most_columns` columns
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
                    if rules.all.contains(&p) && !seen.contains(&p) {
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
