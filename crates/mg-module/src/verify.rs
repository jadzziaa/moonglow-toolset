//! Module verification (Aurora's Build › Verify passes that need no
//! compiler): references to resources that exist nowhere, and module
//! resources nothing references.

use std::collections::{BTreeSet, HashSet};

use mg_core::ResType;
use mg_resman::{ResKey, ResMan};

use crate::Module;
use crate::refs::{RefKind, Reference, references};

/// Which object type a finding belongs to, following Aurora's verify
/// filters (Creatures, Doors, Placeables, Items, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    Module,
    Areas,
    Creatures,
    Doors,
    Encounters,
    Items,
    Placeables,
    Sounds,
    Stores,
    Triggers,
    Waypoints,
    Conversations,
    Scripts,
    Other,
}

impl Category {
    /// The category of a reference, from its source and path (area
    /// instances belong to their object type).
    pub fn of(r: &Reference) -> Category {
        let list = r.path.split('/').nth(1).unwrap_or_default();
        let by_list = |l: &str| match l.split('[').next().unwrap_or_default() {
            "Creature List" => Some(Category::Creatures),
            "Door List" => Some(Category::Doors),
            "Encounter List" => Some(Category::Encounters),
            "Placeable List" => Some(Category::Placeables),
            "SoundList" => Some(Category::Sounds),
            "StoreList" => Some(Category::Stores),
            "TriggerList" => Some(Category::Triggers),
            "WaypointList" => Some(Category::Waypoints),
            _ => None,
        };
        match r.from.restype {
            ResType::GIT => by_list(list).unwrap_or(Category::Areas),
            t => Category::of_type(t),
        }
    }

    pub fn of_type(t: ResType) -> Category {
        match t {
            ResType::IFO => Category::Module,
            ResType::ARE | ResType::GIT | ResType::GIC => Category::Areas,
            ResType::UTC => Category::Creatures,
            ResType::UTD => Category::Doors,
            ResType::UTE => Category::Encounters,
            ResType::UTI => Category::Items,
            ResType::UTP => Category::Placeables,
            ResType::UTS => Category::Sounds,
            ResType::UTM => Category::Stores,
            ResType::UTT => Category::Triggers,
            ResType::UTW => Category::Waypoints,
            ResType::DLG => Category::Conversations,
            ResType::NSS | ResType::NCS => Category::Scripts,
            _ => Category::Other,
        }
    }
}

/// A reference nothing satisfies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missing {
    pub reference: Reference,
    pub category: Category,
    /// For scripts: the source exists but not the compiled script, which the
    /// game needs.
    pub uncompiled: bool,
}

impl Missing {
    /// Whether it breaks the module as it plays: references to scripts,
    /// conversations, areas and the like from areas, placed objects, the
    /// module, conversations and scripts. A blueprint's
    /// matter only once something places it, and an object placed from a
    /// blueprint that is gone keeps working (it holds all it needs), so those
    /// are warnings.
    pub fn is_error(&self) -> bool {
        let r = &self.reference;
        // A missing sound, portrait or movie leaves silence or a blank.
        if matches!(r.kind, RefKind::Sound | RefKind::Portrait | RefKind::Movie) {
            return false;
        }
        match r.from.restype {
            ResType::GIT => !r.path.ends_with("/TemplateResRef"),
            ResType::ARE | ResType::IFO | ResType::DLG | ResType::NSS => true,
            _ => false,
        }
    }
}

/// Every reference in the module.
pub fn module_references(module: &Module) -> Vec<Reference> {
    module.keys().flat_map(|k| references(*k, module.get(k).unwrap_or_default())).collect()
}

