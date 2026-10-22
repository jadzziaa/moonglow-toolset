//! `mg nui import`: windows brought into a module from files, folders, ERF
//! archives and zip files. A window is its layout (NAME.jui) with what
//! belongs to it beside it: its settings (NAME.txt, when it reads as the
//! Creator's) and its event script's source (NAME_e.nss, which holds the
//! code of its handlers). The opener and compiled scripts are left: Build
//! (`mg nui generate`) writes them for the module they are in.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use mg_core::ResType;
use mg_edit::{Command, Edit, Workspace};
use mg_module::Module;
use mg_nui::Severity;
use mg_resman::ResKey;
use serde_json::json;

use crate::{Output, extension, path_text};

/// Past this a file is not a window's (a zip's entry is read no further).
const MOST: u64 = 64 * 1024 * 1024;

/// The resources found, each with where it came from.
type Found = BTreeMap<ResKey, (Vec<u8>, String)>;

/// `mg nui import`.
pub(crate) fn import(
    module: &Path,
    sources: &[std::path::PathBuf],
    replace: bool,
    dry_run: bool,
) -> Result<Output> {
    let mut found = Found::new();
    for source in sources {
        gather(source, &mut found).with_context(|| path_text(source))?;
    }
    let names: Vec<String> = (found.keys())
        .filter(|k| k.restype == ResType::JUI)
        .map(|k| k.resref.to_string())
        .collect();
    if names.is_empty() {
        bail!(
            "no NUI window (a .jui) in {}",
            sources.iter().map(|s| path_text(s)).collect::<Vec<_>>().join(", ")
        );
    }
    let mut ws = Workspace::new(Module::open(module)?);
    let mut out = Output::default();
    let mut edits = Vec::new();
    let mut windows = Vec::new();
    let mut taken = Vec::new();
    for name in &names {
        let key = |ty| ResKey::parse(name, ty);
        let jui = key(ResType::JUI).expect("a found window's key");
        let (layout, from) = &found[&jui];
        let window = mg_nui::parse(layout).with_context(|| format!("{from}: {jui}"))?;
        let mut keys = vec![jui];
        let mut settings = mg_nui::Settings::default();
        if let Some(txt) = key(ResType::TXT)
            && let Some((bytes, from)) = found.get(&txt)
        {
            // (A note of someone's beside a layout isn't its settings.)
            match mg_nui::Settings::parse(bytes) {
                Ok(s) => {
                    settings = s;
                    keys.push(txt);
                }
                Err(e) => out.note(format!("warning: {from}: {txt} left: {e}")),
            }
        }
        if let Some(events) = ResKey::parse(&format!("{name}_e"), ResType::NSS)
            && found.contains_key(&events)
        {
            keys.push(events);
        }
        for k in &keys {
            if ws.module.contains(k) && ws.module.get(k) != Some(&found[k].0[..]) {
                taken.push(k.to_string());
            }
            if ws.module.get(k) != Some(&found[k].0[..]) {
                edits.push(Edit::SetResource { key: *k, data: Some(found[k].0.clone()) });
            }
        }
        let findings = mg_nui::validate(&window, &settings);
        let errors = findings.iter().filter(|d| d.severity == Severity::Error).count();
        if errors > 0 {
            out.note(format!(
                "warning: {jui} has {errors} error(s): mg nui validate {} {name} lists them",
                path_text(module)
            ));
        }
        if mg_nui::check_name(name).is_err() {
            out.note(format!(
                "warning: {name}: a name over 14 letters, or with others than lower case ones, digits and _, has no room for its scripts' names: Build can't make them"
            ));
        }
        windows.push(json!({
            "window": name,
            "from": from,
            "resources": keys.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "errors": errors,
            "warnings": findings.len() - errors,
        }));
    }
    if !taken.is_empty() && !replace {
        bail!(
            "the module has {} already: --replace to replace {}",
            taken.join(", "),
            if taken.len() == 1 { "it" } else { "them" }
        );
    }
    let resources: Vec<ResKey> = edits.iter().map(|e| *e.key()).collect();
    let left = super::left_out(&ws.module, &resources);
    let saved = !dry_run && !edits.is_empty();
    if !edits.is_empty() {
        ws.apply(Command::new("Import NUI windows", edits))?;
    }
    if saved {
        ws.save()?;
    }
    out.json = json!({
        "module": path_text(module),
        "windows": windows,
        "resources": resources.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "saved": saved,
        "dry_run": dry_run,
    });
    if let Some(left) = left {
        out.note(format!("warning: {}", left.said(&ws.module)));
        out.json["left_out"] = left.files(&ws.module).into();
        out.json["include"] = left.line.into();
    }
    for r in &resources {
        out.line(r.to_string());
    }
    let what = if dry_run { "would change" } else { "changed" };
    let plural = |n: usize, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
    out.line(format!(
        "{}: {what} {}",
        plural(names.len(), "window"),
        plural(resources.len(), "resource")
    ));
    if saved {
        out.note(format!(
            "mg nui generate {} NAME writes each window's scripts",
            path_text(module)
        ));
    }
    Ok(out)
}

