//! Find and Replace in a module's text: the localized strings players read
//! (names, descriptions, conversation lines, journal entries, map notes),
//! in every language a string has. Talk-table strings (a StrRef without
//! text of the module's own) aren't the module's to change and are left
//! alone.

use mg_core::LocString;
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;

use crate::Module;

/// What kind of text a string is, to search some kinds only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextKind {
    /// Names: objects', areas', the module's.
    Name,
    Description,
    /// Conversation lines.
    Conversation,
    /// Journal categories and entries.
    Journal,
    /// The rest (map notes, …).
    Other,
}

impl TextKind {
    pub const ALL: [TextKind; 5] = [
        TextKind::Name,
        TextKind::Description,
        TextKind::Conversation,
        TextKind::Journal,
        TextKind::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TextKind::Name => "Names",
            TextKind::Description => "Descriptions",
            TextKind::Conversation => "Conversations",
            TextKind::Journal => "Journal",
            TextKind::Other => "Other text",
        }
    }

    fn of(key: ResKey, label: &str) -> TextKind {
        use mg_core::ResType;
        match (key.restype, label) {
            (ResType::DLG, _) => TextKind::Conversation,
            (ResType::JRL, _) => TextKind::Journal,
            (_, "FirstName" | "LastName" | "LocName" | "LocalizedName" | "Name" | "Mod_Name") => {
                TextKind::Name
            }
            (_, "Description" | "DescIdentified" | "Mod_Description") => TextKind::Description,
            _ => TextKind::Other,
        }
    }
}

/// How text is matched.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    pub match_case: bool,
    /// The text must not run on into letters, digits or `_` either side.
    pub whole_word: bool,
    /// The game's codepage, where a module has its own table
    /// (`encoding.2da`); a language's own otherwise.
    pub game: Option<mg_core::Codepage>,
}

impl Options {
    /// The codepage of a language's text.
    fn codepage(&self, language: mg_core::Language) -> mg_core::Codepage {
        self.game.map_or(language.codepage(), |g| g.for_language(language))
    }
}

/// A step from a GFF's root to a struct inside it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Step {
    Field(String),
    Item(String, usize),
}

/// A string that has the text.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub key: ResKey,
    /// The struct holding the string, from the root.
    pub path: Vec<Step>,
    /// The string's field.
    pub label: String,
    /// Where, for people: `keep › creature GUARD › FirstName`.
    pub place: String,
    pub kind: TextKind,
    /// The string as it is (its first variant with the text), and how many
    /// times the text is in it, in all its variants.
    pub text: String,
    pub count: usize,
}

impl Hit {
    /// The path as `/List[3]/Field` (as [`crate::rename::Usage`] has it).
    pub fn path_text(&self) -> String {
        let mut out = String::new();
        for s in &self.path {
            match s {
                Step::Field(l) => out.push_str(&format!("/{l}")),
                Step::Item(l, i) => out.push_str(&format!("/{l}[{i}]")),
            }
        }
        out.push_str(&format!("/{}", self.label));
        out
    }
}

/// Where `query` is in `text`: byte ranges, left to right, not overlapping.
pub fn matches(text: &str, query: &str, o: Options) -> Vec<std::ops::Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    let same = |a: char, b: char| {
        if o.match_case { a == b } else { a == b || a.to_lowercase().eq(b.to_lowercase()) }
    };
    let mut out = Vec::new();
    let mut from = 0;
    'start: for (start, _) in text.char_indices() {
        if start < from {
            continue;
        }
        let mut end = start;
        let mut rest = text[start..].chars();
        for q in query.chars() {
            match rest.next() {
                Some(c) if same(c, q) => end += c.len_utf8(),
                _ => continue 'start,
            }
        }
        if o.whole_word
            && (word(text[..start].chars().next_back()) || word(text[end..].chars().next()))
        {
            continue;
        }
        out.push(start..end);
        from = end;
    }
    out
}

