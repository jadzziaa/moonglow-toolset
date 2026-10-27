//! Locked objects (the area editor's Lock): a byte of Moonglow's,
//! [`LOCKED`], on a placed object, which the game ignores and Aurora drops.
//!
//! In a nasher project the byte is kept out of the area's JSON, so that a
//! lock never shows in a builder's diff: Moonglow keeps the project's locks
//! in its own data folder instead ([`file`]), each found again by where
//! the object is ([`Lock`]), and puts them back on the objects when the
//! project is opened ([`crate::Module::restore_locks`]). An area file
//! that carries the byte already (written by an earlier Moonglow) keeps it
//! on each object that is still locked, until that object's lock changes:
//! only a change of the lock changes the file. Module archives and module
//! folders keep the byte, as before.

use std::path::{Path, PathBuf};

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;
use mg_rules::ObjectKind;

/// The field Moonglow marks a locked object with.
pub const LOCKED: &str = "MG_Locked";

/// How near a position must be to be the same place: a project's JSON may
/// keep fewer decimals than the area's floats (`truncateFloats`).
const NEAR: f32 = 0.01;

/// A locked object, as it is found again in its area: its list and place
/// in it, its blueprint, tag and position.
#[derive(Debug, Clone, PartialEq)]
pub struct Lock {
    pub area: ResRef,
    /// The GIT list (`Placeable List`, `Creature List`…).
    pub list: String,
    pub index: usize,
    pub template: String,
    pub tag: String,
    pub at: [f32; 3],
}

fn float(s: &Struct, label: &str) -> f32 {
    match s.get(label) {
        Some(Value::Float(f)) => *f,
        Some(Value::Double(d)) => *d as f32,
        _ => 0.0,
    }
}

fn text(s: &Struct, label: &str) -> String {
    match s.get(label) {
        Some(Value::String(b) | Value::ResRef(b)) => String::from_utf8_lossy(b).to_lowercase(),
        _ => String::new(),
    }
}

/// Whether an object is locked.
pub fn is_locked(s: &Struct) -> bool {
    s.integer(LOCKED).is_some_and(|v| v != 0)
}

fn position(kind: ObjectKind, s: &Struct) -> [f32; 3] {
    let labels =
        if kind.has_bearing() { ["X", "Y", "Z"] } else { ["XPosition", "YPosition", "ZPosition"] };
    labels.map(|l| float(s, l))
}

impl Lock {
    fn of(area: ResRef, kind: ObjectKind, index: usize, s: &Struct) -> Lock {
        Lock {
            area,
            list: kind.list().to_string(),
            index,
            template: text(s, "TemplateResRef"),
            tag: text(s, "Tag"),
            at: position(kind, s),
        }
    }

    /// The same blueprint and tag.
    fn alike(&self, s: &Struct) -> bool {
        text(s, "TemplateResRef") == self.template && text(s, "Tag") == self.tag
    }

    /// Alike, and in the same place.
    fn same(&self, kind: ObjectKind, s: &Struct) -> bool {
        let at = position(kind, s);
        self.alike(s) && at.iter().zip(self.at).all(|(a, b)| (a - b).abs() <= NEAR)
    }
}

/// The locked objects of an area's GIT.
pub fn locks_in(area: ResRef, git: &Gff) -> Vec<Lock> {
    let mut out = Vec::new();
    for kind in ObjectKind::ALL {
        for (i, s) in git.root.list(kind.list()).unwrap_or_default().iter().enumerate() {
            if is_locked(s) {
                out.push(Lock::of(area, kind, i, s));
            }
        }
    }
    out
}

/// The locked objects of every area of a module.
pub fn module_locks(module: &crate::Module) -> Vec<Lock> {
    let mut keys: Vec<ResKey> = module.keys_of(ResType::GIT).copied().collect();
    keys.sort();
    let mut out = Vec::new();
    for k in keys {
        if let Some(Ok(git)) = module.gff(&k) {
            out.extend(locks_in(k.resref, &git));
        }
    }
    out
}

/// Locks the objects of `git` that `locks` find (the area's); returns how
/// many it locked that weren't. Each lock finds the object at its place
/// in its list if that is alike and where it was, else one alike where it
/// was, else the one at its place if alike; else none (the lock is lost).
pub fn apply(git: &mut Gff, locks: &[&Lock]) -> usize {
    let mut n = 0;
    for kind in ObjectKind::ALL {
        let Some(list) = git.root.list_mut(kind.list()) else { continue };
        let mut taken = vec![false; list.len()];
        for lock in locks.iter().filter(|l| l.list == kind.list()) {
            let at = |i: usize| list.get(i).filter(|_| !taken[i]);
            let found = at(lock.index)
                .filter(|s| lock.same(kind, s))
                .map(|_| lock.index)
                .or_else(|| (0..list.len()).find(|&i| !taken[i] && lock.same(kind, &list[i])))
                .or_else(|| at(lock.index).filter(|s| lock.alike(s)).map(|_| lock.index));
            if let Some(i) = found {
                taken[i] = true;
                if !is_locked(&list[i]) {
                    list[i].set(LOCKED, Value::Byte(1));
                    n += 1;
                }
            }
        }
    }
    n
}

