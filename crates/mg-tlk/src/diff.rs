//! Differences between talk tables, by line, and merging two tables that
//! were both made from a third. A line that isn't there and an empty line
//! are the same.

use crate::{Tlk, TlkEntry};

/// A line that differs (`None`: no line, or an empty one).
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    /// The line's number in its table (a custom table's lines are these
    /// plus 16,777,216 as the game names them).
    pub line: u32,
    pub from: Option<TlkEntry>,
    pub to: Option<TlkEntry>,
}

fn entry_ref<'a>(t: &'a Tlk, line: usize, empty: &TlkEntry) -> Option<&'a TlkEntry> {
    t.entries.get(line).filter(|e| *e != empty)
}

/// The lines that differ from `from` to `to`.
pub fn diff(from: &Tlk, to: &Tlk) -> Vec<Change> {
    let empty = TlkEntry::default();
    (0..from.entries.len().max(to.entries.len()))
        .filter_map(|i| {
            let (a, b) = (entry_ref(from, i, &empty), entry_ref(to, i, &empty));
            (a != b).then(|| Change { line: i as u32, from: a.cloned(), to: b.cloned() })
        })
        .collect()
}

/// A line both tables changed, each to something else.
#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    pub line: u32,
    pub base: Option<TlkEntry>,
    pub ours: Option<TlkEntry>,
    pub theirs: Option<TlkEntry>,
}

/// Merges `ours` and `theirs`, both made from `base`: a line either
/// changed is taken; where both changed one, the same change is one, and
/// different changes are a conflict (ours is taken, or theirs with
/// `prefer_theirs`). Two lines added under one number are conflicts: a
/// line's number cannot be given away for it.
pub fn merge(base: &Tlk, ours: &Tlk, theirs: &Tlk, prefer_theirs: bool) -> (Tlk, Vec<Conflict>) {
    let empty = TlkEntry::default();
    let mut merged = Tlk::new(ours.language);
    let mut conflicts = Vec::new();
    let len = ours.entries.len().max(theirs.entries.len());
    merged.entries.reserve(len);
    for i in 0..len {
        let (b, o, t) =
            (entry_ref(base, i, &empty), entry_ref(ours, i, &empty), entry_ref(theirs, i, &empty));
        let taken = if o == b {
            t
        } else if t == b || o == t {
            o
        } else {
            conflicts.push(Conflict {
                line: i as u32,
                base: b.cloned(),
                ours: o.cloned(),
                theirs: t.cloned(),
            });
            if prefer_theirs { t } else { o }
        };
        merged.entries.push(taken.cloned().unwrap_or_default());
    }
    while merged.entries.last().is_some_and(|e| *e == empty) {
        merged.entries.pop();
    }
    (merged, conflicts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::Language;

    fn table(lines: &[&str]) -> Tlk {
        let mut t = Tlk::new(Language::ENGLISH);
        for l in lines {
            t.entries.push(if l.is_empty() { TlkEntry::default() } else { TlkEntry::text(*l) });
        }
        t
    }

    #[test]
    fn lines_differ_by_number() {
        let (a, b) = (table(&["one", "two", ""]), table(&["one", "TWO", "", "", "five"]));
        let d = diff(&a, &b);
        let lines: Vec<u32> = d.iter().map(|c| c.line).collect();
        assert_eq!(lines, [1, 4]);
        assert_eq!(d[0].to.as_ref().unwrap().text, b"TWO");
        assert_eq!(d[1].from, None);
        assert!(diff(&a, &table(&["one", "two"])).is_empty(), "an empty line is no line");
    }

    #[test]
    fn two_tables_made_from_one_are_merged() {
        let base = table(&["one", "two", "three"]);
        let ours = table(&["one", "2", "three", "four"]);
        let theirs = table(&["one", "two", "", "", "five"]);
        let (m, conflicts) = merge(&base, &ours, &theirs, false);
        assert!(conflicts.is_empty());
        assert_eq!(m, table(&["one", "2", "", "four", "five"]));
        // Both gave line 3 a text of their own.
        let theirs = table(&["one", "two", "three", "vier"]);
        let (m, conflicts) = merge(&base, &ours, &theirs, false);
        assert_eq!(conflicts.len(), 1);
        assert_eq!((conflicts[0].line, conflicts[0].base.clone()), (3, None));
        assert_eq!(m.entries[3].text, b"four");
        assert_eq!(merge(&base, &ours, &theirs, true).0.entries[3].text, b"vier");
    }
}