/// `text` with `query` replaced by `with`, and how many times.
pub fn replace_text(text: &str, query: &str, with: &str, o: Options) -> (String, usize) {
    let found = matches(text, query, o);
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for r in &found {
        out.push_str(&text[at..r.start]);
        out.push_str(with);
        at = r.end;
    }
    out.push_str(&text[at..]);
    (out, found.len())
}

/// A localized string with `query` replaced by `with` in each variant;
/// `Err` naming a variant whose codepage can't hold the new text.
pub fn replace_locstring(
    ls: &LocString,
    query: &str,
    with: &str,
    o: Options,
) -> Result<LocString, String> {
    let mut out = ls.clone();
    for (key, _) in &ls.strings {
        let (language, gender) = (key.language(), key.gender());
        let game = o.codepage(language);
        let Some(text) = ls.text_in(language, gender, game) else { continue };
        let (new, n) = replace_text(&text, query, with, o);
        if n > 0 && out.set_text_in(language, gender, &new, game).is_none() {
            return Err(format!(
                "{} can't hold {with:?}",
                language.name().unwrap_or("its language")
            ));
        }
    }
    Ok(out)
}

/// Every string of the module with `query` in it, of the kinds asked for,
/// in resource order.
pub fn find(module: &Module, query: &str, o: Options, kinds: &[TextKind]) -> Vec<Hit> {
    let mut out = Vec::new();
    let mut keys: Vec<ResKey> = module.keys().copied().filter(|k| k.restype.is_gff()).collect();
    keys.sort();
    for key in keys {
        // Palettes are rebuilt from the blueprints.
        if key.restype == mg_core::ResType::ITP {
            continue;
        }
        let Some(Ok(gff)) = module.gff(&key) else { continue };
        walk(&gff, key, &gff.root, &mut Vec::new(), query, o, kinds, &mut out);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn walk(
    gff: &Gff,
    key: ResKey,
    s: &Struct,
    path: &mut Vec<Step>,
    query: &str,
    o: Options,
    kinds: &[TextKind],
    out: &mut Vec<Hit>,
) {
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        match &f.value {
            Value::LocString(ls) => {
                let kind = TextKind::of(key, &label);
                if !kinds.contains(&kind) {
                    continue;
                }
                let mut count = 0;
                let mut first = None;
                for (k, _) in &ls.strings {
                    let game = o.codepage(k.language());
                    let Some(text) = ls.text_in(k.language(), k.gender(), game) else {
                        continue;
                    };
                    let n = matches(&text, query, o).len();
                    if n > 0 && first.is_none() {
                        first = Some(text.into_owned());
                    }
                    count += n;
                }
                if let Some(text) = first {
                    let mut hit = Hit {
                        key,
                        path: path.clone(),
                        label: label.clone(),
                        place: String::new(),
                        kind,
                        text,
                        count,
                    };
                    hit.place = crate::rename::describe(key, Some(gff), &hit.path_text());
                    out.push(hit);
                }
            }
            Value::Struct(c) => {
                path.push(Step::Field(label.clone()));
                walk(gff, key, c, path, query, o, kinds, out);
                path.pop();
            }
            Value::List(items) => {
                for (i, item) in items.iter().enumerate() {
                    path.push(Step::Item(label.clone(), i));
                    walk(gff, key, item, path, query, o, kinds, out);
                    path.pop();
                }
            }
            _ => {}
        }
    }
}

/// Replaces the text in the strings `hits` name, in the module; how many
/// times. A string whose language can't hold the new text is left as it
/// is and named in the errors.
pub fn replace(
    module: &mut Module,
    hits: &[Hit],
    query: &str,
    with: &str,
    o: Options,
) -> (usize, Vec<String>) {
    let mut total = 0;
    let mut errors = Vec::new();
    let mut keys: Vec<ResKey> = hits.iter().map(|h| h.key).collect();
    keys.dedup();
    for key in keys {
        let Some(Ok(mut gff)) = module.gff(&key) else { continue };
        for hit in hits.iter().filter(|h| h.key == key) {
            let Some(s) = struct_at(&mut gff.root, &hit.path) else { continue };
            let Some(ls) = s.locstring(&hit.label).cloned() else { continue };
            match replace_locstring(&ls, query, with, o) {
                Ok(new) => {
                    total += hit.count;
                    s.set(&hit.label, Value::LocString(new));
                }
                Err(e) => errors.push(format!("{}: {e}", hit.place)),
            }
        }
        if let Err(e) = module.set_gff(key, &gff) {
            errors.push(format!("{key}: {e}"));
        }
    }
    (total, errors)
}

/// The struct at a path.
pub fn struct_at<'a>(mut s: &'a mut Struct, path: &[Step]) -> Option<&'a mut Struct> {
    for step in path {
        s = match step {
            Step::Field(l) => s.child_mut(l)?,
            Step::Item(l, i) => s.list_mut(l)?.get_mut(*i)?,
        };
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use mg_core::{Gender, Language, ResType};

    use super::*;

    fn o(match_case: bool, whole_word: bool) -> Options {
        Options { match_case, whole_word, game: None }
    }

    #[test]
    fn matching_and_replacing_text() {
        assert_eq!(matches("Orc orcs ORC", "orc", o(false, false)), [0..3, 4..7, 9..12]);
        assert_eq!(matches("Orc orcs ORC", "orc", o(true, false)), vec![4..7]);
        assert_eq!(matches("Orc orcs ORC", "orc", o(false, true)), [0..3, 9..12]);
        assert_eq!(matches("aaaa", "aa", o(false, false)), [0..2, 2..4]);
        assert_eq!(matches("Ærø ærø", "ÆRØ", o(false, false)), [0..5, 6..11]);
        assert_eq!(
            replace_text("The orc, the Orcs", "orc", "goblin", o(false, true)).0,
            "The goblin, the Orcs"
        );
        assert!(matches("x", "", o(false, false)).is_empty());
    }

    #[test]
    fn finds_and_replaces_in_every_language() {
        let mut m = Module::new();
        let key = ResKey::parse("guard", ResType::UTC).unwrap();
        let mut g = Gff::new(*b"UTC ");
        let mut name = LocString::from_text(Language::ENGLISH, Gender::Male, "Orc Guard");
        name.set_text(Language::FRENCH, Gender::Male, "Garde orc").unwrap();
        g.root.set("FirstName", Value::LocString(name));
        g.root.set(
            "Description",
            Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, "No orcs here")),
        );
        let mut item = Struct::new(0);
        item.set(
            "LocalizedName",
            Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, "Orc axe")),
        );
        g.root.set("ItemList", Value::List(vec![item]));
        // A talk-table string has no text of its own: never found.
        g.root.set("LastName", Value::LocString(LocString::from_strref(mg_core::StrRef(5))));
        m.set_gff(key, &g).unwrap();

        let hits = find(&m, "orc", o(false, true), &[TextKind::Name]);
        assert_eq!(hits.len(), 2);
        assert_eq!(
            (hits[0].label.as_str(), hits[0].count, hits[0].text.as_str()),
            ("FirstName", 2, "Orc Guard")
        );
        assert_eq!(hits[1].path_text(), "/ItemList[0]/LocalizedName");
        let (n, errors) = replace(&mut m, &hits, "orc", "goblin", o(false, true));
        assert_eq!((n, errors.len()), (3, 0));
        let g = m.gff(&key).unwrap().unwrap();
        let first = g.root.locstring("FirstName").unwrap();
        assert_eq!(first.text(Language::ENGLISH, Gender::Male).unwrap(), "goblin Guard");
        assert_eq!(first.text(Language::FRENCH, Gender::Male).unwrap(), "Garde goblin");
        // Descriptions weren't asked for.
        let desc = g.root.locstring("Description").unwrap();
        assert_eq!(desc.text(Language::ENGLISH, Gender::Male).unwrap(), "No orcs here");
        // Text a language's codepage can't hold is refused.
        let ls = LocString::from_text(Language::ENGLISH, Gender::Male, "orc");
        assert!(replace_locstring(&ls, "orc", "орк", o(false, false)).is_err());
    }
}
