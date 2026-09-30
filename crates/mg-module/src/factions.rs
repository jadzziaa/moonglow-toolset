//! The module's factions (`repute.fac`) and how they regard each other, with
//! the Faction Editor's operations as Aurora performs them (captured under
//! Wine).
//!
//! A reputation entry `(FactionID1, FactionID2, FactionRep)` says how much
//! faction 2 likes faction 1 (0 hostile … 100 friendly). How the PC faction
//! feels is not stored: players' attitudes are their own.

use std::collections::BTreeMap;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;
use mg_schema::{ExoString, StructExt, fac};

use crate::Module;

/// The five factions every module has; they cannot be removed.
pub const STANDARD: [&str; 5] = ["PC", "Hostile", "Commoner", "Merchant", "Defender"];

/// A reputation looked up where the file has none (Aurora's value, which it
/// also writes into the file when it looks one up).
pub const DEFAULT_REPUTATION: u32 = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct Faction {
    pub name: String,
    /// The faction whose reputations this one started from.
    pub parent: Option<u32>,
    /// "Global effect": a change of reputation applies to the whole faction.
    pub global: bool,
    /// The stored struct, so fields Moonglow does not know survive.
    original: Struct,
}

/// A module's factions and their reputations.
#[derive(Debug, Clone, PartialEq)]
pub struct Factions {
    pub factions: Vec<Faction>,
    /// `(target, perceiver)` → how much `perceiver` likes `target`.
    reputations: BTreeMap<(u32, u32), u32>,
    /// Anything else in the file's root.
    root: Struct,
}

fn decode(b: &[u8]) -> String {
    mg_core::Codepage::WINDOWS_1252.decode(b).into_owned()
}

fn encode(s: &str) -> Vec<u8> {
    mg_core::Codepage::WINDOWS_1252
        .encode(s)
        .map_or_else(|| s.as_bytes().to_vec(), |b| b.into_owned())
}

impl Factions {
    pub fn read(g: &Gff) -> Factions {
        let factions = g
            .root
            .items(&fac::FACTION_LIST)
            .iter()
            .map(|s| {
                let parent = s.read(&fac::faction_list::FACTION_PARENT_ID);
                Faction {
                    name: decode(s.read(&fac::faction_list::FACTION_NAME).as_bytes()),
                    parent: (parent != u32::MAX).then_some(parent),
                    global: s.read(&fac::faction_list::FACTION_GLOBAL) != 0,
                    original: s.clone(),
                }
            })
            .collect();
        let reputations = g
            .root
            .items(&fac::REP_LIST)
            .iter()
            .map(|s| {
                let key =
                    (s.read(&fac::rep_list::FACTION_ID1), s.read(&fac::rep_list::FACTION_ID2));
                (key, s.read(&fac::rep_list::FACTION_REP))
            })
            .collect();
        let mut root = g.root.clone();
        root.remove(fac::FACTION_LIST.label);
        root.remove(fac::REP_LIST.label);
        Factions { factions, reputations, root }
    }

    /// The module's factions (the standard ones if it has no `repute.fac`).
    pub fn of_module(m: &Module) -> Factions {
        let key = ResKey::new(ResRef::from_str("repute").expect("valid"), ResType::FAC);
        let g = m.gff(&key).and_then(Result::ok).unwrap_or_else(crate::new::default_factions);
        Factions::read(&g)
    }