/// Takes the lock byte off every object of `git` but those `file` (the
/// area's file as it stands, if it has one) carries it on already, at the
/// same place in the same list and alike: those keep it, so that only a
/// change of a lock changes the file. Returns how many it took off.
pub fn strip(git: &mut Gff, file: Option<&Gff>) -> usize {
    let mut n = 0;
    for kind in ObjectKind::ALL {
        let before = file.and_then(|f| f.root.list(kind.list())).unwrap_or_default();
        let Some(list) = git.root.list_mut(kind.list()) else { continue };
        for (i, s) in list.iter_mut().enumerate() {
            if s.get(LOCKED).is_none() {
                continue;
            }
            let kept = before.get(i).is_some_and(|b| {
                is_locked(b)
                    && is_locked(s)
                    && text(b, "TemplateResRef") == text(s, "TemplateResRef")
                    && text(b, "Tag") == text(s, "Tag")
            });
            if !kept {
                s.remove(LOCKED);
                n += 1;
            }
        }
    }
    n
}

/// Whether a GIT carries the lock byte anywhere.
pub fn has_locks(git: &Gff) -> bool {
    ObjectKind::ALL.iter().any(|k| {
        git.root.list(k.list()).unwrap_or_default().iter().any(|s| s.get(LOCKED).is_some())
    })
}

/// The file in `dir` (Moonglow's `locks` folder) that keeps the locks of
/// the project at `module`: named by the SHA-1 of the project's path,
/// which stays the same from one build of Moonglow to the next. A file
/// under the name earlier builds gave it (by Rust's own hasher, which a
/// new Rust may change) is renamed to it, so a project keeps its locks.
pub fn file(dir: &Path, module: &Path) -> PathBuf {
    let module = crate::build::settled(module);
    let name = module.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let digest = mg_core::sha1::sha1(module.as_os_str().as_encoded_bytes());
    let path = dir.join(format!("{name}-{}.json", mg_core::sha1::hex(&digest[..8])));
    let old = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        module.hash(&mut h);
        dir.join(format!("{name}-{:016x}.json", h.finish()))
    };
    if old != path && !path.exists() && old.exists() && std::fs::rename(&old, &path).is_err() {
        return old;
    }
    path
}

/// The locks kept in `path` (none if there is no such file); an error for
/// a file that can't be read or is not a list of locks (cut short, or
/// edited by hand), to be said.
pub fn try_read(path: &Path) -> Result<Vec<Lock>, String> {
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let v = serde_json::from_slice::<serde_json::Value>(&data)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(locks_of(&v))
}

/// The locks kept in `path`: none if there are none or it can't be read.
/// A file that can't be read as locks is kept beside, as
/// `<name>.unreadable`, before what is written next replaces it (see
/// [`try_read`] to say why).
pub fn read(path: &Path) -> Vec<Lock> {
    try_read(path).unwrap_or_else(|_| {
        let _ = std::fs::copy(path, crate::sibling(path, ".unreadable"));
        Vec::new()
    })
}

fn locks_of(v: &serde_json::Value) -> Vec<Lock> {
    let locks = v.get("locks").and_then(|l| l.as_array()).cloned().unwrap_or_default();
    locks
        .iter()
        .filter_map(|l| {
            let s = |k: &str| l.get(k).and_then(|v| v.as_str()).map(str::to_string);
            let at = l.get("at")?.as_array()?;
            let f = |i: usize| at.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            Some(Lock {
                area: ResRef::from_str(&s("area")?).ok()?,
                list: s("list")?,
                index: l.get("index")?.as_u64()? as usize,
                template: s("template").unwrap_or_default(),
                tag: s("tag").unwrap_or_default(),
                at: [f(0), f(1), f(2)],
            })
        })
        .collect()
}

