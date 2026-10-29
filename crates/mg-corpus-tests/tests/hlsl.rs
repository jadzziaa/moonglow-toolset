//! The game's programs as Moonglow draws them (its shaders switch on, the
//! default), written in HLSL as wgpu's Direct3D 12 backend writes them and
//! compiled as Direct3D 12 on a Windows machine compiles them: by DXC (the
//! `dxcompiler.dll` Moonglow ships), where `MOONGLOW_DXC` names a DXC
//! release's `bin/x64`, at shader models 6.0 and on
//! (`tools/dx12/dxc.sh`); and by FXC (where there is no `dxcompiler.dll`),
//! where `MOONGLOW_D3DCOMPILER` names Microsoft's `d3dcompiler_47.dll`, at
//! 5.1 (`tools/dx12/fxc.sh`); both under Wine. Heed's Radeon drew nothing
//! of an area through Direct3D 12, where nothing of the game's programs
//! had been seen drawn. The pipelines are those the shipped modules' areas
//! make (each drawn once, from above: tiles, placeables, creatures,
//! doors), as the renderer gives them to wgpu
//! (`Renderer::capture_game_modules`), with their layouts. In
//! `target/test-output/hlsl/game/` (FXC's) and its `dxc-6_0/` and so on.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    let Some(gpu) = mg_testkit::gpu::open(Gpu::headless) else { return };
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
    let root_dir = mg_testkit::output_dir("hlsl/game");
    // Each pipeline's two stages at a shader model, in `dir`; a program
    // written alike for two pipelines (the same pair in two passes, say)
    // is kept once.
    let write_all = |model: naga::back::hlsl::ShaderModel, dir: &Path| {
        let mut written: BTreeMap<String, (PathBuf, String, String)> = BTreeMap::new();
        let mut failures = Vec::new();
        let mut by_text: BTreeMap<String, String> = BTreeMap::new();
        for p in &pipelines {
            let options = hlsl::options_at(&p.groups, model);
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
                        written.insert(label, (file, ep, hlsl::profile_at(stage, model)));
                    }
                    Err(e) => failures.push(format!("{label}: {e}")),
                }
            }
        }
        eprintln!(
            "{} stages written in HLSL at shader model {}, in {}",
            written.len(),
            model.to_str(),
            dir.display()
        );
        assert!(failures.is_empty(), "not written in HLSL: {failures:#?}");
        written.into_values().collect::<Vec<_>>()
    };
    let report = |compiler: &str, compiled: &[hlsl::Compiled]| {
        let mut ms: Vec<u64> = compiled.iter().map(|c| c.ms).collect();
        ms.sort_unstable();
        for c in compiled {
            let file = c.file.file_name().unwrap().to_string_lossy();
            eprintln!("{} {:>6} ms {file}", if c.ok { "OK  " } else { "FAIL" }, c.ms);
        }
        eprintln!(
            "{compiler}: {} of {} compiled; {} ms in all, the median {} ms, the longest {} ms",
            compiled.iter().filter(|c| c.ok).count(),
            compiled.len(),
            ms.iter().sum::<u64>(),
            ms[ms.len() / 2],
            ms.last().copied().unwrap_or(0)
        );
    };
    // FXC, at 5.1 (where Moonglow finds no dxcompiler.dll beside it).
    let fxc_written = write_all(naga::back::hlsl::ShaderModel::V5_1, &root_dir);
    match hlsl::d3dcompiler() {
        None => eprintln!("FXC not run: MOONGLOW_D3DCOMPILER names no d3dcompiler_47.dll"),
        Some(dll) => {
            let compiled = hlsl::fxc(&dll, &root_dir, &fxc_written).unwrap();
            report("FXC", &compiled);
            // What FXC can't compile, and why (Direct3D 12 with FXC draws
            // those meshes with Moonglow's own shader: the pipeline's
            // internal error is caught where it is made, `game::checked`,
            // and logged).
            let mut unknown = Vec::new();
            for c in compiled.iter().filter(|c| !c.ok) {
                let file = c.file.file_name().unwrap().to_string_lossy().into_owned();
                let fs = file.split('+').nth(1).and_then(|f| f.split('.').next()).unwrap_or("");
                let known = KNOWN_FXC_FAILURES.iter().find(|(name, _)| *name == fs);
                match known {
                    Some((_, error)) if c.message.contains(error) => {
                        eprintln!("known to FXC: {file} ({error}; DXC compiles it)");
                    }
                    _ => unknown.push(format!("{file}: {}", c.message)),
                }
            }
            assert!(unknown.is_empty(), "FXC failed: {unknown:#?}");
        }
    }
    // DXC, at each shader model it may be given: everything compiles and
    // is signed, with dxcompiler.dll alone.
    let Some(dxc) = hlsl::dxc_dir() else {
        eprintln!("DXC not run: MOONGLOW_DXC names no folder of dxc.exe and dxcompiler.dll");
        return;
    };
    for &model in hlsl::DXC_SHADER_MODELS {
        let dir = root_dir.join(format!("dxc-{}", model.to_str()));
        std::fs::create_dir_all(&dir).unwrap();
        let written = write_all(model, &dir);
        let compiled = hlsl::dxc(&dxc, &dir, &written).unwrap();
        report(&format!("DXC at {}", model.to_str()), &compiled);
        let failed: Vec<String> = compiled
            .iter()
            .filter(|c| !c.ok)
            .map(|c| format!("{}: {}", c.file.display(), c.message))
            .collect();
        assert!(failed.is_empty(), "DXC failed at {}: {failed:#?}", model.to_str());
    }
}

