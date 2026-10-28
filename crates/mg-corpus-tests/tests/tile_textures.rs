//! A tile's textures, as the tileset editor, Tile Properties and a tile's
//! view list them (`mg_preview::replaceable::uses` and `texture_file`), over
//! a sweep of the tiles of every tileset the game ships and, with
//! `MOONGLOW_HAKS`, of the tilesets in a builder's haks (The Domains of
//! Dread's draw theirs with materials):
//!
//! - every mesh a tile draws with a texture has its diffuse texture among
//!   its uses, and each use names a node of the model;
//! - a texture said to be found is one the renderer finds
//!   (`mg_render::assets::Assets::texture`), and one said missing is one it
//!   doesn't (cube maps aside, which only the TXI's faces make);
//! - a found DDS or TGA has its size and format, from the layer the load
//!   order reads it from.

use std::collections::{BTreeSet, HashMap};

use mg_core::{Codepage, ResRef, ResType};
use mg_mdl::{Model, NodeKind};
use mg_preview::replaceable::{Role, TextureFile, texture_file, uses};
use mg_render::assets::Assets;
use mg_resman::{ErfContainer, GameInstall, LayerClass, ResKey, ResMan, priority};
use mg_rules::GameData;
use mg_set::Tileset;
use mg_testkit::{at_least, builder_hak_dirs, corpus, files_with_ext, sweep};

/// One tile model in how many the sweep takes on an everyday run.
const ONE_IN: usize = 9;

/// What a sweep over some tilesets' tiles found.
#[derive(Default)]
struct Tally {
    models: usize,
    uses: usize,
    textures: usize,
    missing: BTreeSet<String>,
    materials: usize,
    problems: Vec<String>,
}

/// Sweeps the tiles of `tilesets` in `game`: what [`Tally`] counts, each
/// check made of every tile's listing.
fn sweep_tiles(game: &GameData, tilesets: &[ResRef]) -> Tally {
    let rm = &game.resman;
    // The tiles' models, each once with its tileset's environment map.
    let mut models: Vec<(String, Option<String>)> = Vec::new();
    let mut seen = BTreeSet::new();
    for &set in tilesets {
        let Ok(data) = rm.get(&ResKey::new(set, ResType::SET)) else { continue };
        let Ok(t) = Tileset::parse(&data, Codepage::WINDOWS_1252) else { continue };
        for tile in &t.tiles {
            let model = tile.model.to_ascii_lowercase();
            if seen.insert(model.clone()) {
                models.push((model, t.general.env_map.clone()));
            }
        }
    }
    let mut tally = Tally::default();
    let mut files: HashMap<String, Option<TextureFile>> = HashMap::new();
    for (name, env) in sweep(models, ONE_IN) {
        let Ok(data) = rm.get_named(&name, ResType::MDL) else { continue };
        let Ok(model) = Model::read(&data) else { continue };
        tally.models += 1;
        let found = uses(game, &model, env.as_deref());
        tally.uses += found.len();
        let nodes: BTreeSet<&str> = model.nodes.iter().map(|n| n.name.as_str()).collect();
        for u in &found {
            if !u.node.contains(" › ") && !nodes.contains(u.node.as_str()) {
                tally.problems.push(format!("{name}: {} names no node of it ({})", u.node, u.name));
            }
            if u.material.is_some() && u.role == Role::Slot(0) {
                tally.materials += 1;
            }
        }
        // Every drawn mesh with a texture: its diffuse among the uses.
        for n in &model.nodes {
            let NodeKind::Mesh(mesh) = &n.kind else { continue };
            let named = mesh.textures[0]
                .as_deref()
                .map(str::trim)
                .filter(|b| !b.is_empty() && !b.eq_ignore_ascii_case("null"));
            if mesh.render
                && named.is_some()
                && !found.iter().any(|u| u.node == n.name && u.role == Role::Slot(0) && u.drawn)
            {
                tally.problems.push(format!("{name}: mesh {} has no diffuse listed", n.name));
            }
        }
        for u in found.iter().filter(|u| u.drawn) {
            if files.contains_key(&u.name) {
                continue;
            }
            let file = texture_file(game, &u.name);
            tally.textures += 1;
            match &file {
                None => _ = tally.missing.insert(u.name.clone()),
                Some(f) if f.cube => {}
                Some(f) => {
                    let layer = ResRef::cut(&u.name).map(|r| {
                        let key = ResKey::new(r, f.restype);
                        rm.copies(&key).into_iter().any(|i| rm.layers()[i].label == f.layer)
                    });
                    if layer != Some(true) {
                        tally.problems.push(format!("{}: its layer {} has none", u.name, f.layer));
                    }
                    if f.problem.is_none()
                        && f.restype != ResType::PLT
                        && (f.width == 0 || f.height == 0 || f.format.is_empty())
                    {
                        tally.problems.push(format!("{}: no size or format", u.name));
                    }
                }
            }
            // The renderer finds what is found (an MTR of the name aside,
            // which it follows: the listing lists the MTR's own).
            let mtr =
                ResRef::cut(&u.name).is_some_and(|r| rm.contains(&ResKey::new(r, ResType::MTR)));
            let cube = file.as_ref().is_some_and(|f| f.cube);
            // (Found and readable: what the renderer draws.)
            let listed = file.as_ref().is_some_and(|f| f.problem.is_none());
            if !mtr && !cube && listed != Assets::texture(rm, &u.name).is_some() {
                let said = match &file {
                    Some(f) => format!("found in {} ({:?})", f.layer, f.problem),
                    None => "missing".into(),
                };
                tally.problems.push(format!("{}: listed {said}, the renderer differs", u.name));
            }
            files.insert(u.name.clone(), file);
        }
    }
    tally
}

