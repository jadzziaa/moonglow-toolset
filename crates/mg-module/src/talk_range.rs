//! Text typed into a localized string's field, put in the module's custom
//! talk table as a line of its own (`docs/typed-text-proposal.md`): the
//! lines it may take (the project's `[talk]`: a range, each builder's own
//! part of it, and `mg tlk-alloc`'s map, whose lines are never taken), the
//! next free one, and what the field's string becomes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mg_core::{Gender, Language, LocString, StrRef};

/// The custom table's last line: the game's numbers stop below twice
/// [`StrRef::CUSTOM_FLAG`].
pub const LAST_LINE: u32 = StrRef::CUSTOM_FLAG - 1;

/// Lines `from` to `to` of the custom table, both in (line 0 is StrRef
/// 16,777,216).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lines {
    pub from: u32,
    pub to: u32,
}

impl Lines {
    /// `"100000-149999"` (spaces and an en dash allowed), or a line alone.
    pub fn parse(text: &str) -> Result<Lines, String> {
        let text = text.trim();
        let (a, b) = match text.split_once(['-', '–']) {
            Some((a, b)) => (a.trim(), b.trim()),
            None => (text, text),
        };
        let number = |s: &str| -> Result<u32, String> {
            let n: u32 = s
                .replace('_', "")
                .parse()
                .map_err(|_| format!("\"{text}\" is not lines: \"100000-149999\""))?;
            if n > LAST_LINE {
                return Err(format!("{n} is past the custom table's last line, {LAST_LINE}"));
            }
            Ok(n)
        };
        let lines = Lines { from: number(a)?, to: number(b)? };
        if lines.from > lines.to {
            return Err(format!("\"{text}\": the first line is after the last"));
        }
        Ok(lines)
    }

    pub fn contains(&self, line: u32) -> bool {
        (self.from..=self.to).contains(&line)
    }

    pub fn len(&self) -> u32 {
        self.to - self.from + 1
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn overlaps(&self, other: &Lines) -> bool {
        self.from <= other.to && other.from <= self.to
    }
}

impl std::fmt::Display for Lines {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.from, self.to)
    }
}

/// The project's `[talk]`: where typed text goes in the custom table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TalkRules {
    /// `lines`: the lines typed text takes (none: the table's end).
    pub lines: Option<Lines>,
    /// `map`: `mg tlk-alloc`'s map, by its path from the project's file.
    pub map: Option<String>,
    /// `[talk.builders]`: each builder's own part, by their code.
    pub builders: BTreeMap<String, Lines>,
}

/// The lines a builder may take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Allowed {
    /// The table's end, as Move to Talk Table adds a line.
    End,
    /// The lines of `within`, but those of `but` (the other builders'
    /// parts).
    Range { within: Lines, but: Vec<Lines> },
}

impl Allowed {
    pub fn contains(&self, line: u32) -> bool {
        match self {
            Allowed::End => true,
            Allowed::Range { within, but } => {
                within.contains(line) && !but.iter().any(|b| b.contains(line))
            }
        }
    }

    /// For people: `lines 100000-149999` (`but 100000-109999`), `the
    /// table's end`.
    pub fn describe(&self) -> String {
        match self {
            Allowed::End => "the table's end".into(),
            Allowed::Range { within, but } if but.is_empty() => format!("lines {within}"),
            Allowed::Range { within, but } => {
                let but: Vec<String> = but.iter().map(Lines::to_string).collect();
                format!("lines {within} (but {})", but.join(", "))
            }
        }
    }
}

impl TalkRules {
    /// Whether the project says anything of typed text.
    pub fn is_set(&self) -> bool {
        self.lines.is_some() || !self.builders.is_empty()
    }

    /// The lines the builder of `code` may take: their own part, else
    /// the range but every builder's part, else the table's end.
    pub fn allowed(&self, code: Option<&str>) -> Allowed {
        if let Some(own) = code.and_then(|c| self.builders.get(&c.to_ascii_lowercase())) {
            return Allowed::Range { within: *own, but: Vec::new() };
        }
        match self.lines {
            Some(within) => Allowed::Range {
                within,
                but: self.builders.values().copied().filter(|b| b.overlaps(&within)).collect(),
            },
            None => Allowed::End,
        }
    }

    /// The map's file, from the folder of the project's file.
    pub fn map_path(&self, project_dir: &Path) -> Option<PathBuf> {
        self.map.as_ref().map(|m| project_dir.join(m))
    }

