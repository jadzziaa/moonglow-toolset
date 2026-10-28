//! The game's programs as Moonglow draws them (its shaders switch on, the
//! default), written in HLSL as wgpu's Direct3D 12 backend writes them and,
//! where `MOONGLOW_D3DCOMPILER` names Microsoft's `d3dcompiler_47.dll`,
//! compiled by FXC under Wine (`tools/dx12/fxc.sh`), as Direct3D 12 on a
//! Windows machine compiles them: Heed's Radeon drew nothing of an area
//! through Direct3D 12, where nothing of the game's programs had been seen
//! drawn. The pipelines are those the shipped modules' areas make (each
//! drawn once, from above: tiles, placeables, creatures, doors), as the
//! renderer gives them to wgpu (`Renderer::capture_game_modules`), with
//! their layouts. In `target/test-output/hlsl/game/`.

use std::collections::BTreeMap;

use mg_area::{AreaModel, AreaScene, View, overview};
use mg_core::ResType;
use mg_gff::Gff;
use mg_render::{Gpu, Renderer};
use mg_resman::ResKey;
use mg_testkit::{corpus, hlsl};
use naga::ShaderStage;

use crate::area_common::each_module;

#[test]
fn the_game_s_programs_are_written_in_hlsl_as_direct3d_12_writes_them() {
    let root = corpus!();
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU adapter");
        return;
    };
    // (The area view's: four samples a pixel; the programs waited for.)
    let mut r = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm, 4);
    r.game_shaders = true;
    let captured = r.capture_game_modules();
    let mut areas = 0;
    each_module(&root, |opened| {
        let game = &opened.game;
        r.clear_textures();
        for area in opened.module.areas().unwrap() {
            let gff = |t: ResType| {
                let data = game.resman.get(&ResKey::new(area, t)).ok()?;
                Gff::read(&data).ok()
            };
            let (Some(are), Some(git)) = (gff(ResType::ARE), gff(ResType::GIT)) else {
                continue;
            };
            let tileset = are.root.resref("Tileset").and_then(|t| mg_area::tileset(game, t).ok());
            let model = AreaModel::read(game, &are.root, &git.root, tileset.as_ref());
            let loaded = AreaScene::new(&gpu, game, &model);
            let scene = loaded.scene(&model, &View::of(&model));
            r.render_image(&gpu, &game.resman, &scene, &overview(&model), 64, 48);
            areas += 1;
        }
    });
    let pipelines = captured.lock().unwrap().clone();
    let failures_drawing = r.game_shader_failures().to_vec();
    eprintln!(
        "{areas} areas drawn: {} pipelines of the game's programs made, {} could not be",
        pipelines.len(),
        failures_drawing.len()
    );
    assert!(pipelines.len() >= 10, "{} pipelines", pipelines.len());
    let dir = mg_testkit::output_dir("hlsl/game");
    // Each pipeline's two stages; a program written alike for two
    // pipelines (the same pair in two passes, say) is kept once.
    let mut written: BTreeMap<String, (std::path::PathBuf, String, String)> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut by_text: BTreeMap<String, String> = BTreeMap::new();
    for p in &pipelines {
        let options = hlsl::options(&p.groups);
        let name = format!(
            "{}.{}{}",
            p.label.replace('/', "+"),
            p.pass.to_lowercase(),
            if p.skinned { ".skinned" } else { "" }
        );
        let stages = [
            (ShaderStage::Vertex, &p.vs, &p.vs_entry, Some((&p.fs, p.fs_entry.as_str())), "vs"),
            (ShaderStage::Fragment, &p.fs, &p.fs_entry, None, "fs"),
        ];
        for (stage, module, entry, fragment, suffix) in stages {
            let label = format!("{name}.{suffix}");
            match hlsl::write(module, stage, entry, fragment, &options) {
                Ok((text, ep)) => {
                    if by_text.contains_key(&text) {
                        continue;
                    }
                    by_text.insert(text.clone(), label.clone());
                    let file = dir.join(format!("{label}.hlsl"));
                    std::fs::write(&file, &text).unwrap();
                    written.insert(label, (file, ep, hlsl::profile(stage).to_string()));
                }
                Err(e) => failures.push(format!("{label}: {e}")),
            }
        }
    }
    eprintln!("{} stages written in HLSL, in {}", written.len(), dir.display());
    assert!(failures.is_empty(), "not written in HLSL: {failures:#?}");
    let Some(dll) = hlsl::d3dcompiler() else {
        eprintln!("not compiled: MOONGLOW_D3DCOMPILER names no d3dcompiler_47.dll");
        return;
    };
    let shaders: Vec<_> = written.values().cloned().collect();
    let compiled = hlsl::fxc(&dll, &dir, &shaders).unwrap();
    let mut ms: Vec<u64> = compiled.iter().map(|c| c.ms).collect();
    ms.sort_unstable();
    for c in &compiled {
        let file = c.file.file_name().unwrap().to_string_lossy();
        eprintln!("{} {:>6} ms {file}", if c.ok { "OK  " } else { "FAIL" }, c.ms);
    }
    eprintln!(
        "FXC: {} of {} compiled; {} ms in all, the median {} ms, the longest {} ms",
        compiled.iter().filter(|c| c.ok).count(),
        compiled.len(),
        ms.iter().sum::<u64>(),
        ms[ms.len() / 2],
        ms.last().copied().unwrap_or(0)
    );
    // What FXC can't compile, and why (Direct3D 12 draws those meshes with
    // Moonglow's own shader: the pipeline's internal error is caught where
    // it is made, `game::checked`, and logged).
    let mut unknown = Vec::new();
    for c in compiled.iter().filter(|c| !c.ok) {
        let file = c.file.file_name().unwrap().to_string_lossy().into_owned();
        let fs = file.split('+').nth(1).and_then(|f| f.split('.').next()).unwrap_or("");
        let known = KNOWN_FXC_FAILURES.iter().find(|(name, _)| *name == fs);
        match known {
            Some((_, error)) if c.message.contains(error) => {
                eprintln!("known: {file} ({error}, docs/deferred.md)");
            }
            _ => unknown.push(format!("{file}: {}", c.message)),
        }
    }
    assert!(unknown.is_empty(), "FXC failed: {unknown:#?}");
}

