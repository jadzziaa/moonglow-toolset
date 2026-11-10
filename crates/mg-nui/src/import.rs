//! Windows brought into a module from files, folders, ERF archives and
//! zip files (`mg nui import`, and the NUI Creator's Import…). A window is
//! its layout (NAME.jui) with what belongs to it beside it: its settings
//! (NAME.txt, when it reads as the Creator's) and its event script's
//! source (NAME_e.nss, which holds the code of its handlers). The opener
//! and compiled scripts are left: Build writes them for the module they
//! are in.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;

use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_module::Module;
use mg_resman::ResKey;

use crate::{NuiError, Settings, Severity, check_name, parse, validate};

/// Past this a file is not a window's (a zip's entry is read no further).
pub const MOST: u64 = 64 * 1024 * 1024;

/// The resources found, each with where it came from.
pub type Found = BTreeMap<ResKey, (Vec<u8>, String)>;

/// The kinds of resource a window brings.
fn kept(ty: ResType) -> bool {
    matches!(ty, ResType::JUI | ResType::TXT | ResType::NSS)
}

fn failed(what: impl std::fmt::Display) -> NuiError {
    NuiError::Import(what.to_string())
}

/// Adds the windows' resources in `path` to `found`: a .jui file and what
/// is beside it, the .jui, .txt and .nss files of a folder, or the windows
/// of an ERF archive (.erf, .hak, .mod, .nwm) or a zip file. One resource
/// found twice, not the same, is refused.
///
/// Of an archive, only what a window is made of is read: each layout, and
/// the settings and event script of its name (and of the layouts `found`
/// has already). A hak of gigabytes or a module of 20,000 scripts is not
/// read whole to take two windows from it.
pub fn gather(path: &Path, found: &mut Found) -> Result<(), NuiError> {
    let shown = |p: &Path| p.display().to_string();
    // What of an archive that lists `keys` belongs to a window.
    let had: Vec<ResKey> = found.keys().filter(|k| k.restype == ResType::JUI).copied().collect();
    let of_windows = |keys: &mut dyn Iterator<Item = ResKey>| -> BTreeSet<ResKey> {
        let listed: BTreeSet<ResKey> = keys.filter(|k| kept(k.restype)).collect();
        let layouts = listed.iter().chain(&had).filter(|k| k.restype == ResType::JUI);
        let mut wanted = BTreeSet::new();
        for jui in layouts {
            let name = jui.resref.to_string();
            let beside = [
                Some(*jui),
                ResKey::parse(&name, ResType::TXT),
                ResKey::parse(&format!("{name}_e"), ResType::NSS),
            ];
            wanted.extend(beside.into_iter().flatten().filter(|k| listed.contains(k)));
        }
        wanted
    };
    let mut add = |key: ResKey, bytes: Vec<u8>, from: String| -> Result<(), NuiError> {
        match found.get(&key) {
            Some((had, was)) if *had != bytes => {
                Err(failed(format!("{key} is in {was} and in {from}, not the same: bring one")))
            }
            _ => {
                found.insert(key, (bytes, from));
                Ok(())
            }
        }
    };
    let read = |p: &Path| -> Result<Vec<u8>, NuiError> {
        let size = std::fs::metadata(p).map_err(|e| failed(format!("{}: {e}", shown(p))))?.len();
        if size > MOST {
            return Err(failed(format!("{}: {size} bytes is more than a window's file", shown(p))));
        }
        std::fs::read(p).map_err(|e| failed(format!("{}: {e}", shown(p))))
    };
    if path.is_dir() {
        let entries =
            std::fs::read_dir(path).map_err(|e| failed(format!("{}: {e}", shown(path))))?;
        let mut files: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        files.sort();
        for file in files.iter().filter(|f| f.is_file()) {
            let name = file.file_name().unwrap_or_default().to_string_lossy().to_string();
            if let Some(key) = ResKey::from_filename(&name).filter(|k| kept(k.restype)) {
                add(key, read(file)?, shown(file))?;
            }
        }
        return Ok(());
    }
    let extension =
        path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    match extension.as_str() {
        "jui" => {
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let jui = ResKey::from_filename(&name)
                .ok_or_else(|| failed(format!("{name:?} is not a resource's name")))?;
            add(jui, read(path)?, shown(path))?;
            let stem = jui.resref.to_string();
            for file in [format!("{stem}.txt"), format!("{stem}_e.nss")] {
                let beside = path.with_file_name(&file);
                if let (true, Some(key)) = (beside.is_file(), ResKey::from_filename(&file)) {
                    add(key, read(&beside)?, shown(&beside))?;
                }
            }
        }
        "zip" => {
            let file =
                std::fs::File::open(path).map_err(|e| failed(format!("{}: {e}", shown(path))))?;
            let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file))
                .map_err(|e| failed(format!("{}: not a zip file: {e}", shown(path))))?;
            // (Its folders are not the window's: the file's name is.)
            let named = |full: &str| {
                ResKey::from_filename(full.rsplit(['/', '\\']).next().unwrap_or_default())
            };
            let names: Vec<String> = zip.file_names().map(str::to_string).collect();
            let wanted = of_windows(&mut names.iter().filter_map(|n| named(n)));
            for i in 0..zip.len() {
                let mut entry = zip
                    .by_index(i)
                    .map_err(|e| failed(format!("{}: entry {i}: {e}", shown(path))))?;
                if entry.is_dir() {
                    continue;
                }
                let full = entry.name().to_string();
                let Some(key) = named(&full).filter(|k| wanted.contains(k)) else {
                    continue;
                };
                let mut data = Vec::new();
                (&mut entry)
                    .take(MOST + 1)
                    .read_to_end(&mut data)
                    .map_err(|e| failed(format!("{}: {full}: {e}", shown(path))))?;
                if data.len() as u64 > MOST {
                    return Err(failed(format!(
                        "{}: {full}: more than a window's file",
                        shown(path)
                    )));
                }
                add(key, data, format!("{}:{full}", shown(path)))?;
            }
        }
        _ => {
            // (Its table read, and then each window's files from the file
            // where they lie.)
            let erf = mg_resman::ErfContainer::open(path).map_err(|e| {
                failed(format!(
                    "{}: not an ERF archive (.erf, .hak, .mod), a zip, a folder or a .jui: {e}",
                    shown(path)
                ))
            })?;
            let wanted = of_windows(&mut erf.index().iter().map(|(key, _)| *key));
            for &(key, size) in erf.index().iter().filter(|(key, _)| wanted.contains(key)) {
                if size > MOST {
                    return Err(failed(format!(
                        "{}: {key}: {size} bytes is more than a window's file",
                        shown(path)
                    )));
                }
                let data = erf.read_anywhere(&key).map_err(|e| failed(format!("{key}: {e}")))?;
                add(key, data.into_owned(), shown(path))?;
            }
        }
    }
    Ok(())
}

