//! Two copies of a resource, both made from a third, merged (`mg merge-gff`,
//! git's merge driver): [`mg_schema::merge`], each list by what its items
//! are, and what a conversation needs besides.
//!
//! A conversation's lines are numbered by their place in their list
//! (`EntryList`, `ReplyList`), and its links (`StartingList`, a line's
//! `RepliesList` or `EntriesList`) name them by number. Moonglow's
//! Conversation Editor adds a line at the end of its list, so two builders
//! adding lines both put theirs at the numbers after the base's: theirs
//! are numbered on after ours (and every link of theirs to them with
//! them), and both sides' lines are kept. A line deleted renumbers every
//! line after it, and a side that did that (or put its lines in another
//! order, as Aurora does when it saves) has lines at numbers that were
//! other lines': merged by place with a side that changed lines too, a
//! change would land on the wrong line. Their lines are one conflict
//! (`/(lines)`): the lists of one side or the other, whole.
//!
//! The word count (`NumWords`) both changed is the sum of their changes,
//! and in a conversation whose struct ids and word count are as Aurora
//! writes them (each list's place, the words of every line) the merge's
//! are made so too.

use std::collections::HashMap;

use mg_gff::{Gff, Struct, Value};
pub use mg_schema::merge::{Conflict, Merged, Prefer};

use crate::dialog::{self, Kind, Renumbering};

/// Merges `ours` and `theirs`, both made from `base`: a conflict takes
/// `prefer`'s side.
pub fn merge(base: &Gff, ours: &Gff, theirs: &Gff, prefer: Prefer) -> Merged {
    merge_choosing(base, ours, theirs, &|_| prefer)
}

/// [`merge`], each conflict taking the side `choose` gives for its number
/// (0 for the first): its numbers are the same whichever sides they take.
pub fn merge_choosing(
    base: &Gff,
    ours: &Gff,
    theirs: &Gff,
    choose: &dyn Fn(usize) -> Prefer,
) -> Merged {
    let dlg = |g: &Gff| g.file_type == *b"DLG ";
    if dlg(base) && dlg(ours) && dlg(theirs) {
        conversation(base, ours, theirs, choose)
    } else {
        mg_schema::merge::merge_choosing(base, ours, theirs, choose)
    }
}

/// The lists of a conversation that hold its lines and the links that
/// name them by number.
const LINES: [&str; 3] = ["EntryList", "ReplyList", "StartingList"];

/// The path of the conflict of a conversation's lines.
pub const LINES_CONFLICT: &str = "/(lines)";

fn lines(g: &Gff) -> [Option<&Value>; 3] {
    LINES.map(|l| g.root.get(l))
}

/// The lists of lines, as one value (a conflict's).
fn lines_value(g: &Gff) -> Value {
    let mut s = Struct::new(0);
    for (label, v) in LINES.iter().zip(lines(g)) {
        if let Some(v) = v {
            s.set(label, v.clone());
        }
    }
    Value::Struct(s)
}

fn conversation(base: &Gff, ours: &Gff, theirs: &Gff, choose: &dyn Fn(usize) -> Prefer) -> Merged {
    let (mut o, mut t) = (ours.clone(), theirs.clone());
    let (lb, lo, lt) = (lines(base), lines(ours), lines(theirs));
    let mut first = None;
    if lo != lb && lt != lb && lo != lt {
        if in_place(base, ours) && in_place(base, theirs) {
            number_theirs_after_ours(base, ours, &mut t);
        } else {
            let taken = match choose(0) {
                Prefer::Ours => ours,
                Prefer::Theirs => theirs,
            };
            for (label, v) in LINES.iter().zip(lines(taken)) {
                for side in [&mut o, &mut t] {
                    match v {
                        Some(v) => side.root.set(label, v.clone()),
                        None => {
                            side.root.remove(label);
                        }
                    }
                }
            }
            first = Some(Conflict {
                path: LINES_CONFLICT.into(),
                base: Some(lines_value(base)),
                ours: Some(lines_value(ours)),
                theirs: Some(lines_value(theirs)),
            });
        }
    }
    let words = |g: &Gff| g.root.dword("NumWords");
    if let (Some(wb), Some(wo), Some(wt)) = (words(base), words(ours), words(theirs))
        && wo != wb
        && wt != wb
        && wo != wt
    {
        let sum = (i64::from(wo) + i64::from(wt) - i64::from(wb)).clamp(0, u32::MAX.into());
        for side in [&mut o, &mut t] {
            side.root.set("NumWords", Value::Dword(sum as u32));
        }
    }
    let offset = usize::from(first.is_some());
    let mut m = mg_schema::merge::merge_choosing(base, &o, &t, &|n| choose(n + offset));
    if let Some(c) = first {
        m.conflicts.insert(0, c);
    }
    if as_aurora_writes(ours) && as_aurora_writes(theirs) {
        dialog::renumber(&mut m.gff);
    }
    m
}