/// The game's fragment programs FXC can't compile, and its error: the
/// normal-mapped lit programs' height-map loop (`inc_lighting`'s parallax:
/// a texture read with derivatives in a loop of a varying count, which
/// FXC must unroll and can't, X3511 after its warning X3570). Seen with
/// Microsoft's d3dcompiler_47.dll 10.0.15063.674, 2026-10-28. DXC, which
/// Moonglow ships beside its programs since the release after 2.13.0
/// (`mg_render::dx12`), compiles them: none is known to it. FXC is still
/// what Direct3D 12 takes where `dxcompiler.dll` is missing.
const KNOWN_FXC_FAILURES: &[(&str, &str)] = &[("fslit_nm", "X3511"), ("fslit_sm_nm", "X3511")];

/// The lit check (`mg_render::probe::probe_lit`, run as the game's data is
/// read) with the game's own `vslit`/`fslit`: both parts draw right, and
/// what it costs, with no shader cache and with the programs cached (as
/// from the second start on). It runs apart from the window's thread.
/// What each drew: `target/test-output/lit-check/own.png` and `game.png`.
#[test]
fn the_lit_check_draws_with_the_game_s_programs() {
    use std::io::Write;
    let root = corpus!();
    let Some(gpu) = mg_testkit::gpu::open(Gpu::headless) else { return };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let rm = mg_resman::ResMan::for_game(&install).unwrap();
    let cache = mg_testkit::scratch_dir("lit-check-cache");
    let cold = mg_render::probe::probe_lit(&gpu, &rm, Some(cache.clone()));
    let cached = mg_render::probe::probe_lit(&gpu, &rm, Some(cache));
    let _ = writeln!(std::io::stdout(), "lit check, no cache: {cold}");
    let _ = writeln!(std::io::stdout(), "lit check, cached: {cached}");
    let out = mg_testkit::output_dir("lit-check");
    for (name, picture) in [("own", &cold.own_picture), ("game", &cold.game_picture)] {
        save_png(picture.as_ref().expect("drawn"), &out.join(format!("{name}.png")));
    }
    for lit in [&cold, &cached] {
        assert_eq!((&lit.own, &lit.game), (&Ok(()), &Some(Ok(()))), "{lit}");
    }
}

