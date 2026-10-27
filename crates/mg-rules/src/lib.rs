//! Game data as the toolset sees it: the resource manager plus talk tables,
//! with 2DA tables parsed once and cached, strings resolved through
//! `dialog.tlk` or the module's custom TLK, and the generic "choices from a
//! 2DA" lists that fill the editors' dropdowns.
//!
//! Accessors for individual tables (appearance, baseitems, item property cost
//! tables, ...) live next to the editors that use them and build on
//! [`GameData::table`] and [`GameData::choices`].

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use mg_2da::TwoDa;
use mg_core::{Codepage, Gender, Language, LocString, ResRef, ResType, StrRef};
use mg_resman::{GameInstall, ResError, ResKey, ResMan};
use mg_tlk::Tlk;
use thiserror::Error;

pub mod appearance;
pub mod challenge;
pub mod creatures;
pub mod item_wizard;
pub mod items;
pub mod levelup;
pub mod names;
pub mod objects;
pub mod placeables;
pub mod rows;
pub mod spell_warnings;
pub use challenge::Challenge;
pub use creatures::{ClassSpells, CreatureSheet, CreatureStats};
pub use items::{ItemProperty, ItemValue};
pub use objects::ObjectKind;
pub use spell_warnings::{SpellSlots, SpellWarning};

#[derive(Debug, Error)]
pub enum RulesError {
    #[error(transparent)]
    Res(#[from] ResError),
    #[error("{name}.2da: {message}")]
    TwoDa { name: String, message: String },
    #[error("{path}: {message}")]
    Tlk { path: String, message: String },
    #[error("{0:?} is not a valid resource name")]
    BadName(String),
}

/// One entry of a dropdown built from a 2DA: the row and the text shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub row: usize,
    pub text: String,
}

/// Choices in the order of their names, whatever their case and accents
/// (`Árboles` beside `arboles`: [`mg_core::text::sort_key`]; rows of one
/// name in the table's order): long lists are found by name, not by where
/// the table has them.
pub fn by_name(mut choices: Vec<Choice>) -> Vec<Choice> {
    choices.sort_by_cached_key(|c| (mg_core::text::sort_key(&c.text), c.row));
    choices
}

/// [`by_name`] for choices named by a path of groups (`Oficios / Herrero`,
/// parts joined by `separator`): by the first group, then within it, so
/// that a group's own come together under it.
pub fn by_path(mut choices: Vec<Choice>, separator: &str) -> Vec<Choice> {
    choices.sort_by_cached_key(|c| {
        let parts: Vec<_> = c.text.split(separator).map(mg_core::text::sort_key).collect();
        (parts, c.row)
    });
    choices
}

/// What a list of choices shows for `row`: its text, else the number in
/// parentheses (a row the list does not have: deleted from the table, or
/// a value no row stands for).
pub fn choice_text(choices: &[Choice], row: i64) -> String {
    choices
        .iter()
        .find(|c| c.row as i64 == row)
        .map_or_else(|| format!("({row})"), |c| c.text.clone())
}

/// Lists of at most this many choices keep the table's order, which means
/// something there (speeds from slow to fast, difficulties from easy to
/// hard); longer ones are shown by name.
pub const ORDERED_CHOICES: usize = 12;

/// Which columns of a 2DA name its rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceColumns<'a> {
    /// A StrRef column with the display name (e.g. `STRING_REF`, `Name`).
    pub name: Option<&'a str>,
    /// A text column used when the name is missing (e.g. `LABEL`).
    pub label: Option<&'a str>,
}

type ChoiceKey = (ResRef, Option<usize>, Option<usize>);

/// What looking up a 2DA found, kept until [`GameData::invalidate`]. (A
/// read that failed on I/O is not kept: it may succeed next time.)
#[derive(Debug, Clone)]
enum Looked {
    Table(Arc<TwoDa>),
    Missing(ResKey),
    Unreadable(String),
}