/// Whether a conversation's struct ids and word count are those Aurora
/// writes ([`dialog::renumber`] leaves it as it is).
fn as_aurora_writes(g: &Gff) -> bool {
    let mut again = g.clone();
    dialog::renumber(&mut again);
    again == *g
}

/// The numbers of the lines a link list leads to; those that own their
/// line alone (`IsChild` 0, and every starting link) with `owning`.
fn targets(links: &[Struct], owning: bool) -> HashMap<u32, usize> {
    let mut out = HashMap::new();
    for l in links.iter().filter(|l| !owning || !dialog::is_link(l)) {
        *out.entry(dialog::link_index(l)).or_default() += 1;
    }
    out
}

fn covers(side: &HashMap<u32, usize>, base: &HashMap<u32, usize>) -> bool {
    base.iter().all(|(k, n)| side.get(k).is_some_and(|m| m >= n))
}

/// Whether `side` kept every line of `base` at its number: it has as many
/// lines of each kind at least, every link that owns its line is still
/// there (deleting a line takes away the link that owned it, and moving
/// one takes it from its parent), and no line of the base is found at
/// another number by what it is ([`Renumbering::by_lines`]: its text,
/// speaker and those of the lines above it). A side that only added lines
/// at the end and changed lines where they are keeps them; one that
/// deleted or moved lines, or numbered them in another order, doesn't.
fn in_place(base: &Gff, side: &Gff) -> bool {
    for kind in [Kind::Entry, Kind::Reply] {
        let (b, s) = (dialog::nodes(base, kind), dialog::nodes(side, kind));
        if s.len() < b.len() {
            return false;
        }
        for (bn, sn) in b.iter().zip(s) {
            let links = |n: &Struct| targets(n.list(kind.links()).unwrap_or(&[]), true);
            if !covers(&links(sn), &links(bn)) {
                return false;
            }
        }
    }
    let starts = |g: &Gff| targets(g.root.list("StartingList").unwrap_or(&[]), false);
    if !covers(&starts(side), &starts(base)) {
        return false;
    }
    let found = Renumbering::by_lines(base, side);
    [Kind::Entry, Kind::Reply].iter().all(|&kind| {
        (0..dialog::nodes(base, kind).len() as u32)
            .all(|i| found.line(kind, i).is_none_or(|j| j == i))
    })
}

