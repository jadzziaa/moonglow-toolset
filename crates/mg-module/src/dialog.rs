//! Conversations (`.dlg`): NPC entries and PC replies joined by links, and
//! the Conversation Editor's operations. New conversations and nodes have
//! Aurora's fields, order and defaults (captured under Wine: end scripts
//! `nw_walk_wp`, no delay, looping animation, `NumWords` counting the
//! words of every line).
//!
//! A node's children are links: `(Index, Active, ConditionParams, IsChild)`.
//! `IsChild` 0 means the link owns the node it points to (the tree);
//! 1 means it is a link to a node owned elsewhere (shown grey in Aurora).
//! Starting links (NPC greetings) own their entries.

use std::collections::{HashMap, HashSet};

use mg_core::{Gender, Language, LocString, ResRef};
use mg_gff::{Gff, Struct, Value};
use mg_schema::ExoString;

/// The end-of-conversation script Aurora sets on a new conversation.
pub const END_SCRIPT: &str = "nw_walk_wp";

/// NPC lines (entries) or PC lines (replies).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Entry,
    Reply,
}

impl Kind {
    pub fn list(self) -> &'static str {
        match self {
            Kind::Entry => "EntryList",
            Kind::Reply => "ReplyList",
        }
    }

    /// The label of a node's links to its children.
    pub fn links(self) -> &'static str {
        match self {
            Kind::Entry => "RepliesList",
            Kind::Reply => "EntriesList",
        }
    }

    pub fn child(self) -> Kind {
        match self {
            Kind::Entry => Kind::Reply,
            Kind::Reply => Kind::Entry,
        }
    }
}

/// Where children hang: the root (the starting entries) or a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Parent {
    Root,
    Node(Kind, u32),
}

impl Parent {
    pub fn child_kind(self) -> Kind {
        match self {
            Parent::Root => Kind::Entry,
            Parent::Node(k, _) => k.child(),
        }
    }
}

/// Animations of the Other Actions tab, in Aurora's order: (label, value).
pub const ANIMATIONS: [(&str, u32); 19] = [
    ("Default", 0),
    ("No Animation", 88),
    ("Taunt", 28),
    ("Greeting", 29),
    ("Listen", 30),
    ("Worship", 33),
    ("Salute", 34),
    ("Bow", 35),
    ("Steal", 37),
    ("Talk Normal", 38),
    ("Talk Pleading", 39),
    ("Talk Forceful", 40),
    ("Talk Laughing", 41),
    ("Victory 1", 44),
    ("Victory 2", 45),
    ("Victory 3", 46),
    ("Look Far", 48),
    ("Drink", 70),
    ("Read", 71),
];

fn resref(s: &str) -> Value {
    Value::resref(ResRef::from_str(s).expect("valid resref"))
}

/// A new, empty conversation.
pub fn new_dialog() -> Gff {
    let mut g = Gff::new(*b"DLG ");
    let r = &mut g.root;
    r.set("DelayEntry", Value::Dword(0));
    r.set("DelayReply", Value::Dword(0));
    r.set("NumWords", Value::Dword(0));
    r.set("EndConversation", resref(END_SCRIPT));
    r.set("EndConverAbort", resref(END_SCRIPT));
    r.set("PreventZoomIn", Value::Byte(0));
    r.set("EntryList", Value::List(Vec::new()));
    r.set("ReplyList", Value::List(Vec::new()));
    r.set("StartingList", Value::List(Vec::new()));
    g
}

/// A new line with Aurora's defaults.
pub fn new_node(kind: Kind, text: &str) -> Struct {
    let mut s = Struct::new(0);
    if kind == Kind::Entry {
        s.set("Speaker", Value::String(Vec::new()));
    }
    s.set("Animation", Value::Dword(0));
    s.set("AnimLoop", Value::Byte(1));
    let text = mg_core::Codepage::WINDOWS_1252
        .encode(text)
        .map_or_else(|| text.as_bytes().to_vec(), |b| b.into_owned());
    s.set("Text", Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, text)));
    s.set("Script", resref(""));
    s.set("ActionParams", Value::List(Vec::new()));
    s.set("Delay", Value::Dword(u32::MAX));
    s.set("Comment", Value::String(Vec::new()));
    s.set("Sound", resref(""));
    s.set("Quest", Value::String(Vec::new()));
    s.set(kind.links(), Value::List(Vec::new()));
    s
}