/// The resource manager, talk tables and caches for one game install (and,
/// once opened, one module).
#[derive(Debug)]
pub struct GameData {
    pub resman: ResMan,
    pub language: Language,
    tlk: Tlk,
    custom_tlk: Option<Tlk>,
    /// Tables looked up so far, found or not: a missing table is asked for
    /// again and again (per candidate feat in a level-up).
    tables: RwLock<HashMap<ResRef, Looked>>,
    /// The choice lists made so far ([`choice_list`](Self::choice_list)),
    /// by table and the two columns' places in it.
    choices: RwLock<HashMap<ChoiceKey, Arc<Vec<Choice>>>>,
    /// The codepage of the game's text, once worked out (see
    /// [`codepage`](Self::codepage)).
    codepage: RwLock<Option<Codepage>>,
}

/// The codepage of an `encoding.2da`: its `Codepoint` column is the
/// Unicode character of each byte, row by row, in hexadecimal (`0x11f`;
/// the game reads `305` as hexadecimal too, and of a number past the
/// Basic Multilingual Plane its low 16 bits). A row that is missing, blank
/// or no number keeps the character of `own` (Windows-1252 where `own` has
/// characters of more than one byte). `None` for a table that is not one,
/// or that changes nothing. (Settled in the game:
/// `mg-corpus-tests/tests/engine_encoding.rs`.)
pub fn encoding_table(table: &TwoDa, own: Codepage) -> Option<Codepage> {
    let column = table.column("Codepoint")?;
    let base = own.chars().or_else(|| Codepage::WINDOWS_1252.chars())?;
    let mut chars = base;
    for (row, c) in chars.iter_mut().enumerate() {
        let Some(cell) = table.cell(row, column) else { continue };
        let cell = cell.trim();
        let hex = cell.strip_prefix("0x").or_else(|| cell.strip_prefix("0X")).unwrap_or(cell);
        let number = u64::from_str_radix(hex, 16).ok().map(|n| (n & 0xFFFF) as u32);
        if let Some(read) = number.and_then(char::from_u32) {
            *c = read;
        }
    }
    (chars != base).then(|| Codepage::table(chars))
}

/// The codepage of the game's text with these resources: `own`, or the
/// table of an `encoding.2da` among them. (From a hak or below, not the
/// module file's own: the game does not read that one.)
pub fn encoding_of(resman: &ResMan, own: Codepage) -> Codepage {
    let key = ResKey::new(ResRef::from_str("encoding").expect("a resref"), ResType::TWODA);
    let layer = (resman.layers().iter())
        .find(|l| l.priority != mg_resman::priority::MODULE && l.container.contains(&key));
    layer
        .and_then(|l| l.container.read(&key).ok())
        // (Read as the language's own text: it is plain numbers.)
        .and_then(|bytes| TwoDa::parse(&bytes, own).ok())
        .and_then(|table| encoding_table(&table, own))
        .unwrap_or(own)
}

/// The column of a name written out in a table, used where a row's
/// talk-table string is blank: ambientmusic.2da has it for music a hak
/// adds.
pub const DISPLAY_NAME: &str = "DisplayName";

