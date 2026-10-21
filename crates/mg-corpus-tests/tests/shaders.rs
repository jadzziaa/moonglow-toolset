//! The game's own shaders compiled as Moonglow will draw with them
//! (`docs/shaders-proposal.md`, M0): every stock program, and every program
//! in the installed haks, to a validated naga module. Each failure is named
//! with its file, line and glslang's message.
//!
//! A folder's haks that hold shaders are layered together over the game, as
//! a module that lists them would have them: The Domains of Dread's
//! `tdod_shaders` replaces the stock `inc_lighting`, which its other haks'
//! shaders include. Where a hak replaces a stock file, the stock programs
//! are compiled again with it. Haks beyond the user's `hak/` and the
//! game's: `MOONGLOW_HAKS` (the development machine's baseline, 2026-10-21:
//! 65 stock programs, and 37 in `~/Projects/tdod-module/build/hak`).

use std::collections::BTreeSet;
use std::path::Path;

use mg_core::ResType;
use mg_resman::{ErfContainer, GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_shader::{Preamble, Stage, compile, program_stage};
use mg_testkit::{at_least, builder_hak_dirs, corpus, files_with_ext, sweep};

/// Every 3rd program on an everyday run.
const ONE_IN: usize = 3;

/// The programs among `keys` (`.shd` that define `main`), by name.
fn programs(rm: &ResMan, keys: impl IntoIterator<Item = ResKey>) -> Vec<(String, Stage)> {
    let mut out: Vec<(String, Stage)> = keys
        .into_iter()
        .filter(|k| k.restype == ResType::SHD)
        .filter_map(|k| {
            let name = k.resref.to_lowercase().to_string();
            let data = rm.get(&k).ok()?;
            Some((name.clone(), program_stage(&name, &data)?))
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Compiles each program; the failures, one line each.
fn compile_all(rm: &ResMan, list: &[(String, Stage)]) -> Vec<String> {
    let pre = Preamble::default();
    list.iter()
        .filter_map(|(name, stage)| {
            compile(name, *stage, &pre, rm).err().map(|e| {
                let d: Vec<String> = e.diagnostics().iter().map(ToString::to_string).collect();
                format!("{name} ({stage}): {}", d.join("; "))
            })
        })
        .collect()
}

/// The game alone: no user directory (its `override` would win).
fn game(root: &Path) -> ResMan {
    ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap()
}

/// The stock programs: those of the game's keys.
fn stock(rm: &ResMan) -> Vec<(String, Stage)> {
    let keys = rm
        .layers()
        .iter()
        .filter(|l| l.class == LayerClass::Key)
        .flat_map(|l| l.container.keys())
        .collect::<Vec<_>>();
    programs(rm, keys)
}

#[test]
fn every_stock_program_compiles() {
    let root = corpus!();
    let rm = game(&root);
    let all = stock(&rm);
    assert!(all.len() >= 60, "{} stock programs", all.len());
    let list = sweep(all, ONE_IN);
    let failed = compile_all(&rm, &list);
    assert!(failed.is_empty(), "{} of {} failed:\n{}", failed.len(), list.len(), failed.join("\n"));
    assert!(list.len() >= at_least(65, ONE_IN), "{} compiled", list.len());
    eprintln!("{} stock programs compiled", list.len());
}

#[test]
fn every_program_in_the_installed_haks_compiles() {
    let root = corpus!();
    let user = GameInstall::detect().and_then(|g| g.user_dir);
    let gi = GameInstall::new(&root, user, "en");
    let mut dirs = gi.hak_dirs();
    dirs.extend(builder_hak_dirs());
    let (mut compiled, mut failed) = (0, Vec::new());
    for dir in dirs {
        let mut rm = game(&root);
        let stock_names: BTreeSet<ResKey> = rm
            .entries()
            .into_iter()
            .map(|(k, _)| k)
            .filter(|k| k.restype == ResType::SHD)
            .collect();
        let mut mine = Vec::new();
        for path in files_with_ext(&dir, "hak") {
            let Ok(c) = ErfContainer::open(&path) else { continue };
            let shd: Vec<ResKey> =
                mg_resman::Container::keys(&c).filter(|k| k.restype == ResType::SHD).collect();
            if shd.is_empty() {
                continue;
            }
            let name = path.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
            rm.add(priority::HAK_USER, format!("hak:{name}"), LayerClass::Erf, c);
            mine.extend(shd);
        }
        if mine.is_empty() {
            continue;
        }
        let mut list = programs(&rm, mine.iter().copied());
        // Stock files the haks replace: the stock programs again, with them.
        let replaced: Vec<&ResKey> = mine.iter().filter(|k| stock_names.contains(k)).collect();
        if !replaced.is_empty() {
            eprintln!("{}: {} replace stock shaders", dir.display(), replaced.len());
            list.extend(stock(&rm));
            list.sort();
            list.dedup();
        }
        let list = sweep(list, ONE_IN);
        let f = compile_all(&rm, &list);
        eprintln!("{}: {} programs, {} failed", dir.display(), list.len(), f.len());
        compiled += list.len() - f.len();
        failed.extend(f.into_iter().map(|f| format!("{}: {f}", dir.display())));
    }
    assert!(failed.is_empty(), "{} failed:\n{}", failed.len(), failed.join("\n"));
    eprintln!("{compiled} programs in the haks compiled");
}
