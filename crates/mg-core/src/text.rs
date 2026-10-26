//! Putting names in order and finding them as a reader expects: case and
//! the accents of Latin and Greek letters set aside, so that `Árboles`
//! stands beside `arboles`, not after `z`, and a Filter's `arbol` finds it
//! (the game's names are in Western and Central European code pages:
//! Spanish, French, German, Polish, Czech...; a module's own may be in
//! others). No locale's rules beyond that.

use std::cmp::Ordering;

/// `s` in lower case without the accents of its Latin and Greek letters,
/// and with the joined letters written out (`æ` as `ae`, `ß` as `ss`):
/// what names are put in order and found by. Combining marks are dropped,
/// so a decomposed accent folds as a precomposed one does.
pub fn fold(s: &str) -> String {
    if s.is_ascii() {
        return s.to_ascii_lowercase();
    }
    let mut out = String::with_capacity(s.len());
    for c in s.chars().flat_map(char::to_lowercase) {
        match base(c) {
            Some(b) => out.push_str(b),
            None if is_combining(c) => {}
            None => out.push(c),
        }
    }
    out
}

/// The key names are sorted by: [`fold`]ed first, then the text itself, so
/// that names that fold alike (`Arboles`, `arboles`, `Árboles`) keep one
/// order among themselves whatever order they came in.
pub fn sort_key(s: &str) -> (String, String) {
    (fold(s), s.to_owned())
}

/// Two names in the order of [`sort_key`].
pub fn compare(a: &str, b: &str) -> Ordering {
    fold(a).cmp(&fold(b)).then_with(|| a.cmp(b))
}

/// What is typed in a Filter field, [`fold`]ed once, to find names with:
/// `arbol` finds `Árbol`, `ARBOL` and `arbol`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Needle(String);

impl Needle {
    /// The text typed (the spaces around it aside).
    pub fn new(typed: &str) -> Needle {
        Needle(fold(typed.trim()))
    }

    /// Whether nothing was typed (it finds everything).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The text, folded.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether `text` holds it, case and accents aside.
    pub fn finds(&self, text: &str) -> bool {
        self.0.is_empty() || fold(text).contains(&self.0)
    }

    /// Whether `folded` (a name already [`fold`]ed) holds it.
    pub fn finds_folded(&self, folded: &str) -> bool {
        folded.contains(&self.0)
    }
}

/// Whether `text` holds what was typed as `typed`, case and accents aside:
/// a Filter field's match ([`Needle`], for many names).
pub fn contains(text: &str, typed: &str) -> bool {
    Needle::new(typed).finds(text)
}

/// Whether two characters are one letter, case and accents aside (`é` and
/// `E`): a search's match, character by character, where what is found
/// keeps its length (so a joined letter is only itself: `æ` is not `a`).
pub fn same_letter(a: char, b: char) -> bool {
    if a == b {
        return true;
    }
    let (Some(x), Some(y)) = (single_lower(a), single_lower(b)) else {
        return a.to_lowercase().eq(b.to_lowercase());
    };
    let (mut bx, mut by) = ([0; 4], [0; 4]);
    let x = base(x).unwrap_or_else(|| x.encode_utf8(&mut bx));
    let y = base(y).unwrap_or_else(|| y.encode_utf8(&mut by));
    x == y
}

/// `c` in lower case, where that is one character.
fn single_lower(c: char) -> Option<char> {
    let mut lower = c.to_lowercase();
    let first = lower.next()?;
    lower.next().is_none().then_some(first)
}

fn is_combining(c: char) -> bool {
    matches!(c, '\u{300}'..='\u{36f}')
}