/// References that neither the module nor `resman` (the game, haks, and
/// usually the module layer too) can satisfy. Scripts need their compiled
/// form; one with only a source is reported as uncompiled.
pub fn missing(module: &Module, resman: &ResMan) -> Vec<Missing> {
    let has = |k: &ResKey| module.contains(k) || resman.contains(k);
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for r in module_references(module) {
        let candidates = r.candidates();
        // The game runs compiled scripts; a source alone does not satisfy a
        // script reference (but is reported as uncompiled, not missing).
        let satisfied = match r.kind {
            RefKind::Script => candidates.iter().any(|k| k.restype == ResType::NCS && has(k)),
            _ => candidates.iter().any(has),
        };
        if candidates.is_empty() || satisfied {
            continue;
        }
        // Each missing target once per referencing resource (a dialog may
        // use the same script on hundreds of nodes).
        if !seen.insert((r.from, r.kind, r.target)) {
            continue;
        }
        let uncompiled = r.kind == RefKind::Script
            && candidates.iter().any(|k| k.restype == ResType::NSS && has(k));
        out.push(Missing { category: Category::of(&r), reference: r, uncompiled });
    }
    out
}

/// Module resources of the given kinds that nothing in the module references:
/// blueprints, conversations and scripts (script sources count as used when
/// `#include`d; a compiled script and its source are one script).
pub fn unused(module: &Module) -> Vec<ResKey> {
    let mut used: HashSet<(mg_core::ResRef, ResType)> = HashSet::new();
    for r in module_references(module) {
        for k in r.candidates() {
            used.insert((k.resref, k.restype));
        }
    }
    let checked = [
        ResType::UTC,
        ResType::UTD,
        ResType::UTE,
        ResType::UTI,
        ResType::UTM,
        ResType::UTP,
        ResType::UTS,
        ResType::UTT,
        ResType::UTW,
        ResType::DLG,
        ResType::NSS,
        ResType::NCS,
    ];
    let mut out = BTreeSet::new();
    for k in module.keys().filter(|k| checked.contains(&k.restype)) {
        let script = matches!(k.restype, ResType::NSS | ResType::NCS);
        let is_used = if script {
            used.contains(&(k.resref, ResType::NCS)) || used.contains(&(k.resref, ResType::NSS))
        } else {
            used.contains(&(k.resref, k.restype))
        };
        if !is_used {
            out.insert(*k);
        }
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use mg_core::ResRef;
    use mg_gff::Gff;
    use mg_resman::{LayerClass, MemContainer, priority};
    use mg_schema::{StructExt, git, ifo};

    use super::*;

    fn rr(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    fn key(s: &str, t: ResType) -> ResKey {
        ResKey::new(rr(s), t)
    }

    #[test]
    fn missing_and_unused() {
        let mut m = Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        ifo.root.write(&ifo::MOD_ON_MOD_LOAD, rr("base_script"));
        ifo.root.write(&ifo::MOD_ON_HEARTBEAT, rr("nowhere"));
        ifo.root.write(&ifo::MOD_ON_CLIENT_ENTR, rr("src_only"));
        m.set_info(&ifo).unwrap();
        let mut g = Gff::new(*b"GIT ");
        let mut c = git::CREATURE_LIST.new_item();
        c.write(&git::creature_list::TEMPLATE_RES_REF, rr("used_bp"));
        c.write(&git::creature_list::CONVERSATION, rr("gone_dlg"));
        g.root.items_mut(&git::CREATURE_LIST).push(c);
        m.set_gff(key("area1", ResType::GIT), &g).unwrap();
        for (n, t) in [
            ("used_bp", ResType::UTC),
            ("spare_bp", ResType::UTC),
            ("src_only", ResType::NSS),
            ("lonely", ResType::NCS),
        ] {
            m.set(key(n, t), Vec::new());
        }
        let mut base = MemContainer::new();
        base.insert(key("base_script", ResType::NCS), Vec::new());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "base", LayerClass::Key, base);

        let miss = missing(&m, &rm);
        let got: Vec<(String, Category, bool)> = miss
            .iter()
            .map(|m| (m.reference.target.to_string(), m.category, m.uncompiled))
            .collect();
        assert_eq!(
            got,
            [
                ("nowhere".into(), Category::Module, false),
                ("src_only".into(), Category::Module, true),
                ("gone_dlg".into(), Category::Creatures, false),
            ]
        );
        let unused: Vec<String> = unused(&m).iter().map(|k| k.to_string()).collect();
        assert_eq!(unused, ["lonely.ncs", "spare_bp.utc"]);
    }
}