/// A link to node `index`: a starting link, an owning child link, or a
/// link to a node owned elsewhere.
pub fn new_link(index: u32, parent: Parent, is_link: bool) -> Struct {
    let mut s = Struct::new(0);
    s.set("Index", Value::Dword(index));
    s.set("Active", resref(""));
    s.set("ConditionParams", Value::List(Vec::new()));
    if parent != Parent::Root {
        s.set("IsChild", Value::Byte(u8::from(is_link)));
        if is_link {
            s.set("LinkComment", Value::String(Vec::new()));
        }
    }
    s
}

fn list<'a>(g: &'a Gff, label: &str) -> &'a [Struct] {
    g.root.list(label).unwrap_or(&[])
}

fn list_mut<'a>(g: &'a mut Gff, label: &str) -> &'a mut Vec<Struct> {
    if g.root.list(label).is_none() {
        g.root.set(label, Value::List(Vec::new()));
    }
    g.root.list_mut(label).expect("just set")
}

/// The nodes of a kind.
pub fn nodes(g: &Gff, kind: Kind) -> &[Struct] {
    list(g, kind.list())
}

pub fn node(g: &Gff, kind: Kind, index: u32) -> Option<&Struct> {
    nodes(g, kind).get(index as usize)
}

/// A parent's links to its children.
pub fn links(g: &Gff, parent: Parent) -> &[Struct] {
    match parent {
        Parent::Root => list(g, "StartingList"),
        Parent::Node(k, i) => node(g, k, i).and_then(|n| n.list(k.links())).unwrap_or(&[]),
    }
}

fn links_mut(g: &mut Gff, parent: Parent) -> Option<&mut Vec<Struct>> {
    match parent {
        Parent::Root => Some(list_mut(g, "StartingList")),
        Parent::Node(k, i) => {
            let n = list_mut(g, k.list()).get_mut(i as usize)?;
            if n.list(k.links()).is_none() {
                n.set(k.links(), Value::List(Vec::new()));
            }
            n.list_mut(k.links())
        }
    }
}

/// Where a link points.
pub fn link_index(link: &Struct) -> u32 {
    link.dword("Index").unwrap_or(0)
}

/// Whether a link points to a node owned elsewhere.
pub fn is_link(link: &Struct) -> bool {
    link.byte("IsChild") == Some(1)
}

/// Adds a new line under a parent (last), and returns its index.
pub fn add_node(g: &mut Gff, parent: Parent, text: &str) -> u32 {
    let kind = parent.child_kind();
    let nodes = list_mut(g, kind.list());
    nodes.push(new_node(kind, text));
    let index = (nodes.len() - 1) as u32;
    if let Some(l) = links_mut(g, parent) {
        l.push(new_link(index, parent, false));
    }
    renumber(g);
    index
}

/// Adds a link under a parent to an existing line (Paste As Link).
pub fn add_link(g: &mut Gff, parent: Parent, target: u32) -> bool {
    if parent == Parent::Root || node(g, parent.child_kind(), target).is_none() {
        return false;
    }
    let Some(l) = links_mut(g, parent) else { return false };
    l.push(new_link(target, parent, true));
    renumber(g);
    true
}