    /// Every line typed text may take, whoever's: what `mg tlk-alloc`
    /// passes over.
    pub fn reserved(&self) -> Vec<Lines> {
        self.lines.iter().chain(self.builders.values()).copied().collect()
    }

    /// What is wrong in the rules themselves: builders' parts that overlap
    /// one another, or reach outside `lines`.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let parts: Vec<(&String, &Lines)> = self.builders.iter().collect();
        for (i, (a, la)) in parts.iter().enumerate() {
            for (b, lb) in &parts[i + 1..] {
                if la.overlaps(lb) {
                    out.push(format!(
                        "the builders {a} ({la}) and {b} ({lb}) share lines: two checkouts could \
                         give one line two texts"
                    ));
                }
            }
            if let Some(all) = self.lines
                && !(all.contains(la.from) && all.contains(la.to))
            {
                out.push(format!("the builder {a}'s lines ({la}) reach outside lines ({all})"));
            }
        }
        out
    }
}

/// The lines of `mg tlk-alloc`'s map (in use and retired), which typed
/// text never takes.
pub fn map_lines(map: &crate::tlk_alloc::Map) -> std::collections::BTreeSet<u32> {
    map.lines.values().chain(map.retired.values()).copied().collect()
}

/// The first free line a builder may take: in `allowed`, past the
/// table's end (`len` lines) or `empty` there, and not `reserved` (the
/// map's, or held for an undone line's redo). `None`: the range is full.
pub fn next_free(
    allowed: &Allowed,
    len: u32,
    empty: impl Fn(u32) -> bool,
    reserved: impl Fn(u32) -> bool,
) -> Option<u32> {
    match allowed {
        Allowed::End => (len..=LAST_LINE).find(|l| !reserved(*l)),
        Allowed::Range { within, .. } => (within.from..=within.to)
            .find(|l| allowed.contains(*l) && (*l >= len || empty(*l)) && !reserved(*l)),
    }
}

/// How many lines a builder may still take, of how many: none for the
/// table's end (as many as the table can have). `reserved`: the map's
/// lines.
pub fn free_lines(
    allowed: &Allowed,
    len: u32,
    empty: impl Fn(u32) -> bool,
    reserved: &std::collections::BTreeSet<u32>,
) -> Option<(u32, u32)> {
    let Allowed::Range { within, but } = allowed else { return None };
    let (mut total, mut free) = (0, 0);
    // (Past the table's end every line is free but the reserved: counted
    // without going through a range of millions one by one.)
    let past = within.from.max(len);
    for l in within.from..past.min(within.to.saturating_add(1)) {
        if allowed.contains(l) {
            total += 1;
            if empty(l) && !reserved.contains(&l) {
                free += 1;
            }
        }
    }
    if past <= within.to {
        let tail = Lines { from: past, to: within.to };
        let taken: u32 = (but.iter())
            .filter(|b| b.overlaps(&tail))
            .map(|b| b.to.min(tail.to) - b.from.max(tail.from) + 1)
            .sum();
        let n = tail.len() - taken;
        let held = reserved.range(tail.from..=tail.to).filter(|l| allowed.contains(**l)).count();
        total += n;
        free += n - (held as u32).min(n);
    }
    Some((free, total))
}

/// The string a field becomes once its text in `language` is the custom
/// table's `line`: that line's StrRef, and its own text in the language
/// (masculine and feminine) taken out, so the game reads the table's.
/// Other languages' text stays (a string with some is kept in the file:
/// [`other_languages`]).
pub fn in_line(ls: &LocString, language: Language, line: u32) -> LocString {
    let mut out = ls.clone();
    out.strref = StrRef(StrRef::CUSTOM_FLAG | line);
    out.remove(language, Gender::Male);
    out.remove(language, Gender::Female);
    out
}

/// Whether a string has text in another language than `language`: the
/// game shows any text of a string's own before its StrRef's line, in the
/// game's language or not (`tests/engine_tlk.rs`), so such a string's
/// text can't be the table's alone.
pub fn other_languages(ls: &LocString, language: Language) -> bool {
    ls.strings.iter().any(|(k, _)| k.language() != language)
}

