//! The resource manager: resolves a resource name to bytes through a stack of
//! containers in the game's content load order.
//!
//! A [`ResMan`] is a list of [`Layer`]s, highest priority first. A lookup
//! returns the resource from the first layer that has it, except for textures
//! ([`ResMan::texture`]), where the engine prefers DDS over TGA within each
//! [`LayerClass`] before moving to a lower class.
//!
//! Layers carry the engine's numeric [`priority`] and stay sorted by it.
//! [`GameInstall`] finds the game and user directories; [`ResMan::for_game`]
//! builds the base stack the toolset sees with no module open, and
//! [`ResMan::add_haks`] adds a module's haks.

mod container;
mod install;

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use mg_core::{ResRef, ResType};
use thiserror::Error;

pub use container::{
    Container, DirContainer, ERF_READ_LIMIT, ErfContainer, KeyContainer, MemContainer,
};
pub use install::{GameInstall, default_user_dir};

/// A resource name and type.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResKey {
    pub resref: ResRef,
    pub restype: ResType,
}

impl ResKey {
    pub fn new(resref: ResRef, restype: ResType) -> ResKey {
        ResKey { resref, restype }
    }

    /// Parses `"name"` and a type; `None` if the name is not a valid resref.
    pub fn parse(name: &str, restype: ResType) -> Option<ResKey> {
        ResRef::from_str(name).ok().map(|r| ResKey::new(r, restype))
    }

    /// The resource's file in a folder (`name.ext`); an error for a name
    /// that is no file's there ([`ResRef::is_file_name`]: one read from a
    /// crafted archive may name a folder above it).
    pub fn path_in(&self, dir: &Path) -> std::io::Result<PathBuf> {
        if !self.resref.is_file_name() {
            let said = format!("{self:?}: the name is not a file's (it names a folder)");
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, said));
        }
        Ok(dir.join(self.to_string()))
    }
}

impl fmt::Debug for ResKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.resref, self.restype)
    }
}

impl fmt::Display for ResKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.resref.to_lowercase(), self.restype)
    }
}

#[derive(Debug, Error)]
pub enum ResError {
    #[error("{0} not found")]
    NotFound(ResKey),
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("{path}: {message}")]
    Container { path: PathBuf, message: String },
}

impl ResError {
    pub(crate) fn io(path: &Path, source: std::io::Error) -> ResError {
        ResError::Io { path: path.into(), source }
    }
}

/// Where a layer sits in the engine's texture preference (DDS over TGA is
/// decided within a class before falling to the next class).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayerClass {
    /// Loose files: override, development, portraits, module folders.
    Directory,
    /// NWSync content.
    NwSync,
    /// ERF archives: haks, the module, texture packs.
    Erf,
    /// The base game's KEY/BIF archives.
    Key,
}

/// The engine's resource priorities (higher wins), from its content load
/// order (nwn.wiki "Content Load Order"; the engine scales them by 10^6).
/// Layers with equal priority rank in the order they were added, first
/// highest, as a module's haks do in `Mod_HakList` order.
pub mod priority {
    pub const TEMP: u32 = 99;
    pub const PORTRAITS_USER: u32 = 91;
    pub const PORTRAITS: u32 = 90;
    pub const VAULT_USER: u32 = 81;
    pub const VAULT: u32 = 80;
    pub const DEVELOPMENT_USER: u32 = 71;
    pub const DEVELOPMENT: u32 = 70;
    pub const RIM: u32 = 60;
    pub const NWSYNC: u32 = 40;
    /// Haks found in the user's `hak/` directory.
    pub const HAK_USER: u32 = 31;
    /// Haks found in the install's `data/hk/`.
    pub const HAK: u32 = 30;
    pub const CURRENT_GAME: u32 = 23;
    pub const SAVE_GAME: u32 = 22;
    /// The module: below its haks, above `override/`.
    pub const MODULE: u32 = 20;
    pub const USERPATCH: u32 = 13;
    pub const OVERRIDE: u32 = 12;
    /// Steam Workshop items' `override/` folders, under the user's.
    pub const WORKSHOP: u32 = 11;
    pub const AMBIENT_USER: u32 = 9;
    pub const MUSIC_USER: u32 = 8;
    pub const AMBIENT: u32 = 7;
    pub const MUSIC: u32 = 6;
    pub const PATCH: u32 = 3;
    pub const KEY: u32 = 1;
}