/// A lower-case letter's letters without its accent: Latin-1, Latin
/// Extended-A and B (the game's Western and Central European code pages;
/// Romanian, Pinyin, Croatian's digraphs), Latin Extended Additional
/// (Vietnamese, Welsh) and Greek's tonos and dialytika.
fn base(c: char) -> Option<&'static str> {
    if let '\u{1e00}'..='\u{1eff}' = c {
        return extended_additional(c);
    }
    Some(match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'æ' => "ae",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'ď' | 'đ' | 'ð' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'ƒ' => "f",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'ĥ' | 'ħ' => "h",
        'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'ĳ' => "ij",
        'ĵ' => "j",
        'ķ' | 'ĸ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'ñ' | 'ń' | 'ņ' | 'ň' | 'ŉ' | 'ŋ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
        'œ' => "oe",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'ś' | 'ŝ' | 'ş' | 'š' | 'ș' | 'ſ' => "s",
        'ß' => "ss",
        'ţ' | 'ť' | 'ŧ' | 'ț' => "t",
        'þ' => "th",
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        // Latin Extended-B.
        'ǎ' | 'ǟ' | 'ǡ' | 'ǻ' | 'ȁ' | 'ȃ' | 'ȧ' => "a",
        'ǣ' | 'ǽ' => "ae",
        'ƀ' | 'ƃ' => "b",
        'ƈ' | 'ȼ' => "c",
        'ƌ' | 'ȡ' => "d",
        'ǆ' | 'ǳ' => "dz",
        'ȅ' | 'ȇ' | 'ȩ' | 'ɇ' => "e",
        'ǥ' | 'ǧ' | 'ǵ' => "g",
        'ȟ' => "h",
        'ǐ' | 'ȉ' | 'ȋ' => "i",
        'ǰ' | 'ȷ' | 'ɉ' => "j",
        'ƙ' | 'ǩ' => "k",
        'ƚ' | 'ȴ' => "l",
        'ǉ' => "lj",
        'ƞ' | 'ǹ' | 'ȵ' => "n",
        'ǌ' => "nj",
        'ơ' | 'ǒ' | 'ǫ' | 'ǭ' | 'ǿ' | 'ȍ' | 'ȏ' | 'ȫ' | 'ȭ' | 'ȯ' | 'ȱ' => "o",
        'ƥ' => "p",
        'ɋ' => "q",
        'ȑ' | 'ȓ' | 'ɍ' => "r",
        'ȿ' => "s",
        'ƫ' | 'ƭ' | 'ȶ' => "t",
        'ư' | 'ǔ' | 'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' | 'ȕ' | 'ȗ' => "u",
        'ƴ' | 'ȳ' | 'ɏ' => "y",
        'ƶ' | 'ɀ' => "z",
        // Greek: the tonos and the dialytika, and the final sigma.
        'ά' => "α",
        'έ' => "ε",
        'ή' => "η",
        'ί' | 'ϊ' | 'ΐ' => "ι",
        'ό' => "ο",
        'ύ' | 'ϋ' | 'ΰ' => "υ",
        'ώ' => "ω",
        'ς' => "σ",
        _ => return None,
    })
}

