//! nasher source trees: every shipped module unpacked by nasher itself must
//! give, file for file, the text Moonglow writes for the same resources, and
//! Moonglow must read each file back to a resource it writes as the same
//! text (so an untouched tree saves without changes). Opened as a project,
//! edited and saved, a tree must be what nasher unpacks from the edited
//! module.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use mg_core::{ResRef, ResType};
use mg_erf::Erf;
use mg_gff::Gff;
use mg_gff::Value;
use mg_module::nasher::Conversion;
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;
use mg_testkit::{bundled_modules, corpus, nwn_tool, oracle_tool, scratch_dir};
use rayon::prelude::*;

/// Unpacks `module` into the project at `dir` with nasher (made with its
/// default rules, everything into `src`, if there is none), removing the
/// sources of resources the module doesn't have.
fn nasher_unpack(nasher: &Path, module: &Path, dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    if !dir.join("nasher.cfg").exists() {
        std::fs::write(
            dir.join("nasher.cfg"),
            "[package]\n  [package.sources]\n  include = \"src/**/*.{nss,json}\"\n\n  \
             [package.rules]\n  \"*\" = \"src\"\n\n[target]\nname = \"default\"\nfile = \"m.mod\"\n",
        )
        .unwrap();
    }
    let tool = |n: &str| nwn_tool(n).unwrap().display().to_string();
    let out = Command::new(nasher)
        .current_dir(dir)
        // nasher installs nothing when unpacking a given file; keep it away
        // from the real user folder all the same.
        .env("NWN_HOME", dir.join("home"))
        .args([
            "unpack".to_string(),
            format!("--file:{}", module.display()),
            "--yes".into(),
            "--removeDeleted".into(),
            format!("--gffUtil:{}", tool("nwn_gff")),
            format!("--erfUtil:{}", tool("nwn_erf")),
            format!("--tlkUtil:{}", tool("nwn_tlk")),
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "nasher unpack {}: {}", module.display(), {
        String::from_utf8_lossy(&out.stdout)
    });
}

/// The first line where two texts differ, for messages.
fn first_difference(a: &str, b: &str) -> String {
    for (i, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}: ours {x:?}, nasher's {y:?}", i + 1);
        }
    }
    format!("lengths {} and {}", a.len(), b.len())
}

/// Compares one unpacked module; returns the problems found.
fn check_module(nasher: &Path, module: &Path, scratch: &Path) -> (usize, Vec<String>) {
    let name = module.file_stem().unwrap().to_string_lossy().to_string();
    let dir = scratch.join(name.replace(' ', "_"));
    nasher_unpack(nasher, module, &dir);
    let data = std::fs::read(module).unwrap();
    let erf = Erf::read(&data).unwrap();
    let conv = Conversion::default();
    let mut problems = Vec::new();
    let mut checked = 0;
    for entry in std::fs::read_dir(dir.join("src")).unwrap().flatten() {
        let path = entry.path();
        let file = path.file_name().unwrap().to_string_lossy().to_string();
        let Some(res) = file.strip_suffix(".json") else { continue };
        let key = ResKey::from_filename(res).unwrap();
        let Some(e) =
            erf.entries.iter().find(|e| e.resref == key.resref && e.restype == key.restype)
        else {
            problems.push(format!("{name}/{file}: not in the module"));
            continue;
        };
        let theirs = std::fs::read_to_string(&path).unwrap();
        let gff = Gff::read(&erf.data(e).unwrap()).unwrap();
        let ours = match conv.to_source(key.restype, &gff) {
            Ok(t) => t,
            Err(err) => {
                problems.push(format!("{name}/{file}: {err}"));
                continue;
            }
        };
        if ours != theirs {
            problems.push(format!("{name}/{file}: {}", first_difference(&ours, &theirs)));
            continue;
        }
        // Read back and written again, the text is unchanged.
        match conv.from_source(&file, theirs.as_bytes()) {
            Ok(back) => {
                let again = conv.to_source(key.restype, &back).unwrap();
                if again != theirs {
                    problems.push(format!(
                        "{name}/{file}: not stable: {}",
                        first_difference(&again, &theirs)
                    ));
                }
            }
            Err(err) => problems.push(format!("{name}/{file}: reading back: {err}")),
        }
        checked += 1;
    }
    (checked, problems)
}

#[test]
fn moonglow_writes_what_nasher_unpacks() {
    let root = corpus!();
    let nasher = oracle_tool!("nasher");
    let _ = oracle_tool!("nwn_gff");
    let scratch = scratch_dir("nasher-unpack");
    let modules = bundled_modules(&root);
    let results: Vec<_> = modules.par_iter().map(|m| check_module(&nasher, m, &scratch)).collect();
    let checked: usize = results.iter().map(|r| r.0).sum();
    let problems: Vec<&String> = results.iter().flat_map(|r| &r.1).collect();
    eprintln!("{checked} files of {} modules agree with nasher", modules.len());
    assert!(
        problems.is_empty(),
        "{} of {checked} files differ:\n{}",
        problems.len(),
        problems.iter().take(30).map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
    );
}

