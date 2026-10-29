//! Find and Replace in a module's text: the localized strings players read
//! (names, descriptions, conversation lines, journal entries, map notes),
//! in every language a string has, and the plain fields builders and
//! scripts read (tags, resrefs, variables' names and string values, the
//! other text fields). Talk-table strings (a StrRef without text of the
//! module's own) aren't the module's to change and are left alone. Text
//! is found as typed, or by a regular expression (`regex-lite`'s).

use mg_core::LocString;
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;
use mg_rules::ObjectKind;
use regex_lite::Regex;

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
    /// Objects' and areas' tags (`Tag`).
    Tag,
    /// Fields that name a resource (a script, a blueprint, a
    /// conversation…), not the resource's own name (`TemplateResRef` of
    /// a blueprint, an area's `ResRef`), which a rename changes.
    ResRef,
    /// Local variables' names and string values.
    Variable,
    /// The other text fields (`CExoString`: a door's `LinkedTo`, a
    /// key's tag, comments…).
    Field,
}

impl TextKind {
    /// Every kind.
    pub const ALL: [TextKind; 9] = [
        TextKind::Name,
        TextKind::Description,
        TextKind::Conversation,
        TextKind::Journal,
        TextKind::Other,
        TextKind::Tag,
        TextKind::ResRef,
        TextKind::Variable,
        TextKind::Field,
    ];

    /// The strings players read (localized strings): what is looked in
    /// unless other kinds are asked for.
    pub const PLAYERS: [TextKind; 5] = [
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
            TextKind::Tag => "Tags",
            TextKind::ResRef => "ResRefs",
            TextKind::Variable => "Variables",
            TextKind::Field => "Other fields",
        }
    }

    /// For the command line: `names`, `tags`, `resrefs`, `fields`…
    pub fn name(self) -> &'static str {
        match self {
            TextKind::Name => "names",
            TextKind::Description => "descriptions",
            TextKind::Conversation => "conversations",
            TextKind::Journal => "journal",
            TextKind::Other => "other",
            TextKind::Tag => "tags",
            TextKind::ResRef => "resrefs",
            TextKind::Variable => "variables",
            TextKind::Field => "fields",
        }
    }

    /// Whether it is one of the strings players read.
    pub fn players(self) -> bool {
        TextKind::PLAYERS.contains(&self)
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

    /// The kind of a plain field (a string or a resref), where Find and
    /// Replace looks in it: `path` the struct it is in, `label` its own.
    /// None for a resource's own name.
    fn of_plain(key: ResKey, path: &[Step], label: &str, v: &Value) -> Option<TextKind> {
        let in_vars = matches!(path.last(), Some(Step::Item(l, _)) if l == "VarTable");
        match v {
            Value::String(_) if in_vars && matches!(label, "Name" | "Value") => {
                Some(TextKind::Variable)
            }
            Value::String(_) if label == "Tag" => Some(TextKind::Tag),
            Value::String(_) => Some(TextKind::Field),
            Value::ResRef(_) if path.is_empty() && own_name(key, label) => None,
            Value::ResRef(_) => Some(TextKind::ResRef),
            _ => None,
        }
    }
}

/// Whether a root field names the resource itself (a blueprint's
/// `TemplateResRef`, a store's `ResRef`, an area's `ResRef`).
fn own_name(key: ResKey, label: &str) -> bool {
    match ObjectKind::from_restype(key.restype) {
        Some(kind) => kind.resref_field() == label,
        None => key.restype == mg_core::ResType::ARE && label == "ResRef",
    }
}

/// How text is matched.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    /// Off, case and accents are set aside: `arbol` finds `Árbol` (by a
    /// regular expression, case only, of A to Z).
    pub match_case: bool,
    /// The text must not run on into letters, digits or `_` either side.
    pub whole_word: bool,
    /// What is looked for is a regular expression (`regex-lite`'s: `\d`,
    /// `[a-z]+`, `(a|b)`, `^…$`), and the text it is replaced with may
    /// name its groups (`$1`, `${name}`).
    pub regex: bool,
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

/// What is looked for, made once for all the strings looked in.
#[derive(Debug, Clone)]
pub struct Finder {
    query: String,
    regex: Option<Regex>,
    o: Options,
}

