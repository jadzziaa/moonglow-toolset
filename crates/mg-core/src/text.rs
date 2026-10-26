//! Putting names in order as a reader expects them: case and the accents of
//! Latin letters set aside, so that `Árboles` stands beside `arboles`, not
//! after `z` (the game's names are in Western and Central European code
//! pages: Spanish, French, German, Polish, Czech...). No locale's rules
//! beyond that.

use std::cmp::Ordering;

/// `s` in lower case without the accents of its Latin letters, and with
/// the joined letters written out (`æ` as `ae`, `ß` as `ss`): what names
/// are put in order by. Combining marks are dropped, so a decomposed
/// accent folds as a precomposed one does.
pub fn fold(s: &str) -> String {
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

fn is_combining(c: char) -> bool {
    matches!(c, '\u{300}'..='\u{36f}')
}

/// A lower-case Latin letter's letters without its accent (Latin-1 and
/// Latin Extended-A, which the game's Western and Central European code
/// pages draw from, and `ƒ`, `ș` and `ț`).
fn base(c: char) -> Option<&'static str> {
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
        _ => return None,
    })
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
    fn an_accented_name_sorts_beside_its_letter() {
        let mut names = vec!["zarza", "Árboles", "arboles", "Arboles", "Bosque", "álamo"];
        names.sort_by_cached_key(|n| sort_key(n));
        assert_eq!(names, ["álamo", "Arboles", "arboles", "Árboles", "Bosque", "zarza"]);
        // The same order whatever order they came in.
        let mut again = vec!["Árboles", "Bosque", "arboles", "zarza", "álamo", "Arboles"];
        again.sort_by(|a, b| compare(a, b));
        assert_eq!(again, names);
    }
}
