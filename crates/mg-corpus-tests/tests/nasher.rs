//! nasher source trees: every shipped module unpacked by nasher itself must
//! give, file for file, the text Moonglow writes for the same resources, and
//! Moonglow must read each file back to a resource it writes as the same
//! text (so an untouched tree saves without changes).

use std::path::Path;
use std::process::Command;

use mg_erf::Erf;
use mg_gff::Gff;
use mg_module::nasher::Conversion;
use mg_resman::ResKey;
use mg_testkit::{bundled_modules, corpus, nwn_tool, oracle_tool, scratch_dir};
use rayon::prelude::*;

/// Unpacks `module` into the project at `dir` with nasher (its default rules:
/// everything into `src`).
fn nasher_unpack(nasher: &Path, module: &Path, dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("nasher.cfg"),
        "[package]\n  [package.sources]\n  include = \"src/**/*.{nss,json}\"\n\n  \
         [package.rules]\n  \"*\" = \"src\"\n\n[target]\nname = \"default\"\nfile = \"m.mod\"\n",
    )
    .unwrap();
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
