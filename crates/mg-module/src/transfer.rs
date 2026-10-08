//! Exporting module resources to an ERF with their dependencies, and
//! importing an ERF into a module (Aurora's File › Export / Import).

use std::collections::{BTreeSet, HashSet};

use mg_core::{ResRef, ResType, StrRef};
use mg_erf::{Description, Erf, ErfWriter};
use mg_gff::{Gff, Struct};
use mg_resman::{ResKey, ResMan};
use mg_schema::{StructExt, fac, git, ifo, utc};

use crate::refs::{Reference, references};
use crate::{Module, ModuleError};

/// What an export will contain.
#[derive(Debug, Clone, Default)]
pub struct ExportPlan {
    /// The chosen resources and every module resource they depend on.
    pub resources: Vec<ResKey>,
    /// Dependencies found neither in the module nor in the game (or haks).
    pub missing: Vec<Reference>,
}

fn area_companions(k: &ResKey) -> Vec<ResKey> {
    match k.restype {
        ResType::ARE | ResType::GIT | ResType::GIC => [ResType::ARE, ResType::GIT, ResType::GIC]
            .into_iter()
            .map(|t| ResKey::new(k.resref, t))
            .collect(),
        _ => vec![*k],
    }
}

/// What goes with a resource as loose files (those the module has): an
/// area's `.are`, `.git` and `.gic`; a script's source, its compiled
/// script (`.ncs`, what the game runs) and its debug file (`.ndb`).
pub fn file_set(module: &Module, key: &ResKey) -> Vec<ResKey> {
    let types: &[ResType] = match key.restype {
        ResType::ARE | ResType::GIT | ResType::GIC => &[ResType::ARE, ResType::GIT, ResType::GIC],
        ResType::NSS | ResType::NCS | ResType::NDB => &[ResType::NSS, ResType::NCS, ResType::NDB],
        _ => return vec![*key],
    };
    types.iter().map(|t| ResKey::new(key.resref, *t)).filter(|k| module.contains(k)).collect()
}

/// Writes `resources` (those the module has, each once) as files named
/// `name.ext` in `dir` (made if missing), replacing files of those names:
/// as they are in the module, for a `development` or `override` folder or
/// to hand on. Returns the files written.
pub fn export_files(
    module: &Module,
    resources: &[ResKey],
    dir: &std::path::Path,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let mut seen = HashSet::new();
    let mut written = Vec::new();
    for k in resources {
        let Some(bytes) = module.get(k).filter(|_| seen.insert(*k)) else { continue };
        let path = dir.join(k.to_string());
        std::fs::write(&path, bytes)?;
        written.push(path);
    }
    Ok(written)
}

/// The resources to export for `roots`: the roots (an area brings its
/// `.git` and `.gic`), and everything in the module they refer to,
/// transitively. Dependencies that the game or `resman` provides are not
/// exported; ones found nowhere are listed as missing.
pub fn plan_export(module: &Module, roots: &[ResKey], resman: &ResMan) -> ExportPlan {
    let mut plan = ExportPlan::default();
    let mut included: BTreeSet<ResKey> = BTreeSet::new();
    let mut queue: Vec<ResKey> = roots.iter().flat_map(area_companions).collect();
    let mut missing_seen = HashSet::new();
    while let Some(k) = queue.pop() {
        if !module.contains(&k) || !included.insert(k) {
            continue;
        }
        for r in references(k, module.get(&k).unwrap_or_default()) {
            let candidates = r.candidates();
            let in_module: Vec<ResKey> =
                candidates.iter().copied().filter(|c| module.contains(c)).collect();
            if !in_module.is_empty() {
                queue.extend(in_module.iter().flat_map(area_companions));
            } else if !candidates.is_empty()
                && !candidates.iter().any(|c| resman.contains(c))
                && missing_seen.insert((r.kind, r.target))
            {
                plan.missing.push(r);
            }
        }
    }
    // Keep the module's order.
    plan.resources = module.keys().filter(|k| included.contains(k)).copied().collect();
    plan
}