/// Moves the link at `pos` under `from` to the end of `to`'s children
/// (Aurora's drag): its condition and parameters go with it, and a line it
/// owns moves with its branch. Refused for a parent of the other kind, the
/// same parent, a link (not a line) onto the root, or a line onto its own
/// branch.
pub fn move_link(g: &mut Gff, from: Parent, pos: usize, to: Parent) -> bool {
    let Some(link) = links(g, from).get(pos).cloned() else { return false };
    let kind = from.child_kind();
    if to.child_kind() != kind || to == from {
        return false;
    }
    if let Parent::Node(k, i) = to
        && node(g, k, i).is_none()
    {
        return false;
    }
    let linked = is_link(&link);
    if to == Parent::Root && linked {
        return false;
    }
    if !linked && let Parent::Node(k, i) = to {
        // The branch the line owns, itself included.
        let mut branch = HashSet::new();
        let mut stack = vec![(kind, link_index(&link))];
        while let Some((k, i)) = stack.pop() {
            if branch.insert((k, i)) {
                stack.extend(
                    links(g, Parent::Node(k, i))
                        .iter()
                        .filter(|l| !is_link(l))
                        .map(|l| (k.child(), link_index(l))),
                );
            }
        }
        if branch.contains(&(k, i)) {
            return false;
        }
    }
    let mut moved = new_link(link_index(&link), to, linked);
    for label in ["Active", "ConditionParams", "LinkComment"] {
        if let Some(v) = link.get(label).filter(|_| moved.get(label).is_some()) {
            moved.set(label, v.clone());
        }
    }
    if let Some(l) = links_mut(g, from) {
        l.remove(pos);
    }
    if let Some(l) = links_mut(g, to) {
        l.push(moved);
    }
    renumber(g);
    true
}

/// Gives line `from` a link to line `to` (of the other kind), the way
/// Paste As Link and a link drag do in either direction.
pub fn link_lines(g: &mut Gff, from: (Kind, u32), to: (Kind, u32)) -> bool {
    from.0.child() == to.0 && node(g, from.0, from.1).is_some() && {
        add_link(g, Parent::Node(from.0, from.1), to.1)
    }
}

/// The owning link of a line: its parent and position.
pub fn owner(g: &Gff, kind: Kind, index: u32) -> Option<(Parent, usize)> {
    let parents = std::iter::once(Parent::Root).chain(
        [Kind::Entry, Kind::Reply]
            .into_iter()
            .flat_map(|k| (0..nodes(g, k).len() as u32).map(move |i| Parent::Node(k, i))),
    );
    for p in parents {
        if p.child_kind() != kind {
            continue;
        }
        if let Some(pos) = links(g, p).iter().position(|l| !is_link(l) && link_index(l) == index) {
            return Some((p, pos));
        }
    }
    None
}

/// Every line reachable through owning links from the starting list.
fn owned(g: &Gff) -> HashSet<(Kind, u32)> {
    let mut seen = HashSet::new();
    let mut stack: Vec<(Kind, u32)> =
        links(g, Parent::Root).iter().map(|l| (Kind::Entry, link_index(l))).collect();
    while let Some((k, i)) = stack.pop() {
        if !seen.insert((k, i)) {
            continue;
        }
        for l in links(g, Parent::Node(k, i)) {
            if !is_link(l) {
                stack.push((k.child(), link_index(l)));
            }
        }
    }
    seen
}

/// Removes the link at `pos` under a parent. Removing an owning link
/// deletes the lines it owned (and every link to them); the remaining
/// lines are renumbered.
pub fn remove(g: &mut Gff, parent: Parent, pos: usize) {
    let Some(l) = links_mut(g, parent) else { return };
    if pos >= l.len() {
        return;
    }
    l.remove(pos);
    prune(g);
}

/// Deletes lines no owning link reaches, with the links to them, and
/// renumbers.
fn prune(g: &mut Gff) {
    let keep = owned(g);
    let mut maps: HashMap<Kind, Vec<Option<u32>>> = HashMap::new();
    for kind in [Kind::Entry, Kind::Reply] {
        let mut next = 0;
        let map: Vec<Option<u32>> = (0..nodes(g, kind).len() as u32)
            .map(|i| {
                keep.contains(&(kind, i)).then(|| {
                    next += 1;
                    next - 1
                })
            })
            .collect();
        let kept: Vec<Struct> = nodes(g, kind)
            .iter()
            .enumerate()
            .filter(|(i, _)| map[*i].is_some())
            .map(|(_, s)| s.clone())
            .collect();
        g.root.set(kind.list(), Value::List(kept));
        maps.insert(kind, map);
    }
    let remap = |links: &mut Vec<Struct>, to: Kind| {
        links.retain_mut(|l| {
            let old = link_index(l) as usize;
            match maps[&to].get(old).copied().flatten() {
                Some(new) => {
                    l.set("Index", Value::Dword(new));
                    true
                }
                None => false,
            }
        });
    };
    remap(list_mut(g, "StartingList"), Kind::Entry);
    for kind in [Kind::Entry, Kind::Reply] {
        for n in list_mut(g, kind.list()) {
            if let Some(l) = n.list_mut(kind.links()) {
                remap(l, kind.child());
            }
        }
    }
    renumber(g);
}