fn game_data(rm: ResMan) -> GameData {
    GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH))
}

#[test]
fn the_textures_of_the_game_s_tiles_are_listed_as_the_renderer_finds_them() {
    let root = corpus!();
    let install = GameInstall::new(&root, None, "en");
    let game = game_data(ResMan::for_game(&install).unwrap());
    let tilesets = game.resman.list(ResType::SET);
    let t = sweep_tiles(&game, &tilesets);
    eprintln!(
        "{} tilesets, {} tile models: {} uses, {} textures ({} drawn through a material), {} \
         missing: {:?}",
        tilesets.len(),
        t.models,
        t.uses,
        t.textures,
        t.materials,
        t.missing.len(),
        t.missing.iter().take(20).collect::<Vec<_>>()
    );
    assert!(t.problems.is_empty(), "{}:\n{}", t.problems.len(), t.problems.join("\n"));
    assert!(t.models >= at_least(12000, ONE_IN), "{} tile models", t.models);
    assert!(t.textures >= at_least(1800, ONE_IN), "{} textures", t.textures);
    // (The game's tiles name a few textures it ships none of.)
    assert!(t.missing.len() * 50 < t.textures, "{} of {} missing", t.missing.len(), t.textures);
}

/// The tilesets in a builder's haks (`MOONGLOW_HAKS`), each folder's haks
/// that hold a `.set` over the game's data.
#[test]
fn the_textures_of_a_builder_s_tiles_are_listed_as_the_renderer_finds_them() {
    let root = corpus!();
    let install = GameInstall::new(&root, None, "en");
    let dirs = builder_hak_dirs();
    if dirs.is_empty() {
        eprintln!("skipped: no MOONGLOW_HAKS");
        return;
    }
    for dir in dirs {
        let mut rm = ResMan::for_game(&install).unwrap();
        let mut sets = Vec::new();
        for path in files_with_ext(&dir, "hak") {
            let Ok(c) = ErfContainer::open(&path) else { continue };
            let mine: Vec<ResRef> = mg_resman::Container::keys(&c)
                .filter(|k| k.restype == ResType::SET)
                .map(|k| k.resref)
                .collect();
            // (A hak of a tileset's textures alone holds none: every hak
            // is added, for the tilesets to find theirs.)
            sets.extend(mine);
            let name = path.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
            rm.add(priority::HAK_USER, format!("hak:{name}"), LayerClass::Erf, c);
        }
        if sets.is_empty() {
            continue;
        }
        sets.sort();
        sets.dedup();
        let game = game_data(rm);
        let t = sweep_tiles(&game, &sets);
        eprintln!(
            "{}: {} tilesets, {} tile models: {} uses, {} textures ({} drawn through a \
             material), {} missing: {:?}",
            dir.display(),
            sets.len(),
            t.models,
            t.uses,
            t.textures,
            t.materials,
            t.missing.len(),
            t.missing.iter().take(20).collect::<Vec<_>>()
        );
        assert!(t.problems.is_empty(), "{}:\n{}", t.problems.len(), t.problems.join("\n"));
        assert!(t.models > 0, "{}: no tile models", dir.display());
    }
}