/// The windows' resources in `path`: a .jui file and what is beside it, or
/// the .jui, .txt and .nss resources of a folder, an ERF or a zip.
fn gather(path: &Path, found: &mut Found) -> Result<()> {
    let kept = |ty: ResType| matches!(ty, ResType::JUI | ResType::TXT | ResType::NSS);
    let mut add = |key: ResKey, bytes: Vec<u8>, from: String| -> Result<()> {
        match found.get(&key) {
            Some((had, was)) if *had != bytes => {
                bail!("{key} is in {was} and in {from}, not the same: bring one")
            }
            _ => {
                found.insert(key, (bytes, from));
                Ok(())
            }
        }
    };
    let read = |p: &Path| -> Result<Vec<u8>> {
        let size = std::fs::metadata(p).with_context(|| path_text(p))?.len();
        if size > MOST {
            bail!("{}: {size} bytes is more than a window's file", path_text(p));
        }
        std::fs::read(p).with_context(|| path_text(p))
    };
    if path.is_dir() {
        let mut files: Vec<_> =
            std::fs::read_dir(path)?.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        files.sort();
        for file in files.iter().filter(|f| f.is_file()) {
            let name = file.file_name().unwrap_or_default().to_string_lossy().to_string();
            if let Some(key) = ResKey::from_filename(&name).filter(|k| kept(k.restype)) {
                add(key, read(file)?, path_text(file))?;
            }
        }
        return Ok(());
    }
    match extension(path).as_str() {
        "jui" => {
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let jui = ResKey::from_filename(&name)
                .ok_or_else(|| anyhow!("{name:?} is not a resource's name"))?;
            add(jui, read(path)?, path_text(path))?;
            let stem = jui.resref.to_string();
            for (file, ty) in
                [(format!("{stem}.txt"), ResType::TXT), (format!("{stem}_e.nss"), ResType::NSS)]
            {
                let beside = path.with_file_name(&file);
                if let (true, Some(key)) = (beside.is_file(), ResKey::from_filename(&file)) {
                    debug_assert_eq!(key.restype, ty);
                    add(key, read(&beside)?, path_text(&beside))?;
                }
            }
        }
        "zip" => {
            let bytes = read_any(path)?;
            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
                .map_err(|e| anyhow!("not a zip file: {e}"))?;
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i).map_err(|e| anyhow!("entry {i}: {e}"))?;
                if entry.is_dir() {
                    continue;
                }
                // (Its folders are not the window's: the file's name is.)
                let full = entry.name().to_string();
                let name = full.rsplit(['/', '\\']).next().unwrap_or_default().to_string();
                let Some(key) = ResKey::from_filename(&name).filter(|k| kept(k.restype)) else {
                    continue;
                };
                let mut data = Vec::new();
                (&mut entry)
                    .take(MOST + 1)
                    .read_to_end(&mut data)
                    .map_err(|e| anyhow!("{full}: {e}"))?;
                if data.len() as u64 > MOST {
                    bail!("{full}: more than a window's file");
                }
                add(key, data, format!("{}:{full}", path_text(path)))?;
            }
        }
        _ => {
            let bytes = read_any(path)?;
            let erf = mg_erf::Erf::read(&bytes).map_err(|e| {
                anyhow!("not an ERF archive (.erf, .hak, .mod), a zip, a folder or a .jui: {e}")
            })?;
            for entry in erf.entries.iter().filter(|e| kept(e.restype)) {
                let key = ResKey::new(entry.resref, entry.restype);
                let data = erf.data(entry).map_err(|e| anyhow!("{key}: {e}"))?;
                add(key, data.into_owned(), path_text(path))?;
            }
        }
    }
    Ok(())
}

/// An archive's bytes, of any size.
fn read_any(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).with_context(|| path_text(path))
}

/// What `found` holds, for a test's message.
#[cfg(test)]
fn listed(found: &Found) -> Vec<String> {
    let mut names: Vec<String> = found.keys().map(ToString::to_string).collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