/// Struct ids as Aurora writes them: each list's index, and the word count.
fn renumber(g: &mut Gff) {
    fn ids(list: &mut [Struct]) {
        for (i, s) in list.iter_mut().enumerate() {
            s.id = i as u32;
        }
    }
    ids(list_mut(g, "StartingList"));
    for l in list_mut(g, "StartingList") {
        if let Some(p) = l.list_mut("ConditionParams") {
            ids(p);
        }
    }
    for kind in [Kind::Entry, Kind::Reply] {
        let nodes = list_mut(g, kind.list());
        ids(nodes);
        for n in nodes {
            if let Some(p) = n.list_mut("ActionParams") {
                ids(p);
            }
            if let Some(l) = n.list_mut(kind.links()) {
                ids(l);
                for link in l {
                    if let Some(p) = link.list_mut("ConditionParams") {
                        ids(p);
                    }
                }
            }
        }
    }
    let words = word_count(g);
    g.root.set("NumWords", Value::Dword(words));
}

/// The words of every line (the first language's text of each).
pub fn word_count(g: &Gff) -> u32 {
    [Kind::Entry, Kind::Reply]
        .iter()
        .flat_map(|k| nodes(g, *k))
        .filter_map(|n| n.locstring("Text"))
        .map(|t| {
            t.strings.first().map_or(0, |(_, b)| {
                b.split(|c| c.is_ascii_whitespace()).filter(|w| !w.is_empty()).count()
            })
        })
        .sum::<usize>() as u32
}

/// A copied branch: its lines (the first is the top), with links between
/// them in branch-local indices and links out of it by the dialog's index.
#[derive(Debug, Clone, PartialEq)]
pub struct Branch {
    pub kind: Kind,
    /// `(kind, line)`; the line's links are rewritten on paste.
    pub lines: Vec<(Kind, Struct)>,
    /// For each line, its original (kind, index), to remap internal links.
    origin: Vec<(Kind, u32)>,
}

/// Copies the branch a link under a parent points to (following owning
/// links).
pub fn copy_branch(g: &Gff, parent: Parent, pos: usize) -> Option<Branch> {
    let link = links(g, parent).get(pos)?;
    let top = (parent.child_kind(), link_index(link));
    let mut order = Vec::new();
    let mut seen = HashSet::new();
    let mut stack = vec![top];
    while let Some((k, i)) = stack.pop() {
        if !seen.insert((k, i)) {
            continue;
        }
        order.push((k, i));
        for l in links(g, Parent::Node(k, i)).iter().rev() {
            if !is_link(l) {
                stack.push((k.child(), link_index(l)));
            }
        }
    }
    let lines = order.iter().filter_map(|(k, i)| Some((*k, node(g, *k, *i)?.clone()))).collect();
    Some(Branch { kind: top.0, lines, origin: order })
}

/// Pastes a copy of a branch under a parent (Paste). Links inside the branch
/// point to the copies; links out of it keep their target when pasting into
/// the dialog the branch came from, and are dropped otherwise.
pub fn paste_branch(g: &mut Gff, parent: Parent, branch: &Branch, same_dialog: bool) -> bool {
    if branch.kind != parent.child_kind() {
        return false;
    }
    // Where each copied line goes.
    let mut counts: HashMap<Kind, u32> = HashMap::new();
    let placed: Vec<u32> = branch
        .lines
        .iter()
        .map(|(k, _)| {
            let base = nodes(g, *k).len() as u32;
            let c = counts.entry(*k).or_insert(0);
            *c += 1;
            base + *c - 1
        })
        .collect();
    let map: HashMap<(Kind, u32), u32> =
        branch.origin.iter().copied().zip(placed.iter().copied()).collect();
    for (k, line) in &branch.lines {
        let mut s = line.clone();
        if let Some(l) = s.list_mut(k.links()) {
            l.retain_mut(|link| {
                let target = (k.child(), link_index(link));
                match map.get(&target) {
                    Some(new) => {
                        link.set("Index", Value::Dword(*new));
                        true
                    }
                    None => same_dialog && is_link(link),
                }
            });
        }
        list_mut(g, k.list()).push(s);
    }
    if let Some(l) = links_mut(g, parent) {
        l.push(new_link(placed[0], parent, false));
    }
    renumber(g);
    true
}