impl GameData {
    /// Opens the base stack of an install and its `dialog.tlk`.
    pub fn open(install: &GameInstall) -> Result<GameData, RulesError> {
        let resman = ResMan::for_game(install)?;
        let path = install.talk_table(false);
        let bytes = std::fs::read(&path).map_err(|e| RulesError::Tlk {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;
        let tlk = Tlk::read(&bytes).map_err(|e| RulesError::Tlk {
            path: path.display().to_string(),
            message: e.to_string(),
        })?;
        Ok(GameData::new(resman, tlk))
    }

    /// Wraps an existing resource stack and talk table.
    pub fn new(resman: ResMan, tlk: Tlk) -> GameData {
        GameData {
            resman,
            language: tlk.language,
            tlk,
            custom_tlk: None,
            tables: RwLock::new(HashMap::new()),
            choices: RwLock::new(HashMap::new()),
            codepage: RwLock::new(None),
        }
    }

    /// Sets (or clears) the module's custom talk table.
    pub fn set_custom_tlk(&mut self, tlk: Option<Tlk>) {
        self.custom_tlk = tlk;
        // (The lists name rows by its strings.)
        self.choices.get_mut().expect("choice cache poisoned").clear();
    }

    pub fn custom_tlk(&self) -> Option<&Tlk> {
        self.custom_tlk.as_ref()
    }

    /// `dialog.tlk`.
    pub fn tlk(&self) -> &Tlk {
        &self.tlk
    }

    /// Forgets cached tables (call after the resource stack changes, e.g.
    /// when a module's haks change).
    pub fn invalidate(&self) {
        self.tables.write().expect("table cache poisoned").clear();
        self.choices.write().expect("choice cache poisoned").clear();
        *self.codepage.write().expect("codepage poisoned") = None;
    }

    /// The codepage of the game's text: the language's, or the table of a
    /// module's `encoding.2da` (EE 1.87: the character each of the 256
    /// bytes stands for, shipped with fonts to match, for a language the
    /// game has no codepage for). It is read from the module's haks (or
    /// what lies below them), as the game reads it. Worked out once, and
    /// again after
    /// [`invalidate`](Self::invalidate).
    pub fn codepage(&self) -> Codepage {
        if let Some(c) = *self.codepage.read().expect("codepage poisoned") {
            return c;
        }
        let codepage = encoding_of(&self.resman, self.language.codepage());
        *self.codepage.write().expect("codepage poisoned") = Some(codepage);
        codepage
    }

    /// The codepage of a language's text in this game (see
    /// [`Codepage::for_language`]).
    pub fn codepage_of(&self, language: Language) -> Codepage {
        self.codepage().for_language(language)
    }

    /// A 2DA by name (without extension), parsed once and cached (and a
    /// table missing or unreadable is looked for once, until
    /// [`invalidate`](Self::invalidate)).
    pub fn table(&self, name: &str) -> Result<Arc<TwoDa>, RulesError> {
        let resref = ResRef::from_str(name).map_err(|_| RulesError::BadName(name.into()))?;
        let looked = |l: &Looked| match l {
            Looked::Table(t) => Ok(t.clone()),
            Looked::Missing(key) => Err(RulesError::Res(ResError::NotFound(*key))),
            Looked::Unreadable(message) => {
                Err(RulesError::TwoDa { name: name.into(), message: message.clone() })
            }
        };
        if let Some(l) = self.tables.read().expect("table cache poisoned").get(&resref) {
            return looked(l);
        }
        let key = ResKey::new(resref, ResType::TWODA);
        let found = match self.resman.get(&key) {
            Ok(bytes) => match TwoDa::parse(&bytes, self.codepage()) {
                Ok(table) => Looked::Table(Arc::new(table)),
                Err(e) => Looked::Unreadable(e.to_string()),
            },
            Err(ResError::NotFound(key)) => Looked::Missing(key),
            Err(e) => return Err(e.into()),
        };
        let result = looked(&found);
        self.tables.write().expect("table cache poisoned").insert(resref, found);
        result
    }

    /// The text of a talk-table reference: from the custom TLK if the StrRef
    /// has the custom flag, else from `dialog.tlk`. `None` for
    /// [`StrRef::NONE`], rows past the end and rows without text.
    pub fn string(&self, strref: StrRef) -> Option<String> {
        if strref.is_none() {
            return None;
        }
        let tlk = if strref.is_custom() { self.custom_tlk.as_ref()? } else { &self.tlk };
        tlk.text_in(strref, self.codepage())
    }

    /// The text a localized string shows in the toolset's language: the
    /// embedded variant if there is one with text, else the talk-table
    /// string, else its text in another language (English first), as
    /// Aurora shows a name written in English alone.
    pub fn locstring(&self, s: &LocString) -> Option<String> {
        let game = self.codepage();
        s.text_in(self.language, Gender::Male, game)
            .map(|t| t.into_owned())
            .filter(|t| !t.is_empty())
            .or_else(|| self.string(s.strref).filter(|t| !t.is_empty()))
            .or_else(|| s.elsewhere_in(self.language, game).map(|(_, text)| text))
            // (A variant that is there and empty is still its text.)
            .or_else(|| s.text_in(self.language, Gender::Male, game).map(|t| t.into_owned()))
    }

    /// [`locstring`](Self::locstring)'s text as the bytes it has in the
    /// files (the string's own, or the talk table's): what goes on to
    /// another file is not decoded and encoded again, so a name's color
    /// token keeps every byte, and text kept in another codepage than its
    /// language's stays as it is.
    pub fn locstring_bytes(&self, s: &LocString) -> Option<Vec<u8>> {
        let own = || s.get(self.language, Gender::Male);
        let table = || {
            let tlk = if s.strref.is_custom() { self.custom_tlk.as_ref()? } else { &self.tlk };
            tlk.get(s.strref).map(|e| e.text.as_slice())
        };
        // (As `elsewhere`: English first, else the first with text.)
        let elsewhere = || {
            let english = mg_core::LocStringKey::new(Language::ENGLISH, Gender::Male);
            let mut others =
                s.strings.iter().filter(|(k, b)| k.language() != self.language && !b.is_empty());
            let first = others.clone().find(|(k, _)| *k == english).or_else(|| others.next());
            first.map(|(_, b)| b.as_slice())
        };
        own()
            .filter(|b| !b.is_empty())
            .or_else(|| table().filter(|b| !b.is_empty()))
            .or_else(elsewhere)
            .or_else(own)
            .map(<[u8]>::to_vec)
    }

    /// The rows of a 2DA as dropdown choices: each row's name (a StrRef
    /// column resolved through the talk tables), else its label; rows with
    /// neither are skipped, as the toolset skips blank rows.
    pub fn choices(&self, table: &str, cols: ChoiceColumns<'_>) -> Result<Vec<Choice>, RulesError> {
        self.choice_list(table, cols).map(|c| c.to_vec())
    }

    /// [`choices`](Self::choices) without a copy: the list is made once for
    /// a table and its columns (editors ask for theirs every frame), and
    /// again after the tables are forgotten or the module's talk table is
    /// another.
    pub fn choice_list(
        &self,
        table: &str,
        cols: ChoiceColumns<'_>,
    ) -> Result<Arc<Vec<Choice>>, RulesError> {
        let t = self.table(table)?;
        let name_col = cols.name.and_then(|c| t.column(c));
        let label_col = cols.label.and_then(|c| t.column(c));
        let resref = ResRef::from_str(table).map_err(|_| RulesError::BadName(table.into()))?;
        let key = (resref, name_col, label_col);
        if let Some(made) = self.choices.read().expect("choice cache poisoned").get(&key) {
            return Ok(made.clone());
        }
        // A name written out in the table, for a row without a talk-table
        // string (ambientmusic.2da's `DisplayName`, for custom music).
        let display_col = t.column(DISPLAY_NAME);
        let mut out = Vec::new();
        for row in 0..t.len() {
            let name = name_col
                .and_then(|c| t.cell(row, c))
                .and_then(mg_2da::parse_int)
                .and_then(|v| self.string(StrRef(v as u32)))
                .filter(|s| !s.is_empty());
            let display = || {
                let cell = display_col.and_then(|c| t.cell(row, c)).map(str::trim);
                cell.filter(|v| !v.is_empty() && *v != "****").map(str::to_string)
            };
            let text = name
                .or_else(display)
                .or_else(|| label_col.and_then(|c| t.cell(row, c)).map(str::to_string));
            if let Some(text) = text {
                out.push(Choice { row, text });
            }
        }
        let out = Arc::new(out);
        self.choices.write().expect("choice cache poisoned").insert(key, out.clone());
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use mg_resman::{LayerClass, MemContainer, priority};
    use mg_tlk::TlkEntry;

    use super::*;

    fn data() -> GameData {
        let mut mem = MemContainer::new();
        mem.insert(
            ResKey::parse("things", ResType::TWODA).unwrap(),
            &b"2DA V2.0\n\n  LABEL NAME\n0 Sword 1\n1 **** ****\n2 Axe 99\n3 Custom 16777216\n"[..],
        );
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "mem", LayerClass::Key, mem);
        let mut tlk = Tlk::new(Language::ENGLISH);
        tlk.entries.push(TlkEntry::text("Bad Strref"));
        tlk.entries.push(TlkEntry::text("Longsword"));
        let mut gd = GameData::new(rm, tlk);
        let mut custom = Tlk::new(Language::ENGLISH);
        custom.entries.push(TlkEntry::text("From the module"));
        gd.set_custom_tlk(Some(custom));
        gd
    }

    /// The game data of a talk table in `language` with one line of the
    /// bytes "Da\xf0\xb3", and an `encoding.2da` (byte 0xF0 is "ğ") in a
    /// hak, in the module file, or nowhere.
    fn encoded(language: Language, hak: bool, module: bool) -> GameData {
        // (Row 240 by its place, as the game reads it: 240 rows before.)
        let mut text = String::from("2DA V2.0\n\n  Codepoint\n");
        for row in 0..240 {
            text.push_str(&format!("{row} ****\n"));
        }
        text.push_str("240 0x11f\n");
        let key = ResKey::parse("encoding", ResType::TWODA).unwrap();
        let mut rm = ResMan::new();
        for (on, priority, label) in
            [(hak, priority::HAK_USER, "hak:x"), (module, priority::MODULE, "module")]
        {
            let mut mem = MemContainer::new();
            if on {
                mem.insert(key, text.as_bytes());
            }
            rm.add(priority, label, LayerClass::Erf, mem);
        }
        let mut tlk = Tlk::new(language);
        tlk.entries.push(TlkEntry::text(b"Da\xf0\xb3".to_vec()));
        GameData::new(rm, tlk)
    }

    #[test]
    fn a_hak_s_encoding_table_is_the_game_s_codepage() {
        let name = |text: &[u8]| LocString::from_text(Language::ENGLISH, Gender::Male, text);
        // Without one: the language's own, English and Polish.
        let gd = encoded(Language::ENGLISH, false, false);
        assert_eq!(gd.codepage(), Codepage::WINDOWS_1252);
        assert_eq!(gd.string(StrRef(0)).as_deref(), Some("Dað³"));
        assert_eq!(gd.locstring(&name(b"Da\xf0")).as_deref(), Some("Dað"));
        let gd = encoded(Language::POLISH, false, false);
        assert_eq!(gd.codepage(), Codepage::WINDOWS_1250);
        assert_eq!(gd.string(StrRef(0)).as_deref(), Some("Dađł"));
        // (A name in English is Windows-1252 in a Polish game too.)
        assert_eq!(gd.locstring(&name(b"Da\xf0")).as_deref(), Some("Dað"));
        // In the module file alone it is not read: the game doesn't.
        let gd = encoded(Language::ENGLISH, false, true);
        assert_eq!(gd.codepage(), Codepage::WINDOWS_1252);

        // In a hak: its letter, in the talk table and in a name; the other
        // bytes keep the letters of the game's language.
        let gd = encoded(Language::ENGLISH, true, true);
        assert!(gd.codepage().is_table());
        assert_eq!(gd.string(StrRef(0)).as_deref(), Some("Dağ³"));
        assert_eq!(gd.locstring(&name(b"Da\xf0")).as_deref(), Some("Dağ"));
        assert_eq!(gd.codepage().encode("Dağ").unwrap().as_ref(), b"Da\xf0");
        let gd = encoded(Language::POLISH, true, false);
        assert_eq!(gd.string(StrRef(0)).as_deref(), Some("Dağł"));
        assert_eq!(gd.codepage_of(Language::ENGLISH), gd.codepage());
        // Looked up anew when the layers change.
        let mut gd = encoded(Language::ENGLISH, true, false);
        assert!(gd.codepage().is_table());
        gd.resman.remove("hak:x");
        gd.invalidate();
        assert_eq!(gd.codepage(), Codepage::WINDOWS_1252);
    }

    #[test]
    fn an_encoding_table_s_cells() {
        let own = Codepage::WINDOWS_1252;
        let read = |rows: &str| {
            let text = format!("2DA V2.0\n\n  Codepoint\n{rows}");
            encoding_table(&TwoDa::parse(text.as_bytes(), own).unwrap(), own)
        };
        // Hexadecimal with or without its prefix; a number past 16 bits
        // is its low 16; a blank cell, no number and half of a surrogate
        // pair leave the byte as it is.
        let t = read("0 0x11f\n1 131\n2 0X15E\n3 0x1F600\n4 ****\n5 zz\n6 0xD800\n").unwrap();
        let chars = t.chars().unwrap();
        assert_eq!(chars[..4], ['ğ', 'ı', 'Ş', '\u{f600}']);
        assert_eq!(chars[4..], own.chars().unwrap()[4..]);
        // The game's own letters written out are no table at all, nor is
        // a table of another column.
        assert_eq!(read("0 0x0\n1 0x1\n"), None);
        let other = TwoDa::parse(b"2DA V2.0\n\n  Label\n0 0x11f\n", own).unwrap();
        assert_eq!(encoding_table(&other, own), None);
    }

    /// A table that is not there is looked for once, not on every call (a
    /// level-up asks per candidate feat); invalidating forgets it with the
    /// rest. A table that does not parse is likewise kept as an error.
    #[test]
    fn a_missing_table_is_looked_for_once() {
        let mut gd = data();
        let e = gd.table("later").unwrap_err();
        assert!(matches!(e, RulesError::Res(ResError::NotFound(_))), "{e}");
        let mut mem = MemContainer::new();
        mem.insert(ResKey::parse("later", ResType::TWODA).unwrap(), &b"2DA V2.0\n\n A\n0 1\n"[..]);
        mem.insert(ResKey::parse("broken", ResType::TWODA).unwrap(), &b"not a table"[..]);
        gd.resman.add(priority::KEY, "more", LayerClass::Key, mem);
        assert!(gd.table("later").is_err(), "the miss is kept");
        gd.invalidate();
        assert_eq!(gd.table("later").unwrap().get_int(0, "A"), Some(1));
        for _ in 0..2 {
            let e = gd.table("broken").unwrap_err();
            assert!(matches!(e, RulesError::TwoDa { .. }), "{e}");
        }
        assert_eq!(gd.table("things").unwrap().len(), 4);
    }

    #[test]
    fn strings_resolve_through_both_talk_tables() {
        let gd = data();
        assert_eq!(gd.string(StrRef(1)).as_deref(), Some("Longsword"));
        assert_eq!(gd.string(StrRef::custom(0)).as_deref(), Some("From the module"));
        assert_eq!(gd.string(StrRef(99)), None);
        assert_eq!(gd.string(StrRef::NONE), None);
        let mut ls = LocString::from_strref(StrRef(1));
        assert_eq!(gd.locstring(&ls).as_deref(), Some("Longsword"));
        ls.set_text(Language::ENGLISH, Gender::Male, "Mine");
        assert_eq!(gd.locstring(&ls).as_deref(), Some("Mine"));
    }

    #[test]
    fn choices_are_put_in_their_names_order() {
        let c = |row, text: &str| Choice { row, text: text.into() };
        let sorted =
            by_name(vec![c(0, "Toughness"), c(1, "alertness"), c(2, "Dodge"), c(3, "Dodge")]);
        assert_eq!(sorted, [c(1, "alertness"), c(2, "Dodge"), c(3, "Dodge"), c(0, "Toughness")]);
        // An accented first letter beside its letter, not after z (Sacha's
        // Spanish categories).
        let sorted =
            by_name(vec![c(0, "zarza"), c(1, "Árboles"), c(2, "arbustos"), c(3, "Bosque")]);
        assert_eq!(sorted, [c(1, "Árboles"), c(2, "arbustos"), c(3, "Bosque"), c(0, "zarza")]);
    }

    #[test]
    fn choices_named_by_a_path_are_put_in_order_group_by_group() {
        let c = |row, text: &str| Choice { row, text: text.into() };
        let sorted = by_path(
            vec![
                c(0, "Oficios / Herrero"),
                c(1, "Oficios, viejos"),
                c(2, "Oficios"),
                c(3, "Árboles / Pinos"),
                c(4, "Oficios / Alfarero"),
                c(5, "Zona"),
            ],
            " / ",
        );
        let rows: Vec<usize> = sorted.iter().map(|c| c.row).collect();
        // (By name alone, "Oficios, viejos" came between the group and its
        // own: a comma before the slash.)
        assert_eq!(rows, [3, 2, 4, 0, 1, 5]);
    }

    #[test]
    fn a_choice_shows_its_name_else_its_number() {
        let c = |row, text: &str| Choice { row, text: text.into() };
        let list = [c(0, "None"), c(4, "Dodge")];
        assert_eq!(choice_text(&list, 4), "Dodge");
        assert_eq!(choice_text(&list, 0), "None");
        assert_eq!(choice_text(&list, 2), "(2)", "a row the list lacks");
        assert_eq!(choice_text(&list, -1), "(-1)", "a value no row stands for");
        assert_eq!(choice_text(&[], 7), "(7)");
    }

    #[test]
    fn choices_use_names_then_labels_and_skip_blank_rows() {
        let gd = data();
        let c = gd
            .choices("things", ChoiceColumns { name: Some("name"), label: Some("label") })
            .unwrap();
        let got: Vec<(usize, &str)> = c.iter().map(|c| (c.row, c.text.as_str())).collect();
        assert_eq!(got, [(0, "Longsword"), (2, "Axe"), (3, "From the module")]);
        // Cached: the same Arc comes back.
        assert!(Arc::ptr_eq(&gd.table("things").unwrap(), &gd.table("THINGS").unwrap()));
        assert!(matches!(gd.table("missing"), Err(RulesError::Res(_))));
    }

    #[test]
    fn a_choice_list_is_made_once_and_again_when_what_it_reads_changes() {
        let mut gd = data();
        let cols = ChoiceColumns { name: Some("name"), label: Some("label") };
        let texts = |gd: &GameData, cols| -> Vec<String> {
            gd.choice_list("things", cols).unwrap().iter().map(|c| c.text.clone()).collect()
        };
        let first = gd.choice_list("things", cols).unwrap();
        assert!(Arc::ptr_eq(&first, &gd.choice_list("THINGS", cols).unwrap()));
        assert_eq!(texts(&gd, cols), ["Longsword", "Axe", "From the module"]);
        // Other columns: another list.
        let labels = ChoiceColumns { name: None, label: Some("label") };
        assert_eq!(texts(&gd, labels), ["Sword", "Axe", "Custom"]);
        // Another talk table of the module's: its strings.
        let mut custom = Tlk::new(Language::ENGLISH);
        custom.entries.push(TlkEntry::text("From another"));
        gd.set_custom_tlk(Some(custom));
        assert_eq!(texts(&gd, cols), ["Longsword", "Axe", "From another"]);
        // The table changed (a hak over it): read again once forgotten.
        let mut over = MemContainer::new();
        over.insert(
            ResKey::parse("things", ResType::TWODA).unwrap(),
            &b"2DA V2.0

  LABEL NAME
0 Mace ****
"[..],
        );
        gd.resman.add(priority::HAK_USER, "hak:x", LayerClass::Erf, over);
        assert_eq!(texts(&gd, cols), ["Longsword", "Axe", "From another"], "until forgotten");
        gd.invalidate();
        assert_eq!(texts(&gd, cols), ["Mace"]);
    }
}