/// Resolves a custom faction to the standard faction (0-4) it descends
/// from, through the module's `repute.fac` parents.
fn standard_faction(factions: &[Struct], mut id: u32) -> Option<u32> {
    for _ in 0..factions.len() + 1 {
        if id <= 4 {
            return Some(id);
        }
        id = factions.get(id as usize)?.read(&fac::faction_list::FACTION_PARENT_ID);
    }
    None
}

fn reset_factions(key: &ResKey, bytes: &[u8], factions: &[Struct]) -> Option<Vec<u8>> {
    let mut g = Gff::read(bytes).ok()?;
    let mut changed = false;
    let mut fix = |s: &mut Struct| {
        let id = s.read(&utc::FACTION_ID) as u32;
        if id > 4
            && let Some(std) = standard_faction(factions, id)
        {
            s.write(&utc::FACTION_ID, std as u16);
            changed = true;
        }
    };
    match key.restype {
        ResType::UTC => fix(&mut g.root),
        ResType::GIT => g.root.items_mut(&git::CREATURE_LIST).iter_mut().for_each(&mut fix),
        _ => return None,
    }
    if changed { g.to_bytes().ok() } else { None }
}

/// Writes an export ERF. `comments` becomes the archive's description; with
/// `reset_factions`, creatures in custom factions are moved to the standard
/// faction their faction descends from (another module has other factions).
pub fn export_erf(
    module: &Module,
    resources: &[ResKey],
    comments: &str,
    reset: bool,
) -> Result<Vec<u8>, ModuleError> {
    let factions: Vec<Struct> = module
        .gff(&ResKey::new(ResRef::from_str("repute").expect("valid"), ResType::FAC))
        .and_then(Result::ok)
        .map(|f| f.root.items(&fac::FACTION_LIST).to_vec())
        .unwrap_or_default();
    let mut w = ErfWriter::new(*b"ERF ");
    if !comments.is_empty() {
        w.description = Description {
            strref: StrRef::NONE,
            strings: vec![(
                0,
                mg_core::Codepage::WINDOWS_1252
                    .encode(comments)
                    .map_or_else(|| comments.as_bytes().to_vec(), |b| b.into_owned()),
            )],
        };
    }
    for k in resources {
        let Some(bytes) = module.get(k) else { continue };
        let bytes = if reset {
            reset_factions(k, bytes, &factions).unwrap_or_else(|| bytes.to_vec())
        } else {
            bytes.to_vec()
        };
        w.add(k.resref, k.restype, bytes).map_err(|e| ModuleError::Archive {
            path: Default::default(),
            message: e.to_string(),
        })?;
    }
    w.to_bytes()
        .map_err(|e| ModuleError::Archive { path: Default::default(), message: e.to_string() })
}

/// An archive with `resources` exported into it ([`export_erf`]) and what
/// `existing` (an ERF's bytes) already holds kept: a resource of the same
/// name is replaced where it stands, new ones follow. The archive keeps
/// its description unless `comments` gives another.
pub fn export_into_erf(
    existing: &[u8],
    module: &Module,
    resources: &[ResKey],
    comments: &str,
    reset: bool,
) -> Result<Vec<u8>, ModuleError> {
    let archive = |e: &dyn std::fmt::Display| ModuleError::Archive {
        path: Default::default(),
        message: e.to_string(),
    };
    let fresh = export_erf(module, resources, comments, reset)?;
    let fresh = Erf::read(&fresh).map_err(|e| archive(&e))?;
    let old = Erf::read(existing).map_err(|e| archive(&e))?;
    let mut w = ErfWriter::new(old.file_type);
    w.description =
        if comments.is_empty() { old.description.clone() } else { fresh.description.clone() };
    let mut seen = HashSet::new();
    for e in &old.entries {
        if !seen.insert(ResKey::new(e.resref, e.restype)) {
            continue;
        }
        let bytes = match fresh.find(&e.resref, e.restype) {
            Some(new) => fresh.data(new),
            None => old.data(e),
        };
        let bytes = bytes.map_err(|e| archive(&e))?.into_owned();
        w.add(e.resref, e.restype, bytes).map_err(|e| archive(&e))?;
    }
    for e in &fresh.entries {
        if seen.insert(ResKey::new(e.resref, e.restype)) {
            let bytes = fresh.data(e).map_err(|e| archive(&e))?.into_owned();
            w.add(e.resref, e.restype, bytes).map_err(|e| archive(&e))?;
        }
    }
    w.to_bytes().map_err(|e| archive(&e))
}