/// Keeps `locks` in `path` (the file is removed when there are none).
pub fn write(path: &Path, module: &Path, locks: &[Lock]) -> std::io::Result<()> {
    if locks.is_empty() {
        return match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    let locks: Vec<serde_json::Value> = locks
        .iter()
        .map(|l| {
            serde_json::json!({
                "area": l.area.to_string(), "list": l.list, "index": l.index,
                "template": l.template, "tag": l.tag, "at": l.at,
            })
        })
        .collect();
    let v = serde_json::json!({ "module": module.display().to_string(), "locks": locks });
    crate::write_replacing(path, &serde_json::to_vec_pretty(&v)?)
}

#[cfg(test)]
mod tests {
    use mg_schema::{StructExt, git};

    use super::*;

    fn placeable(template: &str, x: f32) -> Struct {
        let mut s = git::PLACEABLE_LIST.new_item();
        s.set("TemplateResRef", Value::resref(ResRef::from_str(template).unwrap()));
        s.set("Tag", Value::String(template.to_uppercase().into_bytes()));
        s.set("X", Value::Float(x));
        s
    }

    fn area(objects: &[(&str, f32, bool)]) -> Gff {
        let mut g = Gff::new(*b"GIT ");
        for &(t, x, locked) in objects {
            let mut s = placeable(t, x);
            if locked {
                s.set(LOCKED, Value::Byte(1));
            }
            g.root.items_mut(&git::PLACEABLE_LIST).push(s);
        }
        g
    }

    fn locked(g: &Gff) -> Vec<bool> {
        g.root.items(&git::PLACEABLE_LIST).iter().map(is_locked).collect()
    }

    /// Locks found again: in their place, moved down the list, or (where
    /// the object moved) at their place; an object gone loses its lock.
    #[test]
    fn locks_are_found_again() {
        let a1 = ResRef::from_str("a1").unwrap();
        let locks =
            locks_in(a1, &area(&[("tree", 1.0, true), ("rock", 2.0, false), ("tree", 3.0, true)]));
        assert_eq!(locks.len(), 2);
        assert_eq!((locks[1].index, locks[1].at[0]), (2, 3.0));
        let all: Vec<&Lock> = locks.iter().collect();
        // As they were.
        let mut g = area(&[("tree", 1.0, false), ("rock", 2.0, false), ("tree", 3.0, false)]);
        assert_eq!(apply(&mut g, &all), 2);
        assert_eq!(locked(&g), [true, false, true]);
        // One added before them (a JSON with fewer decimals, too).
        let mut g = area(&[
            ("well", 0.0, false),
            ("tree", 1.004, false),
            ("rock", 2.0, false),
            ("tree", 3.0, false),
        ]);
        assert_eq!(apply(&mut g, &all), 2);
        assert_eq!(locked(&g), [false, true, false, true]);
        // The first moved (the same place in the list); the second gone.
        let mut g = area(&[("tree", 9.0, false), ("rock", 2.0, false)]);
        assert_eq!(apply(&mut g, &all), 1);
        assert_eq!(locked(&g), [true, false]);
    }

    /// The byte goes, but where the file had it on that object already.
    #[test]
    fn the_byte_is_kept_only_where_the_file_had_it() {
        let file = area(&[("tree", 1.0, true), ("rock", 2.0, false)]);
        let mut now = area(&[("tree", 1.0, true), ("rock", 2.0, true)]);
        assert_eq!(strip(&mut now, Some(&file)), 1);
        assert_eq!(locked(&now), [true, false]);
        assert!(has_locks(&now));
        let mut new = area(&[("tree", 1.0, true)]);
        assert_eq!(strip(&mut new, None), 1);
        assert!(!has_locks(&new));
        // Unlocked (the byte set to 0): taken off.
        let mut now = area(&[("tree", 1.0, true)]);
        now.root.items_mut(&git::PLACEABLE_LIST)[0].set(LOCKED, Value::Byte(0));
        assert_eq!(strip(&mut now, Some(&file)), 1);
        assert!(!has_locks(&now));
    }

    /// A project: the byte never reaches the area's JSON; the locks kept
    /// apart come back on opening, and the project is as it was read. A
    /// file that has the byte keeps it until that lock changes.
    #[test]
    fn a_project_keeps_its_locks_apart() {
        use crate::{Module, ModuleLocation};
        use mg_schema::ifo;
        let root = mg_testkit::scratch_dir("locks-project");
        let key = ResKey::new(ResRef::from_str("a1").unwrap(), ResType::GIT);
        let mut m = Module::new();
        let mut info = Gff::new(*b"IFO ");
        let mut item = ifo::MOD_AREA_LIST.new_item();
        item.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str("a1").unwrap());
        info.root.items_mut(&ifo::MOD_AREA_LIST).push(item);
        m.set_info(&info).unwrap();
        m.set_gff(ResKey::new(key.resref, ResType::ARE), &Gff::new(*b"ARE ")).unwrap();
        m.set_gff(key, &area(&[("tree", 1.0, true), ("rock", 2.0, false)])).unwrap();
        let location = ModuleLocation::Project { root: root.clone(), target: "default".into() };
        m.save_as(&location).unwrap();
        let json = root.join("src/a1.git.json");
        let read = || std::fs::read_to_string(&json).unwrap();
        assert!(!read().contains(LOCKED), "{}", read());
        let locks = module_locks(&m);
        assert_eq!(locks.len(), 1);
        // Saved again, locked: nothing changes on disk.
        let written = read();
        m.save().unwrap();
        assert_eq!(read(), written);

        let mut back = Module::open(&root).unwrap();
        let git = |m: &Module| m.gff(&key).unwrap().unwrap();
        assert_eq!(locked(&git(&back)), [false, false]);
        assert_eq!(back.restore_locks(&locks), 1);
        assert_eq!(locked(&git(&back)), [true, false]);
        assert!(!back.is_dirty());
        assert_eq!(back.as_read(&key), back.get(&key), "the file stands for the locked area");
        back.save().unwrap();
        assert_eq!(read(), written);
        // An archive or folder module has no locks apart.
        assert_eq!(Module::new().restore_locks(&locks), 0);

        // A file with the byte (an earlier Moonglow's): kept while the
        // object stays locked, through other changes; gone with the lock.
        let conv = back.project.as_ref().unwrap().conversion();
        let old = conv.to_source(ResType::GIT, &area(&[("tree", 1.0, true), ("rock", 2.0, false)]));
        std::fs::write(&json, old.unwrap()).unwrap();
        let mut back = Module::open(&root).unwrap();
        let mut g = git(&back);
        g.root.items_mut(&git::PLACEABLE_LIST)[1].set("X", Value::Float(5.0));
        back.set_gff(key, &g).unwrap();
        back.save().unwrap();
        assert_eq!(read().matches(LOCKED).count(), 1, "{}", read());
        g.root.items_mut(&git::PLACEABLE_LIST)[0].remove(LOCKED);
        back.set_gff(key, &g).unwrap();
        back.save().unwrap();
        assert!(!read().contains(LOCKED));
    }

    #[test]
    fn locks_are_kept_in_a_file() {
        let dir = mg_testkit::scratch_dir("locks-file");
        let path = file(&dir, &dir.join("project"));
        assert!(path.file_name().unwrap().to_string_lossy().starts_with("project-"));
        let locks = locks_in(ResRef::from_str("a1").unwrap(), &area(&[("tree", 1.5, true)]));
        write(&path, &dir, &locks).unwrap();
        assert_eq!(read(&path), locks);
        write(&path, &dir, &[]).unwrap();
        assert!(!path.exists());
        write(&path, &dir, &[]).unwrap();
        assert!(read(&path).is_empty());
    }

    /// The file's name is the same from one build to the next (SHA-1, not
    /// Rust's hasher); a file under the name earlier builds gave it is
    /// taken over, so a project keeps its locks.
    #[test]
    fn the_locks_file_keeps_its_name_and_an_old_one_is_taken_over() {
        let dir = mg_testkit::scratch_dir("locks-file-name");
        let project = dir.join("project");
        let path = file(&dir, &project);
        let settled = crate::build::settled(&project);
        let digest = mg_core::sha1::sha1(settled.as_os_str().as_encoded_bytes());
        let name = format!("project-{}.json", mg_core::sha1::hex(&digest[..8]));
        assert_eq!(path, dir.join(name));
        let old = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            settled.hash(&mut h);
            dir.join(format!("project-{:016x}.json", h.finish()))
        };
        let locks = locks_in(ResRef::from_str("a1").unwrap(), &area(&[("tree", 1.5, true)]));
        write(&old, &project, &locks).unwrap();
        assert_eq!(file(&dir, &project), path);
        assert!(!old.exists());
        assert_eq!(read(&path), locks);
    }

    /// A file cut short (or broken by hand) is an error to say, and is
    /// kept beside before the next write replaces it; writing leaves no
    /// file of its own behind.
    #[test]
    fn a_cut_locks_file_is_said_and_kept() {
        let dir = mg_testkit::scratch_dir("locks-file-cut");
        let path = file(&dir, &dir.join("project"));
        let locks = locks_in(ResRef::from_str("a1").unwrap(), &area(&[("tree", 1.5, true)]));
        write(&path, &dir, &locks).unwrap();
        assert!(!crate::sibling(&path, ".moonglow-tmp").exists());
        let whole = std::fs::read(&path).unwrap();
        std::fs::write(&path, &whole[..whole.len() / 2]).unwrap();
        assert!(try_read(&path).is_err());
        assert!(read(&path).is_empty());
        let kept = crate::sibling(&path, ".unreadable");
        assert_eq!(std::fs::read(kept).unwrap(), whole[..whole.len() / 2]);
        assert_eq!(try_read(&dir.join("none.json")), Ok(Vec::new()));
    }
}