/// The custom table's line a StrRef names, if it names one.
pub fn line_of(strref: StrRef) -> Option<u32> {
    (!strref.is_none() && strref.0 & StrRef::CUSTOM_FLAG != 0)
        .then_some(strref.0 & !StrRef::CUSTOM_FLAG)
        .filter(|l| *l <= LAST_LINE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(s: &str) -> Lines {
        Lines::parse(s).unwrap()
    }

    #[test]
    fn lines_are_read_and_checked() {
        assert_eq!(lines("100000-149999"), Lines { from: 100_000, to: 149_999 });
        assert_eq!(lines(" 10 – 20 "), Lines { from: 10, to: 20 });
        assert_eq!(lines("7"), Lines { from: 7, to: 7 });
        assert_eq!(lines("1_000-2_000").len(), 1001);
        assert!(Lines::parse("20-10").unwrap_err().contains("after"));
        assert!(Lines::parse("0-16777216").unwrap_err().contains("past"));
        assert!(Lines::parse("a-b").is_err());
        assert!(lines("10-20").overlaps(&lines("20-30")));
        assert!(!lines("10-20").overlaps(&lines("21-30")));
    }

    #[test]
    fn a_builder_takes_their_own_part_else_the_rest() {
        let mut rules = TalkRules { lines: Some(lines("100-199")), ..Default::default() };
        rules.builders.insert("ab".into(), lines("100-149"));
        let own = rules.allowed(Some("AB"));
        assert_eq!(own, Allowed::Range { within: lines("100-149"), but: Vec::new() });
        let other = rules.allowed(Some("cd"));
        assert!(!other.contains(120) && other.contains(150) && !other.contains(200));
        assert_eq!(other.describe(), "lines 100-199 (but 100-149)");
        assert_eq!(rules.allowed(None), other);
        assert_eq!(TalkRules::default().allowed(Some("ab")), Allowed::End);
        assert_eq!(rules.reserved(), [lines("100-199"), lines("100-149")]);
    }

    #[test]
    fn builders_parts_that_overlap_are_said() {
        let mut rules = TalkRules { lines: Some(lines("100-199")), ..Default::default() };
        rules.builders.insert("ab".into(), lines("100-149"));
        rules.builders.insert("cd".into(), lines("140-260"));
        let p = rules.problems();
        assert_eq!(p.len(), 2, "{p:?}");
        assert!(p[0].contains("share lines") && p[1].contains("reach outside"), "{p:?}");
    }

    /// The next free line: past lines with text, the map's and those
    /// reserved, in the builder's lines; the table's end without a range.
    #[test]
    fn the_next_free_line_passes_over_what_is_taken() {
        // Lines 0 to 9; 3 and 5 empty.
        let empty = |l: u32| l == 3 || l == 5;
        let allowed = Allowed::Range { within: lines("2-12"), but: vec![lines("3-3")] };
        assert_eq!(next_free(&allowed, 10, empty, |_| false), Some(5));
        assert_eq!(next_free(&allowed, 10, empty, |l| l == 5), Some(10));
        assert_eq!(next_free(&Allowed::End, 10, empty, |l| l == 10), Some(11));
        let full = Allowed::Range { within: lines("0-4"), but: Vec::new() };
        assert_eq!(next_free(&full, 10, |_| false, |_| false), None);
        // Free of how many: 5 and 10 to 12 (3 is another's).
        let none = std::collections::BTreeSet::new();
        assert_eq!(free_lines(&allowed, 10, empty, &none), Some((4, 10)));
        assert_eq!(free_lines(&Allowed::End, 10, empty, &none), None);
        // A range of millions past the end, counted at once.
        let wide = Allowed::Range { within: lines("0-16777215"), but: vec![lines("100-199")] };
        let map = std::collections::BTreeSet::from([500]);
        assert_eq!(free_lines(&wide, 10, |_| false, &map), Some((16_777_116 - 11, 16_777_116)));
    }

    #[test]
    fn a_string_in_a_line_names_it_and_keeps_other_languages() {
        let mut ls = LocString::from_text(Language::ENGLISH, Gender::Male, "Sword");
        ls.set(Language::ENGLISH, Gender::Female, b"Sword (f)".to_vec());
        ls.set(Language::FRENCH, Gender::Male, b"Epee".to_vec());
        let out = in_line(&ls, Language::ENGLISH, 42);
        assert_eq!(out.strref, StrRef(16_777_258));
        assert_eq!(out.get(Language::ENGLISH, Gender::Male), None);
        assert_eq!(out.get(Language::ENGLISH, Gender::Female), None);
        assert_eq!(out.get(Language::FRENCH, Gender::Male), Some(&b"Epee"[..]));
        assert_eq!(line_of(out.strref), Some(42));
        assert!(other_languages(&out, Language::ENGLISH));
        assert!(!other_languages(&in_line(&ls, Language::FRENCH, 1), Language::ENGLISH));
        assert_eq!(line_of(StrRef(12)), None);
        assert_eq!(line_of(StrRef::NONE), None);
    }
}