/// A line's text in English (for display and tests).
pub fn text(n: &Struct) -> String {
    n.locstring("Text")
        .and_then(|t| t.text(Language::ENGLISH, Gender::Male).map(|s| s.into_owned()))
        .unwrap_or_default()
}

/// The conversation as an indented outline, starting links first:
/// `kind|text|link?|condition|params` per line, links not followed. Two
/// conversations with the same outline behave the same, whatever the order
/// of their node lists.
pub fn outline(g: &Gff) -> Vec<String> {
    fn params(s: &Struct, label: &str) -> String {
        s.list(label)
            .unwrap_or(&[])
            .iter()
            .map(|p| {
                let k = String::from_utf8_lossy(p.string("Key").unwrap_or_default()).into_owned();
                let v = String::from_utf8_lossy(p.string("Value").unwrap_or_default()).into_owned();
                format!("{k}={v}")
            })
            .collect::<Vec<_>>()
            .join(",")
    }
    fn walk(g: &Gff, parent: Parent, depth: usize, out: &mut Vec<String>) {
        let kind = parent.child_kind();
        for l in links(g, parent) {
            let i = link_index(l);
            let Some(n) = node(g, kind, i) else { continue };
            let script = n.resref("Script").map(|r| r.to_string()).unwrap_or_default();
            out.push(format!(
                "{}{:?}|{}|{}|if {}({})|do {}({})|anim {}|{}",
                "  ".repeat(depth),
                kind,
                text(n),
                if is_link(l) { "link" } else { "" },
                l.resref("Active").map(|r| r.to_string()).unwrap_or_default(),
                params(l, "ConditionParams"),
                script,
                params(n, "ActionParams"),
                n.dword("Animation").unwrap_or(0),
                String::from_utf8_lossy(n.string("Comment").unwrap_or_default()),
            ));
            if !is_link(l) {
                walk(g, Parent::Node(kind, i), depth + 1, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(g, Parent::Root, 0, &mut out);
    out
}

/// A key/value parameter list (`ConditionParams`, `ActionParams`).
pub fn params_value(pairs: &[(String, String)]) -> Value {
    Value::List(
        pairs
            .iter()
            .enumerate()
            .map(|(i, (k, v))| {
                let mut s = Struct::new(i as u32);
                s.set("Key", Value::String(ExoString::from(k.as_str()).0));
                s.set("Value", Value::String(ExoString::from(v.as_str()).0));
                s
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tree Aurora was driven to build (capture `dialog/capdlg.dlg`).
    fn sample() -> Gff {
        let mut g = new_dialog();
        let hello = add_node(&mut g, Parent::Root, "Hello there.");
        let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
        add_node(&mut g, Parent::Node(Kind::Reply, who), "A traveller.");
        let go = add_node(&mut g, Parent::Root, "Go away.");
        add_link(&mut g, Parent::Node(Kind::Entry, go), who);
        g
    }

    #[test]
    fn lines_move_with_their_branch_and_link_either_way() {
        // Hello (Hi (Again)), Second.
        let mut g = new_dialog();
        let hello = add_node(&mut g, Parent::Root, "Hello");
        let hi = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Hi");
        add_node(&mut g, Parent::Node(Kind::Reply, hi), "Again");
        let second = add_node(&mut g, Parent::Root, "Second");
        // Hi, with its branch, under Second.
        assert!(move_link(
            &mut g,
            Parent::Node(Kind::Entry, hello),
            0,
            Parent::Node(Kind::Entry, second)
        ));
        assert_eq!(
            outline(&g),
            [
                "Entry|Hello||if ()|do ()|anim 0|",
                "Entry|Second||if ()|do ()|anim 0|",
                "  Reply|Hi||if ()|do ()|anim 0|",
                "    Entry|Again||if ()|do ()|anim 0|",
            ]
        );
        // Not onto a line of its own kind, nor into its own branch.
        assert!(!move_link(&mut g, Parent::Node(Kind::Entry, second), 0, Parent::Root));
        assert!(!move_link(&mut g, Parent::Root, 1, Parent::Node(Kind::Reply, hi)));
        // Hi (a reply) gets a link to Hello; not to a reply.
        assert!(link_lines(&mut g, (Kind::Reply, hi), (Kind::Entry, hello)));
        assert!(!link_lines(&mut g, (Kind::Reply, hi), (Kind::Reply, hi)));
        let hi_links = Parent::Node(Kind::Reply, hi);
        let pos = links(&g, hi_links).iter().position(is_link).unwrap();
        assert_eq!(link_index(&links(&g, hi_links)[pos]), hello);
        // A link moves as a link, its comment kept, but not to the root.
        links_mut(&mut g, hi_links).unwrap()[pos]
            .set("LinkComment", Value::String(b"note".to_vec()));
        let bye = add_node(&mut g, Parent::Node(Kind::Entry, second), "Bye");
        assert!(!move_link(&mut g, hi_links, pos, Parent::Root));
        assert!(move_link(&mut g, hi_links, pos, Parent::Node(Kind::Reply, bye)));
        let moved = &links(&g, Parent::Node(Kind::Reply, bye))[0];
        assert!(is_link(moved) && link_index(moved) == hello);
        assert_eq!(moved.string("LinkComment"), Some(&b"note"[..]));
        assert!(links(&g, hi_links).iter().all(|l| !is_link(l)));
    }

    #[test]
    fn building_counts_words_and_links() {
        let g = sample();
        assert_eq!(g.root.dword("NumWords"), Some(9));
        let o = outline(&g);
        assert_eq!(o.len(), 5, "{o:#?}");
        assert!(o[4].starts_with("  Reply|Who are you?|link|"), "{o:#?}");
    }

    #[test]
    fn deleting_an_owning_link_deletes_the_branch_and_links_to_it() {
        let mut g = sample();
        // Remove "Hello there." (the first starting link): its reply and
        // that reply's entry go, and so does the link from "Go away.".
        remove(&mut g, Parent::Root, 0);
        assert_eq!(nodes(&g, Kind::Entry).len(), 1);
        assert_eq!(nodes(&g, Kind::Reply).len(), 0);
        assert_eq!(outline(&g), ["Entry|Go away.||if ()|do ()|anim 0|"]);
        assert_eq!(g.root.dword("NumWords"), Some(2));
    }

    #[test]
    fn owners() {
        let g = sample();
        assert_eq!(owner(&g, Kind::Entry, 0), Some((Parent::Root, 0)));
        assert_eq!(owner(&g, Kind::Reply, 0), Some((Parent::Node(Kind::Entry, 0), 0)));
        assert_eq!(owner(&g, Kind::Entry, 1), Some((Parent::Node(Kind::Reply, 0), 0)));
    }

    #[test]
    fn copy_and_paste_a_branch() {
        let mut g = sample();
        let branch = copy_branch(&g, Parent::Root, 0).unwrap();
        assert_eq!(branch.lines.len(), 3);
        assert!(paste_branch(&mut g, Parent::Root, &branch, true));
        let o = outline(&g);
        assert_eq!(o.len(), 8);
        assert_eq!(o[5..], o[..3], "the copy has the same lines");
        assert_eq!(nodes(&g, Kind::Entry).len(), 5);
        // A reply branch cannot go under the root.
        let reply = copy_branch(&g, Parent::Node(Kind::Entry, 0), 0).unwrap();
        assert!(!paste_branch(&mut g, Parent::Root, &reply, true));
    }
}