/// Theirs' new lines (those after the base's) numbered on after ours', and
/// their links with them; ours' new lines put in theirs' list at their
/// numbers, so that a merge by place has both. A kind of line only one side
/// added to, or both the same lines, is left as it is.
fn number_theirs_after_ours(base: &Gff, ours: &Gff, theirs: &mut Gff) {
    let mut shift = [0u32; 2];
    let mut from = [0u32; 2];
    for kind in [Kind::Entry, Kind::Reply] {
        let (b, o, t) = (
            dialog::nodes(base, kind).len(),
            dialog::nodes(ours, kind),
            dialog::nodes(theirs, kind),
        );
        if o.len() > b && t.len() > b && o[b..] != t[b..] {
            shift[kind as usize] = (o.len() - b) as u32;
        }
        from[kind as usize] = b as u32;
    }
    if shift == [0, 0] {
        return;
    }
    let renumber = |links: &mut [Struct], to: Kind| {
        let (at, by) = (from[to as usize], shift[to as usize]);
        for l in links {
            let i = dialog::link_index(l);
            if by > 0 && i >= at {
                l.set("Index", Value::Dword(i + by));
            }
        }
    };
    if let Some(starts) = theirs.root.list_mut("StartingList") {
        renumber(starts, Kind::Entry);
    }
    for kind in [Kind::Entry, Kind::Reply] {
        if let Some(nodes) = theirs.root.list_mut(kind.list()) {
            for n in nodes {
                if let Some(links) = n.list_mut(kind.links()) {
                    renumber(links, kind.child());
                }
            }
        }
    }
    for kind in [Kind::Entry, Kind::Reply] {
        let by = shift[kind as usize];
        let at = from[kind as usize] as usize;
        if by == 0 {
            continue;
        }
        let ours_new = &dialog::nodes(ours, kind)[at..];
        let Some(nodes) = theirs.root.list_mut(kind.list()) else { continue };
        let mut theirs_new = nodes.split_off(at);
        for (j, n) in theirs_new.iter_mut().enumerate() {
            // (A struct id that was its place goes with it.)
            if n.id as usize == at + j {
                n.id += by;
            }
        }
        nodes.extend(ours_new.iter().cloned());
        nodes.append(&mut theirs_new);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::{Parent, add_link, add_node, new_dialog, nodes, remove};

    /// Hello (Who are you? (A traveller.)), Go away. (a link to Who are you?)
    fn sample() -> Gff {
        let mut g = new_dialog();
        let hello = add_node(&mut g, Parent::Root, "Hello there.");
        let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
        add_node(&mut g, Parent::Node(Kind::Reply, who), "A traveller.");
        let go = add_node(&mut g, Parent::Root, "Go away.");
        add_link(&mut g, Parent::Node(Kind::Entry, go), who);
        g
    }

    fn texts(g: &Gff, kind: Kind) -> Vec<String> {
        nodes(g, kind).iter().map(dialog::text).collect()
    }

    /// Each side's lines added, theirs numbered after ours: the merge is
    /// ours with theirs' lines added after, as the editor would have
    /// numbered them (and linked them, a link of theirs to a line of their
    /// own included), the word count and struct ids too.
    #[test]
    fn lines_both_sides_added_are_both_kept() {
        let base = sample();
        let ours_adds = |g: &mut Gff| {
            let r = add_node(g, Parent::Node(Kind::Entry, 0), "Just passing.");
            add_node(g, Parent::Node(Kind::Reply, r), "Then pass.");
        };
        let theirs_adds = |g: &mut Gff| {
            let e = add_node(g, Parent::Root, "Halt!");
            let r = add_node(g, Parent::Node(Kind::Entry, e), "Why?");
            add_node(g, Parent::Node(Kind::Reply, 0), "A bard, too.");
            // A link from a line of the base to one of theirs, and one
            // from one of theirs to another.
            add_link(g, Parent::Node(Kind::Entry, 1), r);
            add_link(g, Parent::Node(Kind::Reply, r), e);
        };
        let mut ours = base.clone();
        ours_adds(&mut ours);
        let mut theirs = base.clone();
        theirs_adds(&mut theirs);
        // Theirs' lines added to ours: the editor numbers them after ours'.
        let mut want = ours.clone();
        theirs_adds(&mut want);
        let m = merge(&base, &ours, &theirs, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        assert_eq!(dialog::outline(&m.gff), dialog::outline(&want));
        assert_eq!(m.gff, want);
        // Whichever side is ours.
        let m = merge(&base, &theirs, &ours, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        assert_eq!(texts(&m.gff, Kind::Entry)[3..], ["Halt!", "A bard, too.", "Then pass."]);
        // Lines both added the same (a commit picked onto both) are one.
        let m = merge(&base, &ours, &ours, Prefer::Ours);
        assert_eq!(m.gff, ours);
    }

    /// One side adds lines while the other changes one, and a side adds
    /// lines the other changed nothing of: merged, no conflict.
    #[test]
    fn lines_added_and_lines_changed_merge() {
        let base = sample();
        let mut ours = base.clone();
        add_node(&mut ours, Parent::Node(Kind::Entry, 1), "Leave now.");
        let mut theirs = base.clone();
        dialog::node_mut(&mut theirs, Kind::Entry, 0)
            .unwrap()
            .set("Comment", Value::String(b"hi".to_vec()));
        let m = merge(&base, &ours, &theirs, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        let mut want = ours.clone();
        dialog::node_mut(&mut want, Kind::Entry, 0)
            .unwrap()
            .set("Comment", Value::String(b"hi".to_vec()));
        assert_eq!(m.gff, want);
        let m = merge(&base, &theirs, &ours, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        assert_eq!(m.gff, want);
    }

    /// A side that deleted a line, merged with one that changed lines: one
    /// conflict of the lines, whole, each side's lists taken together;
    /// with a side that changed no line, the deletion is merged.
    #[test]
    fn a_line_deleted_on_one_side_and_lines_changed_on_the_other_is_a_conflict() {
        let base = sample();
        let mut ours = base.clone();
        // "Who are you?" under Hello, with its answer, deleted.
        remove(&mut ours, Parent::Node(Kind::Entry, 0), 0);
        let mut theirs = base.clone();
        let traveller = 1;
        dialog::node_mut(&mut theirs, Kind::Entry, traveller)
            .unwrap()
            .set("Script", Value::resref(mg_core::ResRef::from_str("at_bard").unwrap()));
        theirs.root.set("DelayEntry", Value::Dword(3));
        let m = merge(&base, &ours, &theirs, Prefer::Ours);
        assert_eq!(m.conflicts.len(), 1, "{:?}", m.conflicts);
        assert_eq!(m.conflicts[0].path, LINES_CONFLICT);
        let as_lines = |g: &Gff| lines(g).map(|v| v.cloned());
        assert_eq!(as_lines(&m.gff), as_lines(&ours));
        assert_eq!(m.gff.root.dword("DelayEntry"), Some(3), "what isn't lines merges");
        let m = merge(&base, &ours, &theirs, Prefer::Theirs);
        assert_eq!(as_lines(&m.gff), as_lines(&theirs));
        // The other side changed no line: the deletion is taken.
        let mut delay = base.clone();
        delay.root.set("DelayEntry", Value::Dword(3));
        let m = merge(&base, &ours, &delay, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        assert_eq!(as_lines(&m.gff), as_lines(&ours));
    }

    /// One line owning three answers. Ours deletes the middle one and adds
    /// another under the same line: as many answers, each line still
    /// linked, but the third answer is the second now. Theirs gives the
    /// third answer a script. Merged by place the script went to ours' new
    /// answer; it is a conflict.
    #[test]
    fn a_line_deleted_and_another_added_is_not_taken_for_the_same_line() {
        let mut base = new_dialog();
        let hello = add_node(&mut base, Parent::Root, "Hello.");
        for text in ["One.", "Two.", "Three."] {
            add_node(&mut base, Parent::Node(Kind::Entry, hello), text);
        }
        let mut ours = base.clone();
        remove(&mut ours, Parent::Node(Kind::Entry, hello), 1);
        add_node(&mut ours, Parent::Node(Kind::Entry, hello), "Four.");
        assert_eq!(texts(&ours, Kind::Reply), ["One.", "Three.", "Four."]);
        let mut theirs = base.clone();
        dialog::node_mut(&mut theirs, Kind::Reply, 2)
            .unwrap()
            .set("Script", Value::resref(mg_core::ResRef::from_str("at_three").unwrap()));
        let m = merge(&base, &ours, &theirs, Prefer::Ours);
        assert_eq!(
            m.conflicts.iter().map(|c| c.path.as_str()).collect::<Vec<_>>(),
            [LINES_CONFLICT]
        );
        assert_eq!(m.gff, ours);
    }

    /// Both changed the word count: the sum of their changes (each added
    /// words to a line of the base).
    #[test]
    fn the_word_count_both_changed_adds_up() {
        let base = sample();
        let set_text = |g: &mut Gff, kind, i, text: &str| {
            let n = dialog::node_mut(g, kind, i).unwrap();
            n.set(
                "Text",
                Value::LocString(mg_core::LocString::from_text(
                    mg_core::Language::ENGLISH,
                    mg_core::Gender::Male,
                    text.as_bytes().to_vec(),
                )),
            );
            dialog::recount(g);
        };
        let mut ours = base.clone();
        set_text(&mut ours, Kind::Entry, 0, "Go away, now, please.");
        let mut theirs = base.clone();
        set_text(&mut theirs, Kind::Reply, 0, "Who are you, stranger?");
        let m = merge(&base, &ours, &theirs, Prefer::Ours);
        assert!(m.conflicts.is_empty(), "{:?}", m.conflicts);
        assert_eq!(m.gff.root.dword("NumWords"), Some(dialog::word_count(&m.gff)));
        assert_eq!(dialog::word_count(&m.gff), dialog::word_count(&base) + 2 + 1);
    }
}
