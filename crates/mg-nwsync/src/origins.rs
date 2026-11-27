//! A manifest's origins file (`manifests/<sha1>.origin`), which
//! `nwn_nwsync_write --write-origins` writes beside it: each container a
//! resource of the manifest came from, with the resources it gave, the
//! copy of each name that won:
//!
//! ```text
//! Erf:/srv/haks/pw_parts.hak
//!     pfh0_head001.mdl
//!     pfh0_head001.plt
//! ResFile:/srv/tlk/world.tlk
//!     world.tlk
//! ```
//!
//! A container's line is its kind and the file it was opened from, as the
//! writer opened it (a whole path, or a name alone); a resource's is its
//! name, a dot and its type's extension (a number for a type with none),
//! after a tab (spaces above). The game reads none of it; it tells which
//! of a module's haks a manifest holds.

/// One container of an origins file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// Its kind (`Erf`, `ResFile`, `ResDir`, ...).
    pub kind: String,
    /// The file or folder it was opened from, as written.
    pub path: String,
    /// The resources it gave (`name.ext`), in the file's order.
    pub resources: Vec<String>,
}

impl Origin {
    /// The file's name alone, without its folders (either system's).
    pub fn file_name(&self) -> &str {
        self.path.rsplit(['/', '\\']).next().unwrap_or(&self.path)
    }

    /// The hak's name (its file's, without `.hak`), where it is a hak.
    pub fn hak(&self) -> Option<&str> {
        let name = self.file_name();
        let (stem, ext) = name.rsplit_once('.')?;
        (self.kind == "Erf" && ext.eq_ignore_ascii_case("hak")).then_some(stem)
    }
}

/// An origins file read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Origins {
    pub containers: Vec<Origin>,
}

impl Origins {
    /// Reads the file's text. Lines it can't place (a resource before any
    /// container) are left out: the file is the writer's description, not
    /// something the game reads.
    pub fn parse(text: &str) -> Origins {
        let mut containers: Vec<Origin> = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if line.starts_with(['\t', ' ']) {
                if let Some(c) = containers.last_mut() {
                    c.resources.push(trimmed.to_string());
                }
                continue;
            }
            let (kind, path) = trimmed.split_once(':').unwrap_or(("", trimmed));
            containers.push(Origin {
                kind: kind.to_string(),
                path: path.to_string(),
                resources: Vec::new(),
            });
        }
        Origins { containers }
    }

    /// The hak of this name (case ignored, as the game finds haks), where
    /// the file lists it.
    pub fn hak(&self, name: &str) -> Option<&Origin> {
        self.containers.iter().find(|c| c.hak().is_some_and(|h| h.eq_ignore_ascii_case(name)))
    }

    /// The haks the file lists, by name.
    pub fn haks(&self) -> impl Iterator<Item = &str> {
        self.containers.iter().filter_map(Origin::hak)
    }

    /// Every resource the file lists.
    pub fn resources(&self) -> usize {
        self.containers.iter().map(|c| c.resources.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file as neverwinter.nim writes it: containers by their whole
    /// paths (a Windows one too), resources by name and extension, a type
    /// without one by its number.
    #[test]
    fn origins_are_read() {
        let text = "Erf:/srv/pw/hak/pw_parts.hak\n\tpfh0_head001.mdl\n\tpfh0_head001.plt\n\
                    Erf:C:\\NWN\\hak\\PW_VFX.HAK\r\n\tvfx_fire.mdl\r\n\
                    ResFile:/srv/pw/tlk/world.tlk\n\tworld.tlk\n\
                    Erf:sounds.erf\n\tsnd_a.wav\n\tthing.9999\n";
        let o = Origins::parse(text);
        assert_eq!(o.containers.len(), 4);
        assert_eq!(o.containers[0].path, "/srv/pw/hak/pw_parts.hak");
        assert_eq!(o.containers[0].resources, ["pfh0_head001.mdl", "pfh0_head001.plt"]);
        assert_eq!(o.containers[1].file_name(), "PW_VFX.HAK");
        assert_eq!(o.containers[1].resources, ["vfx_fire.mdl"]);
        assert_eq!(o.haks().collect::<Vec<_>>(), ["pw_parts", "PW_VFX"]);
        assert!(o.hak("PW_Parts").is_some());
        assert!(o.hak("pw_vfx").is_some());
        // (A talk table and an .erf are no haks.)
        assert!(o.hak("world").is_none() && o.hak("sounds").is_none());
        assert_eq!(o.containers[2].kind, "ResFile");
        assert_eq!(o.containers[3].resources[1], "thing.9999");
        assert_eq!(o.resources(), 6);
    }

    /// An empty file, blank lines and a resource before any container.
    #[test]
    fn odd_files_are_read_as_far_as_they_go() {
        assert_eq!(Origins::parse(""), Origins::default());
        let o = Origins::parse("\tstray.mdl\n\nErf:x.hak\n\n\ta.mdl\nnocolon.hak\n");
        assert_eq!(o.containers.len(), 2);
        assert_eq!(o.containers[0].resources, ["a.mdl"]);
        assert_eq!(o.containers[1].kind, "");
        assert_eq!(o.containers[1].hak(), None);
    }
}