/// A picture as a PNG.
fn save_png(img: &mg_image::Rgba, path: &Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&img.data).unwrap();
}

/// The game's install with one include replaced in the load order, as a
/// builder's copy in `override/` replaces it: `inc_lighting` computing its
/// specular light as if every surface were seen edge on (`fNdotV` zero),
/// which draws the lit check's red mesh a washed-out pink, as Heed's
/// Vulkan drew it (2.13.1: [212, 133, 133]). `own`: whether the game's own
/// copy is what [`mg_render::Assets::game_shader`] gives (the load order's
/// otherwise, as the check read the programs before).
struct Overridden<'a> {
    game: &'a mg_resman::ResMan,
    own: bool,
}

const EDGE_ON: (&str, &str) = ("fNdotV = dot(vNormalThis, -vViewToSurface_n);", "fNdotV = 0.0;");

impl mg_render::Assets for Overridden<'_> {
    fn texture(&self, name: &str) -> Option<mg_render::LoadedTexture> {
        mg_render::Assets::texture(self.game, name)
    }

    fn shader(&self, name: &str) -> Option<String> {
        let text = mg_render::Assets::shader(self.game, name)?;
        Some(if name == "inc_lighting" {
            assert!(text.contains(EDGE_ON.0), "the game's inc_lighting changed");
            text.replace(EDGE_ON.0, EDGE_ON.1)
        } else {
            text
        })
    }

    fn game_shader(&self, name: &str) -> Option<String> {
        if self.own { mg_render::Assets::game_shader(self.game, name) } else { self.shader(name) }
    }

    fn shader_origin(&self, name: &str) -> Option<String> {
        (name == "inc_lighting").then(|| "override".to_string())
    }
}

/// A builder's include in the load order (the game's `inc_lighting`, its
/// specular light computed as if seen edge on) draws the lit check's mesh
/// pink, as Heed's machine drew it; the check reads the game's own copies,
/// so it draws red all the same and says which file the load order
/// replaces. In `target/test-output/lit-check/` (`builder-include.png`,
/// the pink; `game-own.png`, the check's).
#[test]
fn a_builder_s_include_leaves_the_lit_check_to_the_game_s_own() {
    use std::io::Write;
    let root = corpus!();
    let Some(gpu) = mg_testkit::gpu::open(Gpu::headless) else { return };
    let install = mg_resman::GameInstall::new(&root, None, "en");
    let rm = mg_resman::ResMan::for_game(&install).unwrap();
    let out = mg_testkit::output_dir("lit-check");
    // (The load order's copies drawn, as the check read them before.)
    let pink = mg_render::probe::probe_lit(&gpu, &Overridden { game: &rm, own: false }, None);
    let _ = writeln!(std::io::stdout(), "the load order's: {pink}");
    save_png(pink.game_picture.as_ref().unwrap(), &out.join("builder-include.png"));
    assert_eq!(pink.own, Ok(()), "{pink}");
    assert!(pink.game_fails(), "{pink}");
    let [r, g, b] = pink.game_color.unwrap();
    assert!(r > 150 && g > 90 && (g as i32 - b as i32).abs() <= 2, "pink: {pink}");
    assert!(!mg_render::probe::Lit::drew_nothing(pink.game_color));
    // The game's own: red, and the replaced file said.
    let lit = mg_render::probe::probe_lit(&gpu, &Overridden { game: &rm, own: true }, None);
    let _ = writeln!(std::io::stdout(), "the game's own: {lit}");
    save_png(lit.game_picture.as_ref().unwrap(), &out.join("game-own.png"));
    assert_eq!((&lit.own, &lit.game), (&Ok(()), &Some(Ok(()))), "{lit}");
    assert_eq!(lit.overridden, ["inc_lighting.shd (override)"], "{lit}");
    assert!(lit.to_string().contains("the load order has inc_lighting.shd (override)"), "{lit}");
    assert_eq!(lit.game_color, lit.own_color, "{lit}");
}