/// A letter of Latin Extended Additional (U+1E00 to U+1EFF) without its
/// marks. The block is in pairs, capital then small, a letter's together
/// and the letters in order: a pair's letter is where it stands.
fn extended_additional(c: char) -> Option<&'static str> {
    // (Where each letter's pairs begin; "": no letter.)
    const RUNS: [(u32, &str); 33] = [
        (0x1e00, "a"),
        (0x1e02, "b"),
        (0x1e08, "c"),
        (0x1e0a, "d"),
        (0x1e14, "e"),
        (0x1e1e, "f"),
        (0x1e20, "g"),
        (0x1e22, "h"),
        (0x1e2c, "i"),
        (0x1e30, "k"),
        (0x1e36, "l"),
        (0x1e3e, "m"),
        (0x1e44, "n"),
        (0x1e4c, "o"),
        (0x1e54, "p"),
        (0x1e58, "r"),
        (0x1e60, "s"),
        (0x1e6a, "t"),
        (0x1e72, "u"),
        (0x1e7c, "v"),
        (0x1e80, "w"),
        (0x1e8a, "x"),
        (0x1e8e, "y"),
        (0x1e90, "z"),
        // ẖ ẗ ẘ ẙ ẚ ẛ, small letters alone (below); ẜ ẝ ẞ ẟ.
        (0x1e96, ""),
        // Vietnamese's vowels with their tones.
        (0x1ea0, "a"),
        (0x1eb8, "e"),
        (0x1ec8, "i"),
        (0x1ecc, "o"),
        (0x1ee4, "u"),
        (0x1ef2, "y"),
        // Middle Welsh's ll and v, the looped y.
        (0x1efa, ""),
        (0x1f00, ""),
    ];
    let n = u32::from(c);
    let alone = ["h", "t", "w", "y", "a", "s"];
    if let Some(letter) = n.checked_sub(0x1e96).and_then(|i| alone.get(i as usize)) {
        return Some(letter);
    }
    let run = RUNS.windows(2).find(|w| w[0].0 <= n && n < w[1].0)?;
    Some(run[0].1).filter(|l| !l.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_and_accents_are_folded() {
        assert_eq!(fold("Árboles"), "arboles");
        assert_eq!(fold("ÑANDÚ"), "nandu");
        assert_eq!(fold("Œuvre Æther Straße"), "oeuvre aether strasse");
        assert_eq!(fold("Łódź Žluťoučký"), "lodz zlutoucky");
        // A decomposed accent (a and a combining acute) as a precomposed one.
        assert_eq!(fold("A\u{301}rboles"), "arboles");
        // What is not a Latin letter with an accent stays.
        assert_eq!(fold("Oficios / Herrero 2"), "oficios / herrero 2");
        assert_eq!(fold("Дом"), "дом");
    }

    #[test]
    fn letters_beyond_latin_1_and_extended_a_are_folded() {
        // Latin Extended-B: Romanian, Pinyin, Croatian's digraphs.
        assert_eq!(fold("Ǎ ǐ ǒ ǔ ǜ Ơ Ư ǅ ǈ ǋ Șt"), "a i o u u o u dz lj nj st");
        // Latin Extended Additional: Vietnamese, Welsh.
        assert_eq!(fold("Tiếng Việt"), "tieng viet");
        assert_eq!(fold("Hồ Chí Minh, Đà Nẵng"), "ho chi minh, da nang");
        assert_eq!(fold("Ỳ ẁ ẃ ẅ ḃ ḋ ṡ ẗ ẞ"), "y w w w b d s t ss");
        // Greek's tonos and dialytika, and the final sigma.
        assert_eq!(fold("Ελληνικά Άλφα ΐ ΰ λόγος"), "ελληνικα αλφα ι υ λογοσ");
        // What has no letter beneath it stays.
        assert_eq!(fold("Ỻ ẜ"), "ỻ ẜ");
    }

    #[test]
    fn an_accented_name_sorts_beside_its_letter() {
        let mut names = vec!["zarza", "Árboles", "arboles", "Arboles", "Bosque", "álamo"];
        names.sort_by_cached_key(|n| sort_key(n));
        assert_eq!(names, ["álamo", "Arboles", "arboles", "Árboles", "Bosque", "zarza"]);
        // The same order whatever order they came in.
        let mut again = vec!["Árboles", "Bosque", "arboles", "zarza", "álamo", "Arboles"];
        again.sort_by(|a, b| compare(a, b));
        assert_eq!(again, names);
    }

    #[test]
    fn a_filter_finds_a_name_case_and_accents_aside() {
        assert!(contains("Árbol viejo", "arbol"));
        assert!(contains("arbol", "ÁRBOL"));
        assert!(contains("Straße", "strasse"));
        assert!(contains("Hồ Chí Minh", " ho chi "));
        assert!(!contains("Árbol", "arboles"));
        let needle = Needle::new("  ");
        assert!(needle.is_empty() && needle.finds("anything"));
        let needle = Needle::new("Élan");
        assert!(needle.finds("ELAN VITAL") && needle.finds_folded(&fold("élan")));
    }

    #[test]
    fn letters_are_the_same_case_and_accents_aside() {
        assert!(same_letter('é', 'E'));
        assert!(same_letter('Ế', 'e'));
        assert!(same_letter('a', 'A'));
        assert!(!same_letter('a', 'b'));
        // A joined letter is only itself, whatever its case.
        assert!(same_letter('Æ', 'æ'));
        assert!(!same_letter('æ', 'a'));
        // A capital whose small letter is two characters: compared whole.
        assert!(!same_letter('İ', 'i'));
    }
}
