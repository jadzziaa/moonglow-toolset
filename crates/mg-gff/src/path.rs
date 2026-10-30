//! Where something is in a GFF tree: a [`GffPath`] from the root, through
//! struct fields and list items, to a struct or a field.
//!
//! A path has one text form, the one [`Display`](std::fmt::Display) writes
//! and [`FromStr`] reads: the labels from the root, each after a `/`, a
//! list's item as `Label[index]` (`/Creature List[3]/ScriptSpawn`), the root
//! alone as `/`. In a label, `~0`, `~1`, `~2` and `~3` stand for `~`, `/`,
//! `[` and `]`, and `~e` alone for a label that is empty (BioWare's palettes
//! have one), so that every path reads back as it was. Text that is not a
//! path is an error that says why, never a panic.
//!
//! ```
//! use mg_gff::GffPath;
//! let path = GffPath::root().item("Creature List", 3).field("ScriptSpawn");
//! assert_eq!(path.to_string(), "/Creature List[3]/ScriptSpawn");
//! assert_eq!("/Creature List[3]/ScriptSpawn".parse::<GffPath>(), Ok(path));
//! ```

use std::fmt;
use std::str::FromStr;

use crate::Struct;

/// One step into a GFF tree: a struct field, or an item of a list field.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Step {
    Field(String),
    Item(String, usize),
}

impl Step {
    /// The label of the field it steps into (of the list, for an item).
    pub fn label(&self) -> &str {
        match self {
            Step::Field(l) | Step::Item(l, _) => l,
        }
    }

    /// For an item: its list's label and its index.
    pub fn item(&self) -> Option<(&str, usize)> {
        match self {
            Step::Item(l, i) => Some((l, *i)),
            Step::Field(_) => None,
        }
    }
}

/// A path from a GFF's root to a struct inside it, or to a field (whose
/// last step is the field's).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GffPath(pub Vec<Step>);

impl GffPath {
    pub fn root() -> GffPath {
        GffPath(Vec::new())
    }

    /// Whether it is the root's (no steps).
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// This path extended into item `index` of list `label`.
    pub fn item(&self, label: &str, index: usize) -> GffPath {
        let mut p = self.clone();
        p.0.push(Step::Item(label.to_string(), index));
        p
    }

    /// This path extended into struct field `label`.
    pub fn field(&self, label: &str) -> GffPath {
        let mut p = self.clone();
        p.0.push(Step::Field(label.to_string()));
        p
    }

    /// The path without its last step: a field's struct (the root for
    /// the root's own).
    pub fn parent(&self) -> GffPath {
        GffPath(self.0[..self.0.len().saturating_sub(1)].to_vec())
    }

    /// The first step: in an area's GIT, the placed object's
    /// (`Creature List[3]`).
    pub fn first(&self) -> Option<&Step> {
        self.0.first()
    }

    /// The last step's label (a field's own, or its list's for an item);
    /// empty for the root.
    pub fn label(&self) -> &str {
        self.0.last().map_or("", Step::label)
    }

    /// The struct this path leads to from `s`.
    pub fn get<'a>(&self, mut s: &'a Struct) -> Option<&'a Struct> {
        for step in &self.0 {
            s = match step {
                Step::Field(l) => s.child(l)?,
                Step::Item(l, i) => s.list(l)?.get(*i)?,
            };
        }
        Some(s)
    }

    /// The struct this path leads to from `s`, to change.
    pub fn get_mut<'a>(&self, mut s: &'a mut Struct) -> Option<&'a mut Struct> {
        for step in &self.0 {
            s = match step {
                Step::Field(l) => s.child_mut(l)?,
                Step::Item(l, i) => s.list_mut(l)?.get_mut(*i)?,
            };
        }
        Some(s)
    }
}

impl fmt::Display for GffPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("/");
        }
        for step in &self.0 {
            match step {
                Step::Field(l) => write!(f, "/{}", escape(l))?,
                Step::Item(l, i) => write!(f, "/{}[{i}]", escape(l))?,
            }
        }
        Ok(())
    }
}

impl FromStr for GffPath {
    type Err = String;

    /// Reads what `Display` writes: `/` for the root, else `/Label` and
    /// `/Label[index]` steps.
    fn from_str(text: &str) -> Result<GffPath, String> {
        let Some(rest) = text.strip_prefix('/') else {
            return Err(format!("a path starts with /: {text:?}"));
        };
        if rest.is_empty() {
            return Ok(GffPath::root());
        }
        let mut steps = Vec::new();
        for part in rest.split('/') {
            // An item: the label, then [index] at the very end.
            let step = match part.strip_suffix(']').and_then(|p| p.rsplit_once('[')) {
                Some((label, index)) => {
                    let index = index
                        .parse()
                        .map_err(|_| format!("{index:?} in {part:?} is not an item's number"))?;
                    Step::Item(unescape(label)?, index)
                }
                None => Step::Field(unescape(part)?),
            };
            steps.push(step);
        }
        Ok(GffPath(steps))
    }
}