/// One container in the stack, with a label for diagnostics ("where does
/// this resource come from?").
#[derive(Debug)]
pub struct Layer {
    pub label: String,
    pub priority: u32,
    pub class: LayerClass,
    pub container: Box<dyn Container>,
    /// The container's [`Container::fingerprint`] when it was last read.
    pub fingerprint: u64,
}

/// A stack of layers, highest priority first.
#[derive(Debug, Default)]
pub struct ResMan {
    layers: Vec<Layer>,
}

impl ResMan {
    pub fn new() -> ResMan {
        ResMan::default()
    }

    /// The layers, highest priority first.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Adds a layer at a [`priority`], below existing layers of the same
    /// priority.
    pub fn add(
        &mut self,
        priority: u32,
        label: impl Into<String>,
        class: LayerClass,
        c: impl Container + 'static,
    ) {
        let at =
            self.layers.iter().position(|l| l.priority < priority).unwrap_or(self.layers.len());
        let fingerprint = c.fingerprint();
        self.layers.insert(
            at,
            Layer { label: label.into(), priority, class, container: Box::new(c), fingerprint },
        );
    }

    /// Removes and returns the layer with this label.
    pub fn remove(&mut self, label: &str) -> Option<Layer> {
        let i = self.layers.iter().position(|l| l.label == label)?;
        Some(self.layers.remove(i))
    }