/// What an import will do.
#[derive(Debug, Clone, Default)]
pub struct ImportPlan {
    /// Everything in the archive.
    pub resources: Vec<ResKey>,
    /// Resources the module already has (the user picks which to overwrite).
    pub overwrites: Vec<ResKey>,
    /// References from the archive that neither it, the module nor the game
    /// can satisfy.
    pub missing: Vec<Reference>,
}

fn read_archive(data: &[u8]) -> Result<Vec<(ResKey, Vec<u8>)>, ModuleError> {
    let erf = Erf::read(data)
        .map_err(|e| ModuleError::Archive { path: Default::default(), message: e.to_string() })?;
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for e in &erf.entries {
        let k = ResKey::new(e.resref, e.restype);
        if seen.insert(k) {
            let bytes = erf.data(e).map_err(|err| ModuleError::Archive {
                path: Default::default(),
                message: err.to_string(),
            })?;
            out.push((k, bytes.into_owned()));
        }
    }
    Ok(out)
}

pub fn plan_import(
    module: &Module,
    archive: &[u8],
    resman: &ResMan,
) -> Result<ImportPlan, ModuleError> {
    let entries = read_archive(archive)?;
    let in_archive: HashSet<ResKey> = entries.iter().map(|(k, _)| *k).collect();
    let mut plan = ImportPlan {
        resources: entries.iter().map(|(k, _)| *k).collect(),
        overwrites: entries.iter().map(|(k, _)| *k).filter(|k| module.contains(k)).collect(),
        missing: Vec::new(),
    };
    let mut seen = HashSet::new();
    for (k, bytes) in &entries {
        for r in references(*k, bytes) {
            let c = r.candidates();
            let found = c
                .iter()
                .any(|x| in_archive.contains(x) || module.contains(x) || resman.contains(x));
            if !c.is_empty() && !found && seen.insert((r.kind, r.target)) {
                plan.missing.push(r);
            }
        }
    }
    Ok(plan)
}

/// What an import changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportSummary {
    pub added: Vec<ResKey>,
    pub replaced: Vec<ResKey>,
    pub skipped: Vec<ResKey>,
    /// Imported areas added to the module's area list.
    pub new_areas: Vec<ResRef>,
}

/// Imports an archive; `overwrite` decides for each resource the module
/// already has. New areas are added to `Mod_Area_list`.
pub fn import_erf(
    module: &mut Module,
    archive: &[u8],
    overwrite: impl Fn(&ResKey) -> bool,
) -> Result<ImportSummary, ModuleError> {
    let mut summary = ImportSummary::default();
    for (k, bytes) in read_archive(archive)? {
        if module.contains(&k) {
            if !overwrite(&k) {
                summary.skipped.push(k);
                continue;
            }
            summary.replaced.push(k);
        } else {
            summary.added.push(k);
        }
        module.set(k, bytes);
    }
    let mut info = module.info()?;
    let listed: HashSet<ResRef> = module.areas()?.into_iter().collect();
    let areas = info.root.items_mut(&ifo::MOD_AREA_LIST);
    for k in summary.added.iter().filter(|k| k.restype == ResType::ARE) {
        if !listed.contains(&k.resref) {
            let mut a = ifo::MOD_AREA_LIST.new_item();
            a.write(&ifo::mod_area_list::AREA_NAME, k.resref);
            areas.push(a);
            summary.new_areas.push(k.resref);
        }
    }
    if !summary.new_areas.is_empty() {
        module.set_info(&info)?;
    }
    Ok(summary)
}

/// A standard faction id's name, for reports.
pub fn standard_faction_name(id: u32) -> Option<&'static str> {
    ["PC", "Hostile", "Commoner", "Merchant", "Defender"].get(id as usize).copied()
}

#[cfg(test)]
mod tests {
    use mg_resman::{LayerClass, MemContainer, priority};
    use mg_schema::git;

    use super::*;

