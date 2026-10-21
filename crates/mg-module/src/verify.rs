//! Module verification (Aurora's Build › Verify passes that need no
//! compiler): references to resources that exist nowhere, and module
//! resources nothing references.

use std::collections::{BTreeSet, HashSet};

use mg_core::ResType;
use mg_resman::{ResKey, ResMan, priority};
use mg_rules::ObjectKind;

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
        let list = list.split('[').next().unwrap_or_default();
        match r.from.restype {
            ResType::GIT => ObjectKind::from_list(list).map_or(Category::Areas, Category::from),
            t => Category::of_type(t),
        }
    }

    pub fn of_type(t: ResType) -> Category {
        match t {
            ResType::IFO => Category::Module,
            ResType::ARE | ResType::GIT | ResType::GIC => Category::Areas,
            ResType::DLG => Category::Conversations,
            ResType::NSS | ResType::NCS => Category::Scripts,
            t => ObjectKind::from_restype(t).map_or(Category::Other, Category::from),
        }
    }
}

impl From<ObjectKind> for Category {
    /// A type of placed object's or blueprint's category.
    fn from(kind: ObjectKind) -> Category {
        match kind {
            ObjectKind::Creature => Category::Creatures,
            ObjectKind::Door => Category::Doors,
            ObjectKind::Encounter => Category::Encounters,
            ObjectKind::Item => Category::Items,
            ObjectKind::Placeable => Category::Placeables,
            ObjectKind::Sound => Category::Sounds,
            ObjectKind::Store => Category::Stores,
            ObjectKind::Trigger => Category::Triggers,
            ObjectKind::Waypoint => Category::Waypoints,
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
    /// It is there, but only in the haks the user's `userpatch.ini` names:
    /// a player without them has nothing in its place.
    pub only_userpatch: bool,
}

impl Missing {
    /// Whether it breaks the module as it plays: references to scripts,
    /// conversations, areas and the like from areas, placed objects, the
    /// module, conversations and scripts. A blueprint's
    /// matter only once something places it, and an object placed from a
    /// blueprint that is gone keeps working (it holds all it needs), so those
    /// are warnings.
    /// Whether it is an object placed in an area from a blueprint the
    /// module no longer has (nor its haks or the game): common, even in
    /// BioWare's modules, and harmless as it plays; only placing more and
    /// Update Instances need the blueprint.
    pub fn placed_from_gone_blueprint(&self) -> bool {
        let r = &self.reference;
        r.from.restype == ResType::GIT
            && r.path.ends_with("/TemplateResRef")
            && matches!(r.kind, RefKind::Blueprint(_))
    }

    /// What is wrong with it, after its name: "missing", "not compiled",
    /// or found only in userpatch.ini's haks.
    pub fn what(&self) -> &'static str {
        if self.only_userpatch {
            "only in userpatch.ini's haks (players without them won't have it)"
        } else if self.uncompiled {
            "not compiled"
        } else {
            "missing"
        }
    }

    pub fn is_error(&self) -> bool {
        if self.only_userpatch {
            return false;
        }
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
/// form; one with only a source is reported as uncompiled. A reference
/// only userpatch.ini's haks satisfy is reported too, as such (a warning).
pub fn missing(module: &Module, resman: &ResMan) -> Vec<Missing> {
    missing_of(module, resman, module_references(module))
}

/// [`missing`] among the module's references already gathered
/// ([`module_references`]: every GFF read; [`unused_of`] takes them too).
pub fn missing_of(module: &Module, resman: &ResMan, references: Vec<Reference>) -> Vec<Missing> {
    let has = |k: &ResKey| module.contains(k) || resman.contains(k);
    // Found, but only in userpatch.ini's haks (the player's own, not the
    // module's): nothing else in the stack has it.
    let patched = resman.layers().iter().any(|l| l.priority == priority::USERPATCH);
    let only_userpatch = |k: &ResKey| {
        patched
            && !module.contains(k)
            && resman.find(k).is_some_and(|i| resman.layers()[i].priority == priority::USERPATCH)
            && !resman
                .layers()
                .iter()
                .any(|l| l.priority != priority::USERPATCH && l.container.contains(k))
    };
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for r in references {
        let candidates = r.candidates();
        // The game runs compiled scripts; a source alone does not satisfy a
        // script reference (but is reported as uncompiled, not missing).
        let satisfying: Vec<&ResKey> = match r.kind {
            RefKind::Script => {
                candidates.iter().filter(|k| k.restype == ResType::NCS && has(k)).collect()
            }
            _ => candidates.iter().filter(|k| has(k)).collect(),
        };
        let patch_only = !satisfying.is_empty() && satisfying.iter().all(|k| only_userpatch(k));
        if candidates.is_empty() || (!satisfying.is_empty() && !patch_only) {
            continue;
        }
        // Each missing target once per referencing resource (a dialog may
        // use the same script on hundreds of nodes).
        if !seen.insert((r.from, r.kind, r.target)) {
            continue;
        }
        let uncompiled = !patch_only
            && r.kind == RefKind::Script
            && candidates.iter().any(|k| k.restype == ResType::NSS && has(k));
        out.push(Missing {
            category: Category::of(&r),
            reference: r,
            uncompiled,
            only_userpatch: patch_only,
        });
    }
    out
}

/// Module resources of the given kinds that nothing in the module references:
/// blueprints, conversations and scripts (script sources count as used when
/// `#include`d; a compiled script and its source are one script).
pub fn unused(module: &Module) -> Vec<ResKey> {
    unused_of(module, &module_references(module))
}

/// [`unused`] by the module's references already gathered.
pub fn unused_of(module: &Module, references: &[Reference]) -> Vec<ResKey> {
    let mut used: HashSet<(mg_core::ResRef, ResType)> = HashSet::new();
    for r in references {
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

    #[test]
    fn a_placed_object_is_in_its_blueprints_category() {
        for k in ObjectKind::ALL {
            let r = Reference {
                from: ResKey::parse("area1", ResType::GIT).unwrap(),
                path: format!("/{}[2]/OnClick", k.list()),
                kind: RefKind::Script,
                target: rr("x"),
            };
            assert_eq!(Category::of(&r), Category::of_type(k.restype()), "{k:?}");
            assert_eq!(format!("{:?}", Category::from(k)), k.label(), "as Aurora's filters");
        }
        let area = Reference {
            from: ResKey::parse("area1", ResType::GIT).unwrap(),
            path: "/AreaProperties/MusicDay".into(),
            kind: RefKind::Script,
            target: rr("x"),
        };
        assert_eq!(Category::of(&area), Category::Areas);
        assert_eq!(Category::of_type(ResType::NCS), Category::Scripts);
        assert_eq!(Category::of_type(ResType::TWODA), Category::Other);
    }

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

    /// What only userpatch.ini's haks have is said, as a warning: a player
    /// without them has nothing there. What the game has too is not.
    #[test]
    fn a_reference_only_a_userpatch_hak_has_is_a_warning() {
        let mut m = Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        ifo.root.write(&ifo::MOD_ON_MOD_LOAD, rr("patch_only"));
        ifo.root.write(&ifo::MOD_ON_HEARTBEAT, rr("both"));
        m.set_info(&ifo).unwrap();
        let mut patch = MemContainer::new();
        let mut base = MemContainer::new();
        for n in ["patch_only", "both"] {
            patch.insert(key(n, ResType::NCS), Vec::new());
        }
        base.insert(key("both", ResType::NCS), Vec::new());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "base", LayerClass::Key, base);
        rm.add(priority::USERPATCH, "userpatch:hd", LayerClass::Erf, patch);
        let miss = missing(&m, &rm);
        assert_eq!(miss.len(), 1, "{miss:?}");
        assert_eq!(miss[0].reference.target.to_string(), "patch_only");
        assert!(miss[0].only_userpatch && !miss[0].is_error());
        assert!(miss[0].what().starts_with("only in userpatch.ini's haks"));
    }

    #[test]
    fn an_item_blueprint_only_lying_in_an_area_is_used() {
        let mut m = Module::new();
        m.set_info(&Gff::new(*b"IFO ")).unwrap();
        let mut g = Gff::new(*b"GIT ");
        let on_the_ground = |name: &str| {
            let mut item = mg_gff::Struct::new(0);
            item.set("TemplateResRef", mg_gff::Value::resref(rr(name)));
            item
        };
        g.root.set(
            "List",
            mg_gff::Value::List(vec![on_the_ground("floor_sword"), on_the_ground("gone_sword")]),
        );
        m.set_gff(key("area1", ResType::GIT), &g).unwrap();
        m.set(key("floor_sword", ResType::UTI), Vec::new());
        m.set(key("spare_sword", ResType::UTI), Vec::new());

        let unused: Vec<String> = unused(&m).iter().map(|k| k.to_string()).collect();
        assert_eq!(unused, ["spare_sword.uti"]);
        let miss = missing(&m, &ResMan::new());
        let got: Vec<(String, Category)> =
            miss.iter().map(|m| (m.reference.target.to_string(), m.category)).collect();
        assert_eq!(got, [("gone_sword".into(), Category::Items)]);
    }
}
