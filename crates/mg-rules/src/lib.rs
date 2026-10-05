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
use mg_core::{Gender, Language, LocString, ResRef, ResType, StrRef};
use mg_resman::{GameInstall, ResError, ResKey, ResMan};
use mg_tlk::Tlk;
use thiserror::Error;

pub mod challenge;
pub mod creatures;
pub mod items;
pub mod levelup;
pub mod names;
pub mod rows;
pub mod spell_warnings;
pub use challenge::Challenge;
pub use creatures::{ClassSpells, CreatureSheet, CreatureStats};
pub use items::{ItemProperty, ItemValue};
pub use spell_warnings::SpellWarning;

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

/// Choices in the order of their names (whatever their case; rows of one
/// name in the table's order): long lists are found by name, not by where
/// the table has them.
pub fn by_name(mut choices: Vec<Choice>) -> Vec<Choice> {
    choices.sort_by_cached_key(|c| (c.text.to_lowercase(), c.row));
    choices
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

/// The resource manager, talk tables and caches for one game install (and,
/// once opened, one module).
#[derive(Debug)]
pub struct GameData {
    pub resman: ResMan,
    pub language: Language,
    tlk: Tlk,
    custom_tlk: Option<Tlk>,
    tables: RwLock<HashMap<ResRef, Arc<TwoDa>>>,
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
        }
    }

    /// Sets (or clears) the module's custom talk table.
    pub fn set_custom_tlk(&mut self, tlk: Option<Tlk>) {
        self.custom_tlk = tlk;
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
    }

    /// A 2DA by name (without extension), parsed once and cached.
    pub fn table(&self, name: &str) -> Result<Arc<TwoDa>, RulesError> {
        let resref = ResRef::from_str(name).map_err(|_| RulesError::BadName(name.into()))?;
        if let Some(t) = self.tables.read().expect("table cache poisoned").get(&resref) {
            return Ok(t.clone());
        }
        let bytes = self.resman.get(&ResKey::new(resref, ResType::TWODA))?;
        let table = TwoDa::parse(&bytes, self.language.codepage())
            .map_err(|e| RulesError::TwoDa { name: name.into(), message: e.to_string() })?;
        let table = Arc::new(table);
        self.tables.write().expect("table cache poisoned").insert(resref, table.clone());
        Ok(table)
    }

    /// The text of a talk-table reference: from the custom TLK if the StrRef
    /// has the custom flag, else from `dialog.tlk`. `None` for
    /// [`StrRef::NONE`], rows past the end and rows without text.
    pub fn string(&self, strref: StrRef) -> Option<String> {
        if strref.is_none() {
            return None;
        }
        let tlk = if strref.is_custom() { self.custom_tlk.as_ref()? } else { &self.tlk };
        tlk.text(strref)
    }

    /// The text a localized string shows in the toolset's language: the
    /// embedded variant if there is one, else the talk-table string.
    pub fn locstring(&self, s: &LocString) -> Option<String> {
        s.text(self.language, Gender::Male)
            .map(|t| t.into_owned())
            .or_else(|| self.string(s.strref))
    }

    /// The rows of a 2DA as dropdown choices: each row's name (a StrRef
    /// column resolved through the talk tables), else its label; rows with
    /// neither are skipped, as the toolset skips blank rows.
    pub fn choices(&self, table: &str, cols: ChoiceColumns<'_>) -> Result<Vec<Choice>, RulesError> {
        let t = self.table(table)?;
        let name_col = cols.name.and_then(|c| t.column(c));
        let label_col = cols.label.and_then(|c| t.column(c));
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
}