    fn rr(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    fn key(s: &str, t: ResType) -> ResKey {
        ResKey::new(rr(s), t)
    }

    /// A module with one area whose creature uses a module blueprint (with a
    /// module script and a base-game script) and a custom faction.
    fn sample() -> Module {
        let mut m = Module::new();
        let mut ifo = Gff::new(*b"IFO ");
        let mut a = ifo::MOD_AREA_LIST.new_item();
        a.write(&ifo::mod_area_list::AREA_NAME, rr("town"));
        ifo.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
        m.set_info(&ifo).unwrap();
        m.set(key("town", ResType::ARE), Gff::new(*b"ARE ").to_bytes().unwrap());
        let mut g = Gff::new(*b"GIT ");
        let mut c = git::CREATURE_LIST.new_item();
        c.write(&git::creature_list::TEMPLATE_RES_REF, rr("guard"));
        c.write(&git::creature_list::FACTION_ID, 5);
        g.root.items_mut(&git::CREATURE_LIST).push(c);
        m.set_gff(key("town", ResType::GIT), &g).unwrap();
        m.set(key("town", ResType::GIC), Gff::new(*b"GIC ").to_bytes().unwrap());
        let mut u = Gff::new(*b"UTC ");
        u.root.write(&utc::SCRIPT_SPAWN, rr("guard_spawn"));
        u.root.write(&utc::SCRIPT_HEARTBEAT, rr("nw_c2_default1"));
        u.root.write(&utc::CONVERSATION, rr("gone"));
        m.set_gff(key("guard", ResType::UTC), &u).unwrap();
        m.set(key("guard_spawn", ResType::NSS), b"void main() {}".to_vec());
        m.set(key("guard_spawn", ResType::NCS), b"NCS".to_vec());
        m.set(key("unrelated", ResType::UTI), b"x".to_vec());
        // Faction 5 ("Guards") descends from Defender (4).
        let mut f = Gff::new(*b"FAC ");
        let list = f.root.items_mut(&fac::FACTION_LIST);
        for (i, parent) in
            [u32::MAX, u32::MAX, u32::MAX, u32::MAX, u32::MAX, 4].into_iter().enumerate()
        {
            let mut s = fac::FACTION_LIST.new_item();
            s.id = i as u32;
            s.write(&fac::faction_list::FACTION_PARENT_ID, parent);
            list.push(s);
        }
        m.set_gff(key("repute", ResType::FAC), &f).unwrap();
        m
    }

    fn base() -> ResMan {
        let mut c = MemContainer::new();
        c.insert(key("nw_c2_default1", ResType::NCS), Vec::new());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "base", LayerClass::Key, c);
        rm
    }

    #[test]
    fn export_follows_dependencies_within_the_module() {
        let m = sample();
        let plan = plan_export(&m, &[key("town", ResType::ARE)], &base());
        let names: Vec<String> = plan.resources.iter().map(|k| k.to_string()).collect();
        assert_eq!(
            names,
            ["town.are", "town.git", "town.gic", "guard.utc", "guard_spawn.nss", "guard_spawn.ncs"]
        );
        assert_eq!(plan.missing.len(), 1);
        assert_eq!(plan.missing[0].target, rr("gone"));
    }

    #[test]
    fn export_import_round_trip_resets_factions_and_lists_areas() {
        let src = sample();
        let plan = plan_export(&src, &[key("town", ResType::ARE)], &base());
        let erf = export_erf(&src, &plan.resources, "Town with guards", true).unwrap();
        let archive = Erf::read(&erf).unwrap();
        assert_eq!(archive.description.strings[0].1, b"Town with guards");
        let git = Gff::read(&archive.get(&rr("town"), ResType::GIT).unwrap().unwrap()).unwrap();
        assert_eq!(git.root.items(&git::CREATURE_LIST)[0].read(&git::creature_list::FACTION_ID), 4);

        // Import into a module that already has a (different) guard.
        let mut dst = Module::new();
        dst.set_info(&Gff::new(*b"IFO ")).unwrap();
        dst.set(key("guard", ResType::UTC), b"theirs".to_vec());
        let iplan = plan_import(&dst, &erf, &base()).unwrap();
        assert_eq!(iplan.overwrites, [key("guard", ResType::UTC)]);
        assert_eq!(
            iplan.missing.iter().map(|r| r.target.to_string()).collect::<Vec<_>>(),
            ["gone"]
        );
        let summary = import_erf(&mut dst, &erf, |_| false).unwrap();
        assert_eq!(summary.skipped, [key("guard", ResType::UTC)]);
        assert_eq!(summary.new_areas, [rr("town")]);
        assert_eq!(dst.get(&key("guard", ResType::UTC)), Some(&b"theirs"[..]));
        assert_eq!(dst.areas().unwrap(), [rr("town")]);
    }