    /// The file, as Aurora writes it: factions and reputations in id order,
    /// struct ids counting up.
    pub fn to_gff(&self) -> Gff {
        let mut g = Gff::new(*b"FAC ");
        g.root = self.root.clone();
        let list: Vec<Struct> = self
            .factions
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let mut s = f.original.clone();
                s.id = i as u32;
                s.write(&fac::faction_list::FACTION_PARENT_ID, f.parent.unwrap_or(u32::MAX));
                s.write(&fac::faction_list::FACTION_NAME, ExoString(encode(&f.name)));
                s.write(&fac::faction_list::FACTION_GLOBAL, u16::from(f.global));
                s
            })
            .collect();
        let reps: Vec<Struct> = self
            .reputations
            .iter()
            .enumerate()
            .map(|(i, (&(target, perceiver), &rep))| {
                let mut s = Struct::new(i as u32);
                s.write(&fac::rep_list::FACTION_ID1, target);
                s.write(&fac::rep_list::FACTION_ID2, perceiver);
                s.write(&fac::rep_list::FACTION_REP, rep);
                s
            })
            .collect();
        g.root.set(fac::FACTION_LIST.label, Value::List(list));
        g.root.set(fac::REP_LIST.label, Value::List(reps));
        g
    }

    /// How much `perceiver` likes `target`, if the file says.
    pub fn reputation(&self, perceiver: u32, target: u32) -> Option<u32> {
        self.reputations.get(&(target, perceiver)).copied()
    }

    /// How much `perceiver` likes `target`; a missing entry is written with
    /// the default first, as Aurora does.
    fn lookup(&mut self, perceiver: u32, target: u32) -> u32 {
        *self.reputations.entry((target, perceiver)).or_insert(DEFAULT_REPUTATION)
    }

    pub fn set_reputation(&mut self, perceiver: u32, target: u32, rep: u32) {
        self.reputations.insert((target, perceiver), rep.min(100));
    }

    /// Adds a faction that starts with its parent's reputations both ways,
    /// and returns its id. Aurora offers only the standard factions after PC
    /// as parents.
    pub fn add(&mut self, name: &str, global: bool, parent: u32) -> u32 {
        let new = self.factions.len() as u32;
        self.factions.push(Faction {
            name: name.to_string(),
            parent: Some(parent),
            global,
            original: Struct::new(new),
        });
        for f in 0..=new {
            let rep = self.lookup(parent, f);
            self.reputations.insert((f, new), rep);
        }
        for f in 0..=new {
            let rep = self.lookup(f, parent);
            self.reputations.insert((new, f), rep);
        }
        new
    }

    /// Removes a faction (not a standard one): its reputations go, and the
    /// factions after it move down one id. Returns the old-to-new id map
    /// for objects that refer to factions ([`renumber_faction_users`]).
    pub fn remove(&mut self, id: u32) -> Option<impl Fn(u32) -> Option<u32> + use<>> {
        if (id as usize) < STANDARD.len() || id as usize >= self.factions.len() {
            return None;
        }
        self.factions.remove(id as usize);
        let map = move |old: u32| match old.cmp(&id) {
            std::cmp::Ordering::Less => Some(old),
            std::cmp::Ordering::Equal => None,
            std::cmp::Ordering::Greater => Some(old - 1),
        };
        for f in &mut self.factions {
            f.parent = f.parent.and_then(map);
        }
        self.reputations = std::mem::take(&mut self.reputations)
            .into_iter()
            .filter_map(|((t, p), r)| Some(((map(t)?, map(p)?), r)))
            .collect();
        Some(map)
    }
}

/// Labels of the faction fields of objects and their instances.
const FACTION_FIELDS: [&str; 2] = ["FactionID", "Faction"];
const FACTION_USERS: [ResType; 6] =
    [ResType::UTC, ResType::UTD, ResType::UTP, ResType::UTT, ResType::UTE, ResType::GIT];

fn walk(s: &mut Struct, f: &mut dyn FnMut(&mut Value)) {
    for field in &mut s.fields {
        match &mut field.value {
            Value::Struct(c) => walk(c, f),
            Value::List(items) => items.iter_mut().for_each(|i| walk(i, f)),
            v if FACTION_FIELDS.contains(&field.label.to_string_lossy().as_str()) => f(v),
            _ => {}
        }
    }
}

fn faction_of(v: &Value) -> Option<u32> {
    match v {
        Value::Word(x) => Some(u32::from(*x)),
        Value::Dword(x) => Some(*x),
        Value::Int(x) => u32::try_from(*x).ok(),
        _ => None,
    }
}

/// The module's objects (blueprints and placed instances) in a faction.
pub fn faction_users(m: &Module, id: u32) -> Vec<ResKey> {
    let keys: Vec<ResKey> =
        m.keys().filter(|k| FACTION_USERS.contains(&k.restype)).copied().collect();
    keys.into_iter()
        .filter(|k| {
            let Some(Ok(mut g)) = m.gff(k) else { return false };
            let mut used = false;
            walk(&mut g.root, &mut |v| used |= faction_of(v) == Some(id));
            used
        })
        .collect()
}

/// Rewrites the faction ids of the module's objects after a removal;
/// returns the resources changed.
pub fn renumber_faction_users(m: &mut Module, map: impl Fn(u32) -> Option<u32>) -> Vec<ResKey> {
    let keys: Vec<ResKey> =
        m.keys().filter(|k| FACTION_USERS.contains(&k.restype)).copied().collect();
    let mut changed = Vec::new();
    for k in keys {
        let Some(Ok(mut g)) = m.gff(&k) else { continue };
        let mut any = false;
        walk(&mut g.root, &mut |v| {
            let Some(old) = faction_of(v) else { return };
            let new = map(old).unwrap_or(old);
            if new != old {
                any = true;
                *v = match v {
                    Value::Word(_) => Value::Word(new as u16),
                    Value::Int(_) => Value::Int(new as i32),
                    _ => Value::Dword(new),
                };
            }
        });
        if any && m.set_gff(k, &g).is_ok() {
            changed.push(k);
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_factions_round_trip() {
        let g = crate::new::default_factions();
        let f = Factions::read(&g);
        assert_eq!(f.factions.len(), 5);
        assert_eq!(f.reputation(1, 0), Some(0), "Hostile hates PCs");
        assert_eq!(f.reputation(4, 2), Some(100), "Defenders like Commoners");
        assert_eq!(f.to_gff(), g);
    }

    #[test]
    fn standard_factions_cannot_be_removed() {
        let standard = Factions::read(&crate::new::default_factions());
        let mut f = standard.clone();
        assert!(f.remove(4).is_none());
        let id = f.add("Guards", true, 4);
        assert_eq!(id, 5);
        assert!(f.remove(id).is_some());
        assert_eq!(f.factions, standard.factions);
    }
}
