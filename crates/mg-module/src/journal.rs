//! The module's journal (`module.jrl`): categories (quests) with entries,
//! and the Journal Editor's new categories and entries as Aurora makes them
//! (captured under Wine): "Category000" with the same tag, lowest priority,
//! no XP and no picture; entries numbered after the highest ID, with text
//! "Entry001" and so on.

use mg_core::{Gender, Language, LocString};
use mg_gff::{Gff, Struct, Value};
use mg_schema::{ExoString, StructExt, jrl};

/// Priorities as the editor names them (0 highest).
pub const PRIORITIES: [&str; 5] = ["Highest", "High", "Medium", "Low", "Lowest"];

/// "No picture" (the `Picture` field Aurora writes).
pub const NO_PICTURE: u16 = u16::MAX;

/// An empty journal.
pub fn new_journal() -> Gff {
    let mut g = Gff::new(*b"JRL ");
    g.root.items_mut(&jrl::CATEGORIES);
    g
}

fn tag_of(s: &Struct) -> Vec<u8> {
    s.read(&jrl::categories::TAG).0
}

/// A new category after `existing`: "CategoryNNN" (NNN the number of
/// categories, or the next free number) as name and tag.
pub fn new_category(existing: &[Struct]) -> Struct {
    let mut n = existing.len();
    let name = loop {
        let name = format!("Category{n:03}");
        if !existing.iter().any(|c| tag_of(c).eq_ignore_ascii_case(name.as_bytes())) {
            break name;
        }
        n += 1;
    };
    let mut s = Struct::new(existing.len() as u32);
    s.write(
        &jrl::categories::NAME,
        LocString::from_text(Language::ENGLISH, Gender::Male, name.as_bytes().to_vec()),
    );
    s.write(&jrl::categories::XP, 0);
    s.write(&jrl::categories::PRIORITY, 4);
    s.write(&jrl::categories::PICTURE, NO_PICTURE);
    s.write(&jrl::categories::COMMENT, ExoString::from(""));
    s.write(&jrl::categories::TAG, ExoString::from(name.as_str()));
    s.items_mut(&jrl::categories::ENTRY_LIST);
    s
}

/// A new entry after `existing`: the next ID after the highest, text
/// "EntryNNN" with that ID.
pub fn new_entry(existing: &[Struct]) -> Struct {
    let id = existing
        .iter()
        .map(|e| e.read(&jrl::categories::entry_list::ID))
        .max()
        .map_or(1, |m| m + 1);
    let mut s = Struct::new(existing.len() as u32);
    s.write(&jrl::categories::entry_list::ID, id);
    s.write(&jrl::categories::entry_list::END, 0);
    let text = format!("Entry{id:03}");
    s.write(
        &jrl::categories::entry_list::TEXT,
        LocString::from_text(Language::ENGLISH, Gender::Male, text.into_bytes()),
    );
    s
}

/// Gives a list's structs their index as id, as Aurora writes lists.
pub fn renumber(mut list: Vec<Struct>) -> Vec<Struct> {
    for (i, s) in list.iter_mut().enumerate() {
        s.id = i as u32;
    }
    list
}

/// A category's entries.
pub fn entries(category: &Struct) -> &[Struct] {
    category.items(&jrl::categories::ENTRY_LIST)
}

/// A list value for [`mg_gff::Struct::set`].
pub fn list_value(list: Vec<Struct>) -> Value {
    Value::List(renumber(list))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_ids_follow_aurora() {
        let c0 = new_category(&[]);
        assert_eq!(c0.read(&jrl::categories::TAG).as_bytes(), b"Category000");
        let c1 = new_category(std::slice::from_ref(&c0));
        assert_eq!(c1.id, 1);
        assert_eq!(c1.read(&jrl::categories::TAG).as_bytes(), b"Category001");
        // A taken number moves on.
        let taken = new_category(&[c1.clone()]);
        assert_eq!(taken.read(&jrl::categories::TAG).as_bytes(), b"Category002");
        let e1 = new_entry(&[]);
        let mut e5 = new_entry(std::slice::from_ref(&e1));
        e5.write(&jrl::categories::entry_list::ID, 5);
        let next = new_entry(&[e1, e5]);
        assert_eq!(next.read(&jrl::categories::entry_list::ID), 6);
    }
}