    #[test]
    fn exporting_into_an_archive_keeps_what_it_holds() {
        let src = sample();
        let town = export_erf(&src, &[key("town", ResType::ARE)], "The town", false).unwrap();
        let mut changed = sample();
        changed.set(key("town", ResType::ARE), b"another town".to_vec());
        changed.set(key("guard", ResType::UTC), b"a guard".to_vec());
        let keys = [key("guard", ResType::UTC), key("town", ResType::ARE)];
        let both = export_into_erf(&town, &changed, &keys, "", false).unwrap();
        let archive = Erf::read(&both).unwrap();
        let names: Vec<String> = archive.entries.iter().map(|e| e.filename()).collect();
        assert_eq!(names, ["town.are", "guard.utc"], "replaced where it stood, the new one after");
        assert_eq!(&*archive.get(&rr("town"), ResType::ARE).unwrap().unwrap(), b"another town");
        assert_eq!(archive.description.strings[0].1, b"The town", "its description is kept");
        assert!(export_into_erf(b"not an archive", &changed, &keys, "", false).is_err());
    }

    #[test]
    fn faction_chains() {
        let mut list = Vec::new();
        for parent in [u32::MAX, u32::MAX, u32::MAX, u32::MAX, u32::MAX, 6, 2, 7] {
            let mut s = Struct::new(0);
            s.write(&fac::faction_list::FACTION_PARENT_ID, parent);
            list.push(s);
        }
        assert_eq!(standard_faction(&list, 5), Some(2));
        assert_eq!(standard_faction(&list, 7), None, "a cycle resolves to nothing");
        assert_eq!(standard_faction(&list, 3), Some(3));
    }

    #[test]
    fn resources_are_written_as_files_with_what_goes_with_them() {
        let mut m = sample();
        m.set(key("guard_spawn", ResType::NSS), b"void main() {}".to_vec());
        // A script: its source and its compiled script; an area: its three
        // files; a blueprint: itself.
        let script = file_set(&m, &key("guard_spawn", ResType::NSS));
        assert_eq!(script, [key("guard_spawn", ResType::NSS), key("guard_spawn", ResType::NCS)]);
        assert_eq!(file_set(&m, &key("guard_spawn", ResType::NCS)), script);
        let area = file_set(&m, &key("town", ResType::GIT));
        assert_eq!(area, [ResType::ARE, ResType::GIT, ResType::GIC].map(|t| key("town", t)));
        assert_eq!(file_set(&m, &key("guard", ResType::UTC)), [key("guard", ResType::UTC)]);

        let dir = mg_testkit::scratch_dir("export-files").join("development");
        let mut all = script.clone();
        all.extend(area);
        // (One named twice, and one the module lacks, are written once and
        // not at all.)
        all.extend([key("guard_spawn", ResType::NCS), key("nowhere", ResType::UTC)]);
        let written = export_files(&m, &all, &dir).unwrap();
        let names: Vec<String> =
            written.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(
            names,
            ["guard_spawn.nss", "guard_spawn.ncs", "town.are", "town.git", "town.gic"]
        );
        assert_eq!(std::fs::read(dir.join("guard_spawn.ncs")).unwrap(), b"NCS");
        assert_eq!(
            std::fs::read(dir.join("town.git")).unwrap(),
            m.get(&key("town", ResType::GIT)).unwrap()
        );
        // Again: the files are replaced.
        m.set(key("guard_spawn", ResType::NCS), b"NCS2".to_vec());
        export_files(&m, &script, &dir).unwrap();
        assert_eq!(std::fs::read(dir.join("guard_spawn.ncs")).unwrap(), b"NCS2");
    }
}