/// A window found, as it would come into a module.
#[derive(Debug, Clone)]
pub struct Incoming {
    /// Its name: its layout's ResRef.
    pub name: String,
    /// Where its layout was found.
    pub from: String,
    /// What it brings: its layout, and its settings and event script's
    /// source where they were found.
    pub resources: Vec<ResKey>,
    /// Those of them the module has, otherwise than found: replaced.
    pub taken: Vec<ResKey>,
    /// What its checks find ([`validate`]).
    pub errors: usize,
    pub warnings: usize,
    /// What is to be said of it: settings beside it that aren't the
    /// Creator's (left), a name Build can't make scripts of.
    pub notes: Vec<String>,
    edits: Vec<Edit>,
}

/// The windows among `found` (a layout each, with its settings and event
/// script's source), as they would come into `module`. None is an error.
pub fn incoming(module: &Module, found: &Found) -> Result<Vec<Incoming>, NuiError> {
    let names: Vec<String> = (found.keys())
        .filter(|k| k.restype == ResType::JUI)
        .map(|k| k.resref.to_string())
        .collect();
    if names.is_empty() {
        return Err(failed("no NUI window (a .jui) there"));
    }
    let mut windows = Vec::new();
    for name in names {
        let key = |ty| ResKey::parse(&name, ty);
        let jui = key(ResType::JUI).expect("a found window's key");
        let (layout, from) = &found[&jui];
        let window = parse(layout).map_err(|e| failed(format!("{from}: {jui}: {e}")))?;
        let mut resources = vec![jui];
        let mut notes = Vec::new();
        let mut settings = Settings::default();
        if let Some(txt) = key(ResType::TXT)
            && let Some((bytes, from)) = found.get(&txt)
        {
            // (A note of someone's beside a layout isn't its settings.)
            match Settings::parse(bytes) {
                Ok(s) => {
                    settings = s;
                    resources.push(txt);
                }
                Err(e) => notes.push(format!("{from}: {txt} left: {e}")),
            }
        }
        if let Some(events) = ResKey::parse(&format!("{name}_e"), ResType::NSS)
            && found.contains_key(&events)
        {
            resources.push(events);
        }
        let mut taken = Vec::new();
        let mut edits = Vec::new();
        for k in &resources {
            let bytes = &found[k].0;
            if module.get(k) != Some(&bytes[..]) {
                if module.contains(k) {
                    taken.push(*k);
                }
                edits.push(Edit::SetResource { key: *k, data: Some(bytes.clone()) });
            }
        }
        if check_name(&name).is_err() {
            notes.push(format!(
                "{name}: a name over 14 letters, or with others than lower case ones, digits and \
                 _, has no room for its scripts' names: Build can't make them"
            ));
        }
        let findings = validate(&window, &settings);
        let errors = findings.iter().filter(|d| d.severity == Severity::Error).count();
        windows.push(Incoming {
            name,
            from: from.clone(),
            resources,
            taken,
            errors,
            warnings: findings.len() - errors,
            notes,
            edits,
        });
    }
    Ok(windows)
}