impl Finder {
    /// `query` as `o` says to match it: `Err` saying why a regular
    /// expression isn't one.
    pub fn new(query: &str, o: Options) -> Result<Finder, String> {
        let regex = match o.regex {
            false => None,
            true => {
                let flags = if o.match_case { "" } else { "(?i)" };
                Some(Regex::new(&format!("{flags}{query}")).map_err(|e| e.to_string())?)
            }
        };
        Ok(Finder { query: query.to_string(), regex, o })
    }

    /// Where it is in `text`: byte ranges, left to right, not
    /// overlapping (a regular expression's empty matches left out).
    pub fn find(&self, text: &str) -> Vec<std::ops::Range<usize>> {
        let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        let whole = |r: &std::ops::Range<usize>| {
            !self.o.whole_word
                || !(word(text[..r.start].chars().next_back())
                    || word(text[r.end..].chars().next()))
        };
        if let Some(re) = &self.regex {
            return re
                .find_iter(text)
                .map(|m| m.range())
                .filter(|r| !r.is_empty() && whole(r))
                .collect();
        }
        let query = &self.query;
        if query.is_empty() {
            return Vec::new();
        }
        let same = |a: char, b: char| {
            if self.o.match_case { a == b } else { mg_core::text::same_letter(a, b) }
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
            if !whole(&(start..end)) {
                continue;
            }
            out.push(start..end);
            from = end;
        }
        out
    }