/// A label as a path writes it.
fn escape(label: &str) -> String {
    if label.is_empty() {
        return "~e".into();
    }
    let mut out = String::with_capacity(label.len());
    for c in label.chars() {
        match c {
            '~' => out.push_str("~0"),
            '/' => out.push_str("~1"),
            '[' => out.push_str("~2"),
            ']' => out.push_str("~3"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(text: &str) -> Result<String, String> {
    if text == "~e" {
        return Ok(String::new());
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '~' => out.push(match chars.next() {
                Some('0') => '~',
                Some('1') => '/',
                Some('2') => '[',
                Some('3') => ']',
                other => {
                    let shown = other.map(String::from).unwrap_or_default();
                    return Err(format!("~{shown} is no escape (~0 ~1 ~2 ~3, or ~e alone)"));
                }
            }),
            '[' | ']' => return Err(format!("a stray {c} in {text:?} (write ~2 for [, ~3 for ])")),
            c => out.push(c),
        }
    }
    if out.is_empty() {
        return Err("an empty label is written ~e".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn read(text: &str) -> Result<GffPath, String> {
        text.parse()
    }

    #[test]
    fn a_path_is_written_as_it_reads() {
        assert_eq!(GffPath::root().to_string(), "/");
        assert_eq!(read("/"), Ok(GffPath::root()));
        let p = GffPath::root().item("Creature List", 10).item("ItemList", 0).field("Tag");
        assert_eq!(p.to_string(), "/Creature List[10]/ItemList[0]/Tag");
        assert_eq!(read(&p.to_string()), Ok(p.clone()));
        assert_eq!(p.parent().to_string(), "/Creature List[10]/ItemList[0]");
        assert_eq!(
            (p.label(), p.first().and_then(Step::item)),
            ("Tag", Some(("Creature List", 10)))
        );
        assert_eq!(GffPath::root().parent(), GffPath::root());
        assert_eq!(GffPath::root().label(), "");
    }

    /// A label with the characters a path is written with, or none, is
    /// written escaped, and reads back as it was.
    #[test]
    fn labels_with_a_path_s_characters_are_escaped() {
        let p = GffPath::root().item("a/b", 2).field("[x]").field("~0").field("");
        assert_eq!(p.to_string(), "/a~1b[2]/~2x~3/~00/~e");
        assert_eq!(read(&p.to_string()), Ok(p));
        assert_eq!(read("/~e[1]"), Ok(GffPath::root().item("", 1)));
        // (`~e` stands for an empty label only alone.)
        assert_eq!(read("/x~e"), Err("~e is no escape (~0 ~1 ~2 ~3, or ~e alone)".into()));
    }

    #[test]
    fn text_that_is_no_path_is_an_error() {
        for bad in
            ["", "Tag", "//", "/a//b", "/a/", "/List[x]", "/List[-1]", "/a]b", "/a[b", "/~", "/~9"]
        {
            assert!(read(bad).is_err(), "{bad:?} read as {:?}", read(bad));
        }
        assert_eq!(read("Tag"), Err("a path starts with /: \"Tag\"".into()));
        assert_eq!(read("/List[x]"), Err("\"x\" in \"List[x]\" is not an item's number".into()));
        assert_eq!(read("//"), Err("an empty label is written ~e".into()));
    }

    #[test]
    fn a_path_leads_to_its_struct() {
        let mut item = Struct::new(1);
        item.set("Inner", crate::Value::Struct(Struct::new(2)));
        let mut root = Struct::new(0);
        root.set("List", crate::Value::List(vec![Struct::new(3), item]));
        let at = GffPath::root().item("List", 1).field("Inner");
        assert_eq!(at.get(&root).map(|s| s.id), Some(2));
        assert_eq!(GffPath::root().item("List", 2).get(&root), None);
        assert_eq!(GffPath::root().field("List").get(&root), None, "a list is no struct");
        at.get_mut(&mut root).unwrap().id = 4;
        assert_eq!(at.get(&root).map(|s| s.id), Some(4));
    }

    fn label() -> impl Strategy<Value = String> {
        prop_oneof![Just(String::new()), "[a-zA-Z_ ~/\\[\\]0-9e]{1,16}"]
    }

    fn step() -> impl Strategy<Value = Step> {
        prop_oneof![
            label().prop_map(Step::Field),
            (label(), 0usize..100_000).prop_map(|(l, i)| Step::Item(l, i)),
        ]
    }

    proptest! {
        #[test]
        fn every_path_reads_back(steps in prop::collection::vec(step(), 0..6)) {
            let p = GffPath(steps);
            prop_assert_eq!(read(&p.to_string()), Ok(p));
        }

        /// Any text reads as a path or is an error that says why: never a
        /// panic, and what reads is written back the same way.
        #[test]
        fn any_text_reads_or_is_an_error(text in "/?[a-z~/\\[\\]0-9e]{0,24}") {
            if let Ok(p) = read(&text) {
                prop_assert_eq!(read(&p.to_string()), Ok(p));
            }
        }
    }
}