/// The game's fragment programs FXC can't compile, and its error: the
/// normal-mapped lit programs' height-map loop (`inc_lighting`'s parallax:
/// a texture read with derivatives in a loop of a varying count, which
/// FXC must unroll and can't, X3511 after its warning X3570). Seen with
/// Microsoft's d3dcompiler_47.dll 10.0.15063.674, 2026-10-28; DXC (which
/// Moonglow doesn't ship) takes such a loop.
const KNOWN_FXC_FAILURES: &[(&str, &str)] = &[("fslit_nm", "X3511"), ("fslit_sm_nm", "X3511")];

/// The lit check (`mg_render::probe::probe_lit`, run as the game's data is
/// read) with the game's own `vslit`/`fslit`: both parts draw right, and
/// what it costs, with no shader cache and with the programs cached (as
/// from the second start on). It runs apart from the window's thread.
#[test]
fn the_lit_check_draws_with_the_game_s_programs() {
    use std::io::Write;
    let root = corpus!();
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("skipped: no GPU adapter");
        return;
    };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let rm = mg_resman::ResMan::for_game(&install).unwrap();
    let cache = mg_testkit::scratch_dir("lit-check-cache");
    let cold = mg_render::probe::probe_lit(&gpu, &rm, Some(cache.clone()));
    let cached = mg_render::probe::probe_lit(&gpu, &rm, Some(cache));
    let _ = writeln!(std::io::stdout(), "lit check, no cache: {cold}");
    let _ = writeln!(std::io::stdout(), "lit check, cached: {cached}");
    for lit in [&cold, &cached] {
        assert_eq!((&lit.own, &lit.game), (&Ok(()), &Some(Ok(()))), "{lit}");
    }
}