    /// `text` with what it finds replaced by `with` (a regular
    /// expression's groups named in it, `$1`, put in), and how many times.
    pub fn replace(&self, text: &str, with: &str) -> (String, usize) {
        let found = self.find(text);
        let mut out = String::with_capacity(text.len());
        let mut at = 0;
        for r in &found {
            out.push_str(&text[at..r.start]);
            match &self.regex {
                Some(re) => {
                    // (The match's own groups, as the expression found it.)
                    let caps = re
                        .captures_at(text, r.start)
                        .filter(|c| c.get(0).is_some_and(|m| m.range() == *r));
                    match caps {
                        Some(caps) => caps.expand(with, &mut out),
                        None => out.push_str(with),
                    }
                }
                None => out.push_str(with),
            }
            at = r.end;
        }
        out.push_str(&text[at..]);
        (out, found.len())
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

/// Where `query` is in `text`: byte ranges, left to right, not overlapping
/// (none for a regular expression that isn't one: [`Finder::new`] says
/// why).
pub fn matches(text: &str, query: &str, o: Options) -> Vec<std::ops::Range<usize>> {
    Finder::new(query, o).map_or(Vec::new(), |f| f.find(text))
}

/// `text` with `query` replaced by `with`, and how many times.
pub fn replace_text(text: &str, query: &str, with: &str, o: Options) -> (String, usize) {
    match Finder::new(query, o) {
        Ok(f) => f.replace(text, with),
        Err(_) => (text.to_string(), 0),
    }
}

/// A localized string with `query` replaced by `with` in each variant;
/// `Err` naming a variant whose codepage can't hold the new text.
pub fn replace_locstring(
    ls: &LocString,
    query: &str,
    with: &str,
    o: Options,
) -> Result<LocString, String> {
    replace_locstring_by(ls, &Finder::new(query, o)?, with)
}

fn replace_locstring_by(ls: &LocString, f: &Finder, with: &str) -> Result<LocString, String> {
    let mut out = ls.clone();
    for (key, _) in &ls.strings {
        let (language, gender) = (key.language(), key.gender());
        let game = f.o.codepage(language);
        let Some(text) = ls.text_in(language, gender, game) else { continue };
        let (new, n) = f.replace(&text, with);
        if n > 0 && out.set_text_in(language, gender, &new, game).is_none() {
            return Err(format!(
                "{} can't hold {with:?}",
                language.name().unwrap_or("its language")
            ));
        }
    }
    Ok(out)
}

/// The text of a plain field (a string, a resref) in the module's
/// codepage.
fn plain_text(v: &Value, o: Options) -> Option<String> {
    let cp = o.game.unwrap_or_default();
    match v {
        Value::String(b) | Value::ResRef(b) => Some(cp.decode(b).into_owned()),
        _ => None,
    }
}

/// A field's value with `query` replaced by `with`: a localized string's
/// every variant, a plain string's or resref's text; `Err` saying why it
/// can't hold the new text (a resref of more than 16 characters or not
/// of letters, digits and `_`; a tag of more than 32; a codepage), `None`
/// for a field it isn't in.
pub fn replace_value(
    v: &Value,
    query: &str,
    with: &str,
    o: Options,
) -> Result<Option<Value>, String> {
    replace_value_by(v, &Finder::new(query, o)?, with, "")
}

fn replace_value_by(
    v: &Value,
    f: &Finder,
    with: &str,
    label: &str,
) -> Result<Option<Value>, String> {
    if let Value::LocString(ls) = v {
        return replace_locstring_by(ls, f, with).map(|ls| Some(Value::LocString(ls)));
    }
    let Some(text) = plain_text(v, f.o) else { return Ok(None) };
    let (new, n) = f.replace(&text, with);
    if n == 0 {
        return Ok(None);
    }
    let cp = f.o.game.unwrap_or_default();
    let bytes = (cp.encode(&new).map(std::borrow::Cow::into_owned))
        .ok_or_else(|| format!("{new:?}: the codepage can't hold it"))?;
    match v {
        Value::ResRef(_) => {
            let r = mg_core::ResRef::from_bytes(&bytes)
                .map_err(|e| format!("{new:?} isn't a resource's name: {e}"))?;
            Ok(Some(Value::resref(r)))
        }
        _ if label == "Tag" && new.chars().count() > crate::tags::MAX_TAG => {
            Err(format!("{new:?} is longer than a tag's {} characters", crate::tags::MAX_TAG))
        }
        _ => Ok(Some(Value::String(bytes))),
    }
}

/// Every string of the module with `query` in it, of the kinds asked for,
/// in resource order (none for a regular expression that isn't one:
/// [`Finder::new`] says why).
pub fn find(module: &Module, query: &str, o: Options, kinds: &[TextKind]) -> Vec<Hit> {
    let Ok(f) = Finder::new(query, o) else { return Vec::new() };
    let mut out = Vec::new();
    let mut keys: Vec<ResKey> = module.keys().copied().filter(|k| k.restype.is_gff()).collect();
    keys.sort();
    for key in keys {
        // Palettes are rebuilt from the blueprints.
        if key.restype == mg_core::ResType::ITP {
            continue;
        }
        let Some(Ok(gff)) = module.gff(&key) else { continue };
        walk(&gff, key, &gff.root, &mut Vec::new(), &f, kinds, &mut out);
    }
    out
}

fn walk(
    gff: &Gff,
    key: ResKey,
    s: &Struct,
    path: &mut Vec<Step>,
    f: &Finder,
    kinds: &[TextKind],
    out: &mut Vec<Hit>,
) {
    for field in &s.fields {
        let label = field.label.to_string_lossy();
        let mut hit = |kind: TextKind, text: String, count: usize| {
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
        };
        match &field.value {
            Value::LocString(ls) => {
                let kind = TextKind::of(key, &label);
                if !kinds.contains(&kind) {
                    continue;
                }
                let mut count = 0;
                let mut first = None;
                for (k, _) in &ls.strings {
                    let game = f.o.codepage(k.language());
                    let Some(text) = ls.text_in(k.language(), k.gender(), game) else {
                        continue;
                    };
                    let n = f.find(&text).len();
                    if n > 0 && first.is_none() {
                        first = Some(text.into_owned());
                    }
                    count += n;
                }
                if let Some(text) = first {
                    hit(kind, text, count);
                }
            }
            Value::Struct(c) => {
                path.push(Step::Field(label.clone()));
                walk(gff, key, c, path, f, kinds, out);
                path.pop();
            }
            Value::List(items) => {
                for (i, item) in items.iter().enumerate() {
                    path.push(Step::Item(label.clone(), i));
                    walk(gff, key, item, path, f, kinds, out);
                    path.pop();
                }
            }
            v => {
                let Some(kind) = TextKind::of_plain(key, path, &label, v) else { continue };
                if !kinds.contains(&kind) {
                    continue;
                }
                let Some(text) = plain_text(v, f.o) else { continue };
                let n = f.find(&text).len();
                if n > 0 {
                    hit(kind, text, n);
                }
            }
        }
    }
}

/// Replaces the text in the strings `hits` name, in the module; how many
/// times. A string that can't hold the new text (a language's codepage,
/// a resref's or tag's length) is left as it is and named in the errors.
pub fn replace(
    module: &mut Module,
    hits: &[Hit],
    query: &str,
    with: &str,
    o: Options,
) -> (usize, Vec<String>) {
    let f = match Finder::new(query, o) {
        Ok(f) => f,
        Err(e) => return (0, vec![e]),
    };
    let mut total = 0;
    let mut errors = Vec::new();
    // Each resource once, in the order of its first hit (the hits of one
    // need not be together).
    let mut seen = std::collections::HashSet::new();
    let keys: Vec<ResKey> = hits.iter().map(|h| h.key).filter(|k| seen.insert(*k)).collect();
    for key in keys {
        let Some(Ok(mut gff)) = module.gff(&key) else { continue };
        for hit in hits.iter().filter(|h| h.key == key) {
            let Some(s) = struct_at(&mut gff.root, &hit.path) else { continue };
            let Some(v) = s.get(&hit.label).cloned() else { continue };
            match replace_value_by(&v, &f, with, &hit.label) {
                Ok(Some(new)) => {
                    total += hit.count;
                    s.set(&hit.label, new);
                }
                Ok(None) => {}
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

/// The value a hit's field takes with `query` replaced by `with` (as
/// [`replace`] sets it): `None` where it is already so.
pub fn replaced(
    v: &Value,
    hit: &Hit,
    query: &str,
    with: &str,
    o: Options,
) -> Result<Option<Value>, String> {
    replace_value_by(v, &Finder::new(query, o)?, with, &hit.label)
}
#[cfg(test)]
mod tests {
    use mg_core::{Gender, Language, ResType};

    use super::*;

    fn o(match_case: bool, whole_word: bool) -> Options {
        Options { match_case, whole_word, ..Default::default() }
    }

    #[test]
    fn matching_and_replacing_text() {
        assert_eq!(matches("Orc orcs ORC", "orc", o(false, false)), [0..3, 4..7, 9..12]);
        assert_eq!(matches("Orc orcs ORC", "orc", o(true, false)), vec![4..7]);
        assert_eq!(matches("Orc orcs ORC", "orc", o(false, true)), [0..3, 9..12]);
        assert_eq!(matches("aaaa", "aa", o(false, false)), [0..2, 2..4]);
        assert_eq!(matches("Ærø ærø", "ÆRØ", o(false, false)), [0..5, 6..11]);
        // Without Match case, accents are set aside too; with it, kept.
        assert_eq!(matches("Árbol arbol", "arbol", o(false, false)), [0..6, 7..12]);
        assert_eq!(matches("Árbol arbol", "ÁRBOL", o(false, false)), [0..6, 7..12]);
        assert_eq!(matches("Árbol arbol", "arbol", o(true, false)), vec![7..12]);
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

    #[test]
    fn hits_in_any_order_are_each_replaced_once() {
        let mut m = Module::new();
        let named = |name: &str| {
            let mut g = Gff::new(*b"UTC ");
            let name = LocString::from_text(Language::ENGLISH, Gender::Male, name);
            g.root.set("FirstName", Value::LocString(name.clone()));
            g.root.set("LastName", Value::LocString(name));
            g
        };
        let (a, b) =
            (ResKey::parse("a", ResType::UTC).unwrap(), ResKey::parse("b", ResType::UTC).unwrap());
        m.set_gff(a, &named("cat")).unwrap();
        m.set_gff(b, &named("cat")).unwrap();
        // One resource's hits apart (a list sorted by name or place), and a
        // new text that holds the old.
        let mut hits = find(&m, "cat", o(false, false), &[TextKind::Name]);
        assert_eq!(hits.len(), 4);
        hits.swap(1, 2);
        assert_ne!(hits[0].key, hits[1].key);
        let (n, errors) = replace(&mut m, &hits, "cat", "wildcat", o(false, false));
        assert_eq!((n, errors.len()), (4, 0));
        for key in [a, b] {
            let g = m.gff(&key).unwrap().unwrap();
            for label in ["FirstName", "LastName"] {
                let name = g.root.locstring(label).unwrap();
                assert_eq!(name.text(Language::ENGLISH, Gender::Male).unwrap(), "wildcat");
            }
        }
    }

    fn re(match_case: bool, whole_word: bool) -> Options {
        Options { regex: true, ..o(match_case, whole_word) }
    }

    #[test]
    fn a_regular_expression_finds_and_replaces_with_its_groups() {
        assert_eq!(matches("kz_inn_01 kz_INN_2", r"inn_\d+", re(false, false)), [3..9, 13..18]);
        assert_eq!(matches("kz_inn_01 kz_INN_2", r"inn_\d+", re(true, false)), vec![3..9]);
        // Whole words: not inside a longer one.
        assert_eq!(matches("orc orcs", "orc", re(false, true)), vec![0..3]);
        assert_eq!(
            replace_text("kz_inn_01, kz_well_2", r"kz_(\w+?)_(\d+)", "${1}_$2", re(false, false)),
            ("inn_01, well_2".to_string(), 2)
        );
        // Anchored, and an empty match finds nothing.
        assert_eq!(replace_text("ab ab", "^ab", "x", re(false, false)).0, "x ab");
        assert!(matches("abc", "z*", re(false, false)).is_empty());
        // One that isn't a regular expression finds nothing, and says why.
        assert!(matches("a(b", "a(b", re(false, false)).is_empty());
        assert!(Finder::new("a(b", re(false, false)).is_err());
        assert!(Finder::new("a(b", o(false, false)).is_ok(), "as typed, it is text");
    }

    /// Tags, resrefs, variables' names and string values and the other
    /// text fields are found where asked for, and replaced; a resource's
    /// own name isn't; a resref or a tag too long for its field is left
    /// and said.
    #[test]
    fn plain_fields_are_found_and_replaced_where_asked() {
        let mut m = Module::new();
        let key = ResKey::parse("kz_door", ResType::UTD).unwrap();
        let mut g = Gff::new(*b"UTD ");
        g.root.set("Tag", Value::String(b"KZ_DOOR".to_vec()));
        g.root.set("TemplateResRef", Value::resref(mg_core::ResRef::from_str("kz_door").unwrap()));
        g.root.set("OnOpen", Value::resref(mg_core::ResRef::from_str("kz_door_open").unwrap()));
        g.root.set("LinkedTo", Value::String(b"KZ_DOOR_EXIT".to_vec()));
        g.root.set(
            "LocName",
            Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, "kz door")),
        );
        let mut var = Struct::new(0);
        var.set("Name", Value::String(b"kz_owner".to_vec()));
        var.set("Type", Value::Dword(3));
        var.set("Value", Value::String(b"kz_none".to_vec()));
        g.root.set("VarTable", Value::List(vec![var]));
        m.set_gff(key, &g).unwrap();
        let o = o(false, false);
        let kinds_of = |kinds: &[TextKind]| -> Vec<(TextKind, String)> {
            find(&m, "kz_", o, kinds).into_iter().map(|h| (h.kind, h.path_text())).collect()
        };
        // Players' text only, unless asked.
        assert!(kinds_of(&TextKind::PLAYERS).is_empty(), "\"kz door\" has no kz_");
        assert_eq!(
            kinds_of(&TextKind::ALL),
            [
                (TextKind::Tag, "/Tag".to_string()),
                (TextKind::ResRef, "/OnOpen".to_string()),
                (TextKind::Field, "/LinkedTo".to_string()),
                (TextKind::Variable, "/VarTable[0]/Name".to_string()),
                (TextKind::Variable, "/VarTable[0]/Value".to_string()),
            ],
            "not its own TemplateResRef"
        );
        let hits = find(&m, "kz_", o, &[TextKind::Tag, TextKind::ResRef, TextKind::Variable]);
        let (n, errors) = replace(&mut m, &hits, "kz_", "krezk_", o);
        assert_eq!((n, errors.len()), (4, 0), "{errors:?}");
        let g = m.gff(&key).unwrap().unwrap();
        assert_eq!(g.root.string("Tag"), Some(&b"krezk_DOOR"[..]));
        assert_eq!(g.root.resref("OnOpen").unwrap().to_string(), "krezk_door_open");
        assert_eq!(g.root.string("LinkedTo"), Some(&b"KZ_DOOR_EXIT"[..]), "fields not asked");
        let var = &g.root.list("VarTable").unwrap()[0];
        assert_eq!(
            (var.string("Name"), var.string("Value")),
            (Some(&b"krezk_owner"[..]), Some(&b"krezk_none"[..]))
        );
        // Too long for a resref (16) or a tag (32): left, and said.
        let long = "x".repeat(20);
        let hits = find(&m, "krezk_", o, &[TextKind::Tag, TextKind::ResRef]);
        let (n, errors) = replace(&mut m, &hits, "krezk_", &long, o);
        assert_eq!(n, 1, "the tag: {errors:?}");
        assert!(errors[0].contains("isn't a resource's name"), "{errors:?}");
        let hits = find(&m, "x", o, &[TextKind::Tag]);
        let (n, errors) = replace(&mut m, &hits, "x", "yy", o);
        assert_eq!(n, 0);
        assert!(errors[0].contains("longer than a tag's 32 characters"), "{errors:?}");
    }
}