/// Every file under `dir/src`, by path relative to `dir`.
fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(base: &Path, d: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                out.insert(p.strip_prefix(base).unwrap().to_path_buf(), std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, &dir.join("src"), &mut out);
    out
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

/// Opens nasher's tree of `module` as a project, saves it untouched (nothing
/// may be written), then edits it, saves, and compares the tree with nasher's
/// unpack of the edited module into the original tree.
fn edit_project(nasher: &Path, module: &Path, scratch: &Path) -> Vec<String> {
    let name = module.file_stem().unwrap().to_string_lossy().replace(' ', "_");
    let dir = scratch.join(&name);
    nasher_unpack(nasher, module, &dir);
    let expected = scratch.join(format!("{name}-nasher"));
    copy_dir(&dir, &expected);
    let before = tree(&dir);

    let mut m = Module::open(&dir).unwrap();
    let mut problems = Vec::new();
    let project = m.project.as_ref().unwrap();
    for w in &project.warnings {
        problems.push(format!("{name}: warning: {w}"));
    }
    // Every source file is a resource; .ncs files aren't in the tree.
    let original = Module::open(module).unwrap();
    let want: Vec<ResKey> =
        original.keys().filter(|k| k.restype != ResType::NCS).copied().collect();
    let mut have: Vec<ResKey> = m.keys().copied().collect();
    have.sort();
    let mut want_sorted = want.clone();
    want_sorted.sort();
    if have != want_sorted {
        problems.push(format!(
            "{name}: {} resources read, {} in the module",
            have.len(),
            want.len()
        ));
    }

    m.save().unwrap();
    if tree(&dir) != before {
        problems.push(format!("{name}: saving untouched changed the tree"));
    }

    // Edit: the module's name, an object in an area moved, a script added and
    // one removed.
    let mut info = m.info().unwrap();
    let Some(Value::LocString(ls)) = info.root.get_mut("Mod_Name") else { panic!("no Mod_Name") };
    ls.set(mg_core::Language::ENGLISH, mg_core::Gender::Male, "Edited in Moonglow");
    m.set_info(&info).unwrap();
    let git = m.keys_of(ResType::GIT).next().copied();
    if let Some(git) = git {
        let mut g = m.gff(&git).unwrap().unwrap();
        for list in ["Placeable List", "Creature List", "WaypointList"] {
            if let Some(Value::List(items)) = g.root.get_mut(list)
                && let Some(first) = items.first_mut()
                && let Some(Value::Float(x)) = first.get_mut("XPosition")
            {
                *x += 1.25;
                break;
            }
        }
        m.set_gff(git, &g).unwrap();
    }
    m.set(
        ResKey::new(ResRef::from_str("mg_added").unwrap(), ResType::NSS),
        b"void main() {}\n".to_vec(),
    );
    let nss = m.keys_of(ResType::NSS).next().copied();
    if let Some(nss) = nss {
        m.remove(&nss);
    }
    m.save().unwrap();

    // nasher's unpack of the same module into the original tree.
    let edited = scratch.join(format!("{name}-edited.mod"));
    m.save_as(&ModuleLocation::Archive(edited.clone())).unwrap();
    nasher_unpack(nasher, &edited, &expected);
    let (ours, theirs) = (tree(&dir), tree(&expected));
    for (path, bytes) in &theirs {
        match ours.get(path) {
            None => problems.push(format!("{name}: {} missing", path.display())),
            Some(b) if b != bytes => problems.push(format!(
                "{name}: {}: {}",
                path.display(),
                first_difference(&String::from_utf8_lossy(b), &String::from_utf8_lossy(bytes))
            )),
            Some(_) => {}
        }
    }
    for path in ours.keys().filter(|p| !theirs.contains_key(*p)) {
        problems.push(format!("{name}: {} not in nasher's tree", path.display()));
    }
    problems
}

#[test]
fn projects_save_what_nasher_unpacks() {
    let root = corpus!();
    let nasher = oracle_tool!("nasher");
    let _ = oracle_tool!("nwn_gff");
    let scratch = scratch_dir("nasher-projects");
    let modules: Vec<_> = bundled_modules(&root)
        .into_iter()
        .filter(|m| m.extension().is_some_and(|e| e == "mod"))
        .collect();
    let problems: Vec<String> =
        modules.par_iter().flat_map(|m| edit_project(&nasher, m, &scratch)).collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