/// The windows brought in, as one edit (one Undo step): only what differs
/// from what the module has.
pub fn command(windows: &[Incoming]) -> Command {
    Command::new("Import NUI windows", windows.iter().flat_map(|w| w.edits.clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `found` holds, for a test's message.
    fn listed(found: &Found) -> Vec<String> {
        let mut names: Vec<String> = found.keys().map(ToString::to_string).collect();
        names.sort();
        names
    }

    #[test]
    fn a_window_s_files_are_found_beside_it_and_others_left() {
        let dir = mg_testkit::scratch_dir("mg-nui-gather");
        for (name, data) in [
            ("crier.jui", "{}"),
            ("crier.txt", "{}"),
            ("crier_e.nss", "void main() {}"),
            ("crier_o.nss", "// generated"),
            ("readme.md", "notes"),
        ] {
            std::fs::write(dir.join(name), data).unwrap();
        }
        let mut found = Found::new();
        gather(&dir.join("crier.jui"), &mut found).unwrap();
        assert_eq!(listed(&found), ["crier.jui", "crier.txt", "crier_e.nss"]);
        // The folder: every resource of a kind a window has.
        let mut found = Found::new();
        gather(&dir, &mut found).unwrap();
        assert_eq!(listed(&found), ["crier.jui", "crier.txt", "crier_e.nss", "crier_o.nss"]);
        // Two different copies of one: refused.
        let other = mg_testkit::scratch_dir("mg-nui-gather-other");
        std::fs::write(other.join("crier.jui"), "{\"version\":1}").unwrap();
        let said = gather(&other, &mut found).unwrap_err().to_string();
        assert!(said.contains("crier.jui is in"), "{said}");
        // Not an archive: said, no panic.
        std::fs::write(other.join("junk.hak"), b"HAK V1.0\0\0").unwrap();
        assert!(gather(&other.join("junk.hak"), &mut Found::new()).is_err());
        std::fs::write(other.join("junk.zip"), b"PK\x03\x04 cut").unwrap();
        assert!(gather(&other.join("junk.zip"), &mut Found::new()).is_err());
        assert!(gather(&other.join("missing.jui"), &mut Found::new()).is_err());
    }

    /// Of an archive with one window among a thousand scripts, the
    /// window's three files are read (every script and text file of it
    /// was, after the whole file); a script of a layout brought before
    /// is too.
    #[test]
    fn an_archive_gives_its_windows_files_alone() {
        use std::io::Write;
        let dir = mg_testkit::scratch_dir("mg-nui-gather-archive");
        let files: Vec<(String, Vec<u8>)> = [
            ("crier.jui", "{}"),
            ("crier.txt", "{}"),
            ("crier_e.nss", "void main() {}"),
            ("crier_o.nss", "// generated"),
            ("town_e.nss", "void main() { /* town */ }"),
            ("notes.txt", "a note"),
        ]
        .into_iter()
        .map(|(n, d)| (n.to_string(), d.as_bytes().to_vec()))
        .chain((0..1000).map(|i| (format!("script{i:04}.nss"), b"void main() {}".to_vec())))
        .collect();
        let mut erf = mg_erf::ErfWriter::new(*b"HAK ");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(dir.join("all.zip")).unwrap());
        for (name, data) in &files {
            let key = ResKey::from_filename(name).unwrap();
            erf.add(key.resref, key.restype, &data[..]).unwrap();
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file(format!("src/{name}"), options).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
        std::fs::write(dir.join("all.hak"), erf.to_bytes().unwrap()).unwrap();
        for archive in ["all.hak", "all.zip"] {
            let mut found = Found::new();
            gather(&dir.join(archive), &mut found).unwrap();
            assert_eq!(listed(&found), ["crier.jui", "crier.txt", "crier_e.nss"], "{archive}");
            assert_eq!(found[&ResKey::from_filename("crier_e.nss").unwrap()].0, b"void main() {}");
            // With the town's layout found before, its script comes too.
            let mut found = Found::new();
            let town = ResKey::from_filename("town.jui").unwrap();
            found.insert(town, (b"{}".to_vec(), "elsewhere".into()));
            gather(&dir.join(archive), &mut found).unwrap();
            assert_eq!(
                listed(&found),
                ["crier.jui", "crier.txt", "crier_e.nss", "town.jui", "town_e.nss"],
                "{archive}"
            );
        }
    }

    /// A window comes in with its settings and event script; a resource the
    /// module has otherwise is taken (replaced), one it has as found is
    /// left as it is, and settings that aren't the Creator's are left and
    /// said.
    #[test]
    fn windows_come_in_with_what_belongs_to_them() {
        let key = |n: &str| ResKey::from_filename(n).unwrap();
        let layout = serde_json::to_vec(&crate::window()).unwrap();
        let mut found = Found::new();
        for (name, bytes) in [
            ("crier.jui", layout.clone()),
            ("crier.txt", Settings::default().bytes()),
            ("crier_e.nss", b"void main() {}".to_vec()),
            ("notes.jui", layout.clone()),
            ("notes.txt", b"a note".to_vec()),
        ] {
            found.insert(key(name), (bytes, "here".into()));
        }
        let mut m = Module::new();
        m.set(key("crier.txt"), b"{\"format\": \"moonglow.nui/1\"}".to_vec());
        m.set(key("notes.jui"), layout);
        let w = incoming(&m, &found).unwrap();
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].name, "crier");
        assert_eq!(w[0].resources, [key("crier.jui"), key("crier.txt"), key("crier_e.nss")]);
        assert_eq!(w[0].taken, [key("crier.txt")]);
        assert_eq!((w[0].errors, w[0].notes.len()), (0, 0));
        assert_eq!(
            (w[1].resources.as_slice(), w[1].taken.as_slice()),
            (&[key("notes.jui")][..], &[][..])
        );
        assert!(w[1].notes[0].starts_with("here: notes.txt left"), "{:?}", w[1].notes);
        let c = command(&w);
        let keys: Vec<_> = c.edits.iter().map(|e| e.key().to_string()).collect();
        assert_eq!(keys, ["crier.jui", "crier.txt", "crier_e.nss"]);
        assert!(incoming(&m, &Found::new()).is_err());
    }
}