    /// The layer with this label, to change in place.
    pub fn layer_mut(&mut self, label: &str) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.label == label)
    }

    /// Puts another container in a layer, keeping the layer's place among
    /// those of its priority (removing and adding it again would put it
    /// last); returns the one it held.
    pub fn replace(
        &mut self,
        label: &str,
        c: impl Container + 'static,
    ) -> Option<Box<dyn Container>> {
        let layer = self.layer_mut(label)?;
        layer.fingerprint = c.fingerprint();
        Some(std::mem::replace(&mut layer.container, Box::new(c)))
    }

    /// Reads a layer's container again from disk ([`Container::rescan`]):
    /// a folder's new and removed files, an archive written again.
    /// `Ok(false)`: no layer has this label.
    pub fn rescan(&mut self, label: &str) -> Result<bool, ResError> {
        match self.layer_mut(label) {
            Some(l) => {
                l.container.rescan()?;
                l.fingerprint = l.container.fingerprint();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// The layers whose files changed on disk since they were read: haks
    /// written again, files added, removed or changed in a folder.
    pub fn changed_layers(&self, which: impl Fn(&Layer) -> bool) -> Vec<String> {
        self.layers
            .iter()
            .filter(|l| {
                which(l) && l.fingerprint != 0 && l.container.fingerprint() != l.fingerprint
            })
            .map(|l| l.label.clone())
            .collect()
    }

    /// Reads the [`changed_layers`](Self::changed_layers) among `which`
    /// again; returns their labels.
    pub fn reload_changed(
        &mut self,
        which: impl Fn(&Layer) -> bool,
    ) -> Result<Vec<String>, ResError> {
        let changed = self.changed_layers(which);
        for label in &changed {
            self.rescan(label)?;
        }
        Ok(changed)
    }

    /// The index of the layer that provides a resource.
    pub fn find(&self, key: &ResKey) -> Option<usize> {
        self.layers.iter().position(|l| l.container.contains(key))
    }

    pub fn contains(&self, key: &ResKey) -> bool {
        self.find(key).is_some()
    }

    /// The label of the layer that provides a resource.
    pub fn origin(&self, key: &ResKey) -> Option<&str> {
        self.find(key).map(|i| self.layers[i].label.as_str())
    }

    /// The bytes of a resource from the highest layer that has it.
    pub fn get(&self, key: &ResKey) -> Result<Cow<'_, [u8]>, ResError> {
        let i = self.find(key).ok_or(ResError::NotFound(*key))?;
        self.layers[i].container.read(key)
    }

    /// The first `n` bytes of a resource from the highest layer that has
    /// it ([`Container::read_start`]).
    pub fn get_start(&self, key: &ResKey, n: usize) -> Result<Cow<'_, [u8]>, ResError> {
        let i = self.find(key).ok_or(ResError::NotFound(*key))?;
        self.layers[i].container.read_start(key, n)
    }

    /// Convenience: [`ResMan::get`] by name and type.
    pub fn get_named(&self, name: &str, restype: ResType) -> Result<Cow<'_, [u8]>, ResError> {
        let key = ResKey::parse(name, restype).ok_or_else(|| ResError::Container {
            path: PathBuf::new(),
            message: format!("invalid resref {name:?}"),
        })?;
        self.get(&key)
    }

    /// Resolves a texture the way the engine does: within each layer class
    /// (directories, NWSync, ERFs, keys), a DDS anywhere in the class wins
    /// over a TGA; only then does a lower class count. Returns the type found.
    pub fn texture(&self, resref: ResRef) -> Option<(ResType, Cow<'_, [u8]>)> {
        let classes = [LayerClass::Directory, LayerClass::NwSync, LayerClass::Erf, LayerClass::Key];
        for class in classes {
            for restype in [ResType::DDS, ResType::TGA] {
                let key = ResKey::new(resref, restype);
                for l in self.layers.iter().filter(|l| l.class == class) {
                    if l.container.contains(&key) {
                        return l.container.read(&key).ok().map(|d| (restype, d));
                    }
                }
            }
        }
        None
    }

    /// The type [`ResMan::texture`] finds for a texture (DDS or TGA),
    /// without reading it.
    pub fn texture_type(&self, resref: ResRef) -> Option<ResType> {
        let classes = [LayerClass::Directory, LayerClass::NwSync, LayerClass::Erf, LayerClass::Key];
        for class in classes {
            for restype in [ResType::DDS, ResType::TGA] {
                let key = ResKey::new(resref, restype);
                if self.layers.iter().any(|l| l.class == class && l.container.contains(&key)) {
                    return Some(restype);
                }
            }
        }
        None
    }

    /// Every distinct resource with the index of the layer that provides it
    /// (the highest that has it), sorted by name and type.
    pub fn entries(&self) -> Vec<(ResKey, usize)> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for (i, l) in self.layers.iter().enumerate() {
            for k in l.container.keys() {
                if seen.insert(k) {
                    out.push((k, i));
                }
            }
        }
        out.sort_by_key(|(k, _)| (k.resref, k.restype.extension().unwrap_or_default()));
        out
    }

    /// The layers that have a resource, highest first: the first has the
    /// copy the game reads, the others the copies it overrides.
    pub fn copies(&self, key: &ResKey) -> Vec<usize> {
        let has = |(_, l): &(usize, &Layer)| l.container.contains(key);
        self.layers.iter().enumerate().filter(has).map(|(i, _)| i).collect()
    }

    /// The bytes of a resource as one layer has it (whether or not a
    /// higher one overrides it).
    pub fn get_in(&self, layer: usize, key: &ResKey) -> Result<Cow<'_, [u8]>, ResError> {
        match self.layers.get(layer) {
            Some(l) if l.container.contains(key) => l.container.read(key),
            _ => Err(ResError::NotFound(*key)),
        }
    }

    /// Every resource with every layer that has it, highest first (the
    /// first is the copy the game reads), sorted by name and type: what
    /// [`entries`](Self::entries) gives, and the copies it overrides.
    pub fn every_copy(&self) -> Vec<(ResKey, Vec<usize>)> {
        let mut at: HashMap<ResKey, Vec<usize>> = HashMap::new();
        for (i, l) in self.layers.iter().enumerate() {
            for k in l.container.keys() {
                let layers = at.entry(k).or_default();
                // (A layer lists a key once; a container that lists it
                // twice still has one copy of it.)
                if layers.last() != Some(&i) {
                    layers.push(i);
                }
            }
        }
        let mut out: Vec<(ResKey, Vec<usize>)> = at.into_iter().collect();
        out.sort_by_key(|(k, _)| (k.resref, k.restype.extension().unwrap_or_default()));
        out
    }

    /// Every distinct resource of a type across all layers, sorted by name.
    pub fn list(&self, restype: ResType) -> Vec<ResRef> {
        let mut seen = HashSet::new();
        for l in &self.layers {
            for k in l.container.keys() {
                if k.restype == restype {
                    seen.insert(k.resref);
                }
            }
        }
        let mut v: Vec<ResRef> = seen.into_iter().collect();
        v.sort();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(n: &str, t: ResType) -> ResKey {
        ResKey::parse(n, t).unwrap()
    }

    #[test]
    fn entries_name_the_providing_layer() {
        let (mut low, mut high) = (MemContainer::new(), MemContainer::new());
        low.insert(key("a", ResType::NSS), b"low".to_vec());
        low.insert(key("b", ResType::NSS), b"low".to_vec());
        high.insert(key("a", ResType::NSS), b"high".to_vec());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "low", LayerClass::Key, low);
        rm.add(priority::OVERRIDE, "high", LayerClass::Directory, high);
        let entries: Vec<(String, &str)> = rm
            .entries()
            .iter()
            .map(|(k, i)| (k.to_string(), rm.layers()[*i].label.as_str()))
            .collect();
        assert_eq!(entries, [("a.nss".to_string(), "high"), ("b.nss".to_string(), "low")]);
    }

    /// Two haks with the same table (a builder's report: the resource
    /// browser found only the one the game reads): every copy, highest
    /// first, each readable from its own layer.
    #[test]
    fn every_copy_of_a_resource_is_found() {
        let (mut game, mut lower, mut upper) =
            (MemContainer::new(), MemContainer::new(), MemContainer::new());
        let portraits = key("portraits", ResType::TWODA);
        game.insert(portraits, b"game".to_vec());
        game.insert(key("classes", ResType::TWODA), b"game".to_vec());
        lower.insert(portraits, b"lower".to_vec());
        upper.insert(portraits, b"upper".to_vec());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "key:nwn_base", LayerClass::Key, game);
        // (Haks of equal priority rank in the order added, first highest.)
        rm.add(priority::HAK, "hak:upper", LayerClass::Erf, upper);
        rm.add(priority::HAK, "hak:lower", LayerClass::Erf, lower);
        let labels = |layers: &[usize]| -> Vec<String> {
            layers.iter().map(|&i| rm.layers()[i].label.clone()).collect()
        };
        assert_eq!(labels(&rm.copies(&portraits)), ["hak:upper", "hak:lower", "key:nwn_base"]);
        let every = rm.every_copy();
        let found: Vec<(String, Vec<String>)> =
            every.iter().map(|(k, l)| (k.to_string(), labels(l))).collect();
        assert_eq!(
            found,
            [
                ("classes.2da".to_string(), vec!["key:nwn_base".to_string()]),
                (
                    "portraits.2da".to_string(),
                    vec!["hak:upper".into(), "hak:lower".into(), "key:nwn_base".into()]
                ),
            ]
        );
        let copies = rm.copies(&portraits);
        let data: Vec<Vec<u8>> =
            copies.iter().map(|&i| rm.get_in(i, &portraits).unwrap().into_owned()).collect();
        assert_eq!(data, [b"upper".to_vec(), b"lower".to_vec(), b"game".to_vec()]);
        assert_eq!(&*rm.get(&portraits).unwrap(), b"upper", "the game reads the highest");
        // A layer without it, and one past the last.
        let classes = key("classes", ResType::TWODA);
        assert!(matches!(rm.get_in(copies[0], &classes), Err(ResError::NotFound(_))));
        assert!(rm.get_in(99, &portraits).is_err());
    }

    #[test]
    fn higher_layers_win_and_list_is_distinct() {
        let mut low = MemContainer::new();
        low.insert(key("a", ResType::TWODA), &b"low"[..]);
        low.insert(key("b", ResType::TWODA), &b"b"[..]);
        let mut high = MemContainer::new();
        high.insert(key("A", ResType::TWODA), &b"high"[..]);
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "low", LayerClass::Key, low);
        rm.add(priority::OVERRIDE, "high", LayerClass::Directory, high);
        assert_eq!(rm.get(&key("a", ResType::TWODA)).unwrap().as_ref(), b"high");
        assert_eq!(rm.origin(&key("b", ResType::TWODA)), Some("low"));
        assert!(matches!(rm.get(&key("c", ResType::TWODA)), Err(ResError::NotFound(_))));
        let names: Vec<String> = rm.list(ResType::TWODA).iter().map(|r| r.to_string()).collect();
        assert_eq!(names.len(), 2);
        assert!(rm.remove("high").is_some());
        assert_eq!(rm.get(&key("a", ResType::TWODA)).unwrap().as_ref(), b"low");
    }

    #[test]
    fn texture_prefers_dds_within_a_class() {
        let mut hak1 = MemContainer::new();
        hak1.insert(key("tex", ResType::TGA), &b"tga-hak1"[..]);
        let mut hak2 = MemContainer::new();
        hak2.insert(key("tex", ResType::DDS), &b"dds-hak2"[..]);
        let mut bif = MemContainer::new();
        bif.insert(key("tex", ResType::DDS), &b"dds-bif"[..]);
        let mut ovr = MemContainer::new();
        ovr.insert(key("other", ResType::TGA), &b"tga-ovr"[..]);
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "base", LayerClass::Key, bif);
        rm.add(priority::HAK, "hak1", LayerClass::Erf, hak1);
        rm.add(priority::HAK, "hak2", LayerClass::Erf, hak2);
        rm.add(priority::OVERRIDE, "override", LayerClass::Directory, ovr);
        assert_eq!(
            rm.layers().iter().map(|l| l.label.as_str()).collect::<Vec<_>>(),
            ["hak1", "hak2", "override", "base"],
            "sorted by priority, equal priorities in insertion order"
        );
        let tex = ResRef::from_str("tex").unwrap();
        let (t, d) = rm.texture(tex).unwrap();
        assert_eq!((t, d.as_ref()), (ResType::DDS, &b"dds-hak2"[..]));
        // A TGA in a directory beats a DDS in a lower class, whatever the
        // priorities.
        let other = ResRef::from_str("other").unwrap();
        assert_eq!(rm.texture(other).unwrap().0, ResType::TGA);
    }

    #[test]
    fn layers_change_in_place() {
        let one = |data: &'static [u8]| {
            let mut c = MemContainer::new();
            c.insert(key("x", ResType::NSS), data);
            c
        };
        let mut rm = ResMan::new();
        rm.add(priority::HAK, "first", LayerClass::Erf, one(b"1"));
        rm.add(priority::HAK, "second", LayerClass::Erf, one(b"2"));
        assert!(rm.replace("first", one(b"one")).is_some());
        let labels: Vec<&str> = rm.layers().iter().map(|l| l.label.as_str()).collect();
        assert_eq!(labels, ["first", "second"], "the same place among equals");
        assert_eq!(rm.get(&key("x", ResType::NSS)).unwrap().as_ref(), b"one");
        assert!(rm.replace("third", one(b"3")).is_none());

        let dir = std::env::temp_dir().join(format!("mg-resman-rescan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        rm.add(priority::OVERRIDE, "folder", LayerClass::Directory, DirContainer::open(&dir));
        assert!(!rm.contains(&key("new", ResType::NSS)));
        std::fs::write(dir.join("new.nss"), b"void main() {}").unwrap();
        assert!(rm.rescan("folder").unwrap());
        assert!(rm.contains(&key("new", ResType::NSS)), "a file added since it was opened");
        std::fs::remove_file(dir.join("new.nss")).unwrap();
        rm.rescan("folder").unwrap();
        assert!(!rm.contains(&key("new", ResType::NSS)));
        assert!(!rm.rescan("nothing").unwrap());

        // An archive written again (Windows refuses to write a mapped file).
        if cfg!(windows) {
            std::fs::remove_dir_all(&dir).unwrap();
            return;
        }
        let hak = dir.join("test.hak");
        let write = |names: &[&str]| {
            let mut w = mg_erf::ErfWriter::new(*b"HAK ");
            for n in names {
                w.add(ResRef::from_str(n).unwrap(), ResType::NSS, &b"x"[..]).unwrap();
            }
            std::fs::write(&hak, w.to_bytes().unwrap()).unwrap();
        };
        write(&["a"]);
        rm.add(priority::HAK, "hak", LayerClass::Erf, ErfContainer::open(&hak).unwrap());
        write(&["a", "b"]);
        assert!(!rm.contains(&key("b", ResType::NSS)));
        rm.rescan("hak").unwrap();
        assert!(rm.contains(&key("b", ResType::NSS)));
        std::fs::write(&hak, b"not an archive").unwrap();
        assert!(rm.rescan("hak").is_err());
        assert!(rm.contains(&key("b", ResType::NSS)), "kept what it had");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A resource's first bytes are its bytes' first, from an archive (read
    /// alone) and from other containers alike.
    #[test]
    fn a_resource_s_first_bytes() {
        let dir = std::env::temp_dir().join(format!("mg-resman-start-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let hak = dir.join("x.hak");
        let mut w = mg_erf::ErfWriter::new(*b"HAK ");
        w.add(ResRef::from_str("long").unwrap(), ResType::MDL, &b"\0\0\0\0 and the rest"[..])
            .unwrap();
        w.add(ResRef::from_str("short").unwrap(), ResType::MDL, &b"ab"[..]).unwrap();
        std::fs::write(&hak, w.to_bytes().unwrap()).unwrap();
        let mut mem = MemContainer::new();
        mem.insert(key("mem", ResType::MDL), &b"newmodel m"[..]);
        let mut rm = ResMan::new();
        rm.add(priority::HAK, "hak", LayerClass::Erf, ErfContainer::open(&hak).unwrap());
        rm.add(priority::MODULE, "module", LayerClass::Erf, mem);
        for (name, n) in [("long", 12), ("long", 1000), ("short", 12), ("mem", 3), ("mem", 0)] {
            let k = key(name, ResType::MDL);
            let all = rm.get(&k).unwrap();
            assert_eq!(&rm.get_start(&k, n).unwrap()[..], &all[..n.min(all.len())], "{name} {n}");
        }
        assert!(rm.get_start(&key("none", ResType::MDL), 12).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn filenames() {
        assert_eq!(ResKey::from_filename("NW_Chicken.UTC"), Some(key("nw_chicken", ResType::UTC)));
        assert_eq!(ResKey::from_filename("a.b.2da"), Some(key("a.b", ResType::TWODA)));
        assert_eq!(ResKey::from_filename("readme"), None);
        assert_eq!(ResKey::from_filename("x.unknownext"), None);
        assert_eq!(ResKey::from_filename("waytoolongname_abcdef.utc"), None);
    }

    #[test]
    fn changed_folders_and_archives_are_reloaded() {
        let dir = std::env::temp_dir().join(format!("mg-resman-reload-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let folder = dir.join("override");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("a.2da"), "2DA V2.0\n").unwrap();
        let hak = dir.join("x.hak");
        let write_hak = |n: usize| {
            let mut w = mg_erf::ErfWriter::new(*b"HAK ");
            for i in 0..n {
                w.add(ResRef::from_str(&format!("r{i}")).unwrap(), ResType::TWODA, b"x".to_vec())
                    .unwrap();
            }
            std::fs::write(&hak, w.to_bytes().unwrap()).unwrap();
        };
        write_hak(1);
        let mut rm = ResMan::new();
        rm.add(priority::OVERRIDE, "override", LayerClass::Directory, DirContainer::open(&folder));
        rm.add(priority::HAK_USER, "hak:x", LayerClass::Erf, ErfContainer::open(&hak).unwrap());
        rm.add(priority::KEY, "mem", LayerClass::Key, MemContainer::new());
        assert!(rm.changed_layers(|_| true).is_empty());

        std::fs::write(folder.join("b.2da"), "2DA V2.0\n").unwrap();
        write_hak(2);
        let mut changed = rm.changed_layers(|_| true);
        changed.sort();
        assert_eq!(changed, ["hak:x", "override"]);
        assert!(!rm.contains(&ResKey::parse("b", ResType::TWODA).unwrap()));
        rm.reload_changed(|_| true).unwrap();
        assert!(rm.contains(&ResKey::parse("b", ResType::TWODA).unwrap()));
        assert!(rm.contains(&ResKey::parse("r1", ResType::TWODA).unwrap()));
        assert!(rm.changed_layers(|_| true).is_empty());
        std::fs::remove_file(folder.join("a.2da")).unwrap();
        assert_eq!(rm.changed_layers(|_| true), ["override"]);
        assert!(rm.changed_layers(|l| l.label != "override").is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
