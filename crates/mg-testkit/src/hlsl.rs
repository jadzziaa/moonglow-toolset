//! Shaders written in HLSL as wgpu's Direct3D 12 backend writes them, and
//! compiled by FXC as it compiles them, on a machine without Direct3D 12
//! (`feature = "hlsl"`).
//!
//! wgpu 30 (`wgpu-hal`'s `dx12/device.rs`) writes each pipeline stage with
//! naga's HLSL backend at shader model 5.1 where it compiles with FXC (its
//! `Dx12Compiler::Auto` takes FXC where no `dxcompiler.dll` is found, and
//! Moonglow ships none): [`options`] repeats its options, the binding map
//! of the pipeline's layout included (CBV, SRV and UAV registers in order
//! across the groups, in space 0; each group's samplers through the
//! sampler heap and an index buffer of its own; the special constants of
//! the first vertex and instance after them). [`write`] writes a stage as
//! it does, the vertex stage with the fragment stage's inputs (naga drops
//! the outputs the fragment program doesn't read). [`fxc`] compiles what
//! was written with Microsoft's `d3dcompiler_47.dll` under Wine
//! (`tools/dx12/fxc.sh`), where `MOONGLOW_D3DCOMPILER` names one.

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use naga::back::hlsl;
use wgpu_types as wgt;

/// The options wgpu's Direct3D 12 backend writes a pipeline's stages
/// with, for a pipeline of bind groups laid out as `groups` (each group's
/// entries, in the pipeline's order), at shader model 5.1 (FXC's).
pub fn options(groups: &[Vec<wgt::BindGroupLayoutEntry>]) -> hlsl::Options {
    let mut binding_map = hlsl::BindingMap::default();
    let mut sampler_buffer_binding_map = hlsl::SamplerIndexBufferBindingMap::default();
    let mut dynamic_storage_buffer_offsets_targets = std::collections::BTreeMap::new();
    let (mut cbv, mut srv, mut uav) =
        (hlsl::BindTarget::default(), hlsl::BindTarget::default(), hlsl::BindTarget::default());
    for (index, entries) in groups.iter().enumerate() {
        let group = index as u32;
        // (As wgpu-core keeps a group's entries: by binding.)
        let mut entries = entries.clone();
        entries.sort_by_key(|e| e.binding);
        let mut dynamic_storage = 0;
        for e in &entries {
            let count = e.count.map_or(1, NonZeroU32::get);
            let (target, dynamic) = match e.ty {
                wgt::BindingType::Buffer {
                    ty: wgt::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    ..
                } => continue,
                wgt::BindingType::Sampler(_) => continue,
                wgt::BindingType::Buffer { ty: wgt::BufferBindingType::Uniform, .. } => {
                    (&mut cbv, false)
                }
                wgt::BindingType::Buffer {
                    ty: wgt::BufferBindingType::Storage { read_only },
                    has_dynamic_offset,
                    ..
                } => (if read_only { &mut srv } else { &mut uav }, has_dynamic_offset),
                wgt::BindingType::StorageTexture {
                    access: wgt::StorageTextureAccess::ReadOnly,
                    ..
                }
                | wgt::BindingType::Texture { .. }
                | wgt::BindingType::AccelerationStructure { .. }
                | wgt::BindingType::ExternalTexture => (&mut srv, false),
                wgt::BindingType::StorageTexture { .. } => (&mut uav, false),
            };
            let offsets = dynamic.then(|| {
                dynamic_storage += 1;
                dynamic_storage - 1
            });
            binding_map.insert(
                naga::ResourceBinding { group, binding: e.binding },
                hlsl::BindTarget {
                    binding_array_size: e.count.map(NonZeroU32::get),
                    dynamic_storage_buffer_offsets_index: offsets,
                    ..*target
                },
            );
            target.register += count;
        }
        let mut samplers = 0;
        for e in entries.iter().filter(|e| matches!(e.ty, wgt::BindingType::Sampler(_))) {
            binding_map.insert(
                naga::ResourceBinding { group, binding: e.binding },
                hlsl::BindTarget { space: 255, register: samplers, ..Default::default() },
            );
            samplers += 1;
        }
        if samplers > 0 {
            sampler_buffer_binding_map.insert(hlsl::SamplerIndexBufferKey { group }, srv);
            srv.register += 1;
        }
        for e in &entries {
            if let wgt::BindingType::Buffer {
                ty: wgt::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                ..
            } = e.ty
            {
                binding_map.insert(
                    naga::ResourceBinding { group, binding: e.binding },
                    hlsl::BindTarget {
                        binding_array_size: e.count.map(NonZeroU32::get),
                        restrict_indexing: true,
                        ..cbv
                    },
                );
                cbv.register += e.count.map_or(1, NonZeroU32::get);
            }
        }
        if dynamic_storage > 0 {
            dynamic_storage_buffer_offsets_targets.insert(
                group,
                hlsl::OffsetsBindTarget {
                    space: cbv.space,
                    register: cbv.register,
                    size: dynamic_storage,
                },
            );
            cbv.register += 1;
        }
    }
    let heap = |register| hlsl::BindTarget { space: 0, register, ..Default::default() };
    hlsl::Options {
        shader_model: hlsl::ShaderModel::V5_1,
        binding_map,
        fake_missing_bindings: false,
        special_constants_binding: Some(cbv),
        immediates_target: None,
        dynamic_storage_buffer_offsets_targets,
        zero_initialize_workgroup_memory: true,
        restrict_indexing: true,
        sampler_heap_target: hlsl::SamplerHeapBindTargets {
            standard_samplers: heap(0),
            comparison_samplers: heap(2048),
        },
        sampler_buffer_binding_map,
        external_texture_binding_map: Default::default(),
        force_loop_bounding: true,
        task_dispatch_limits: None,
        mesh_shader_primitive_indices_clamp: true,
        ray_query_initialization_tracking: true,
    }
}

/// The bind groups a module's resources take, laid out as a pipeline of
/// them alone would be (for a module whose pipeline's layout isn't at
/// hand: Moonglow's own WGSL), each resource visible to every stage.
pub fn groups_of(module: &naga::Module) -> Vec<Vec<wgt::BindGroupLayoutEntry>> {
    let mut groups: Vec<Vec<wgt::BindGroupLayoutEntry>> = Vec::new();
    for (_, var) in module.global_variables.iter() {
        let Some(binding) = &var.binding else { continue };
        let ty = match (&var.space, &module.types[var.ty].inner) {
            (naga::AddressSpace::Uniform, _) => wgt::BindingType::Buffer {
                ty: wgt::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            (naga::AddressSpace::Storage { access }, _) => wgt::BindingType::Buffer {
                ty: wgt::BufferBindingType::Storage {
                    read_only: !access.contains(naga::StorageAccess::STORE),
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            (_, naga::TypeInner::Sampler { comparison }) => {
                wgt::BindingType::Sampler(if *comparison {
                    wgt::SamplerBindingType::Comparison
                } else {
                    wgt::SamplerBindingType::Filtering
                })
            }
            (
                _,
                naga::TypeInner::Image {
                    class: naga::ImageClass::Storage { format, access }, ..
                },
            ) => {
                let _ = format;
                wgt::BindingType::StorageTexture {
                    access: if access.contains(naga::StorageAccess::STORE) {
                        wgt::StorageTextureAccess::ReadWrite
                    } else {
                        wgt::StorageTextureAccess::ReadOnly
                    },
                    format: wgt::TextureFormat::Rgba8Unorm,
                    view_dimension: wgt::TextureViewDimension::D2,
                }
            }
            (_, naga::TypeInner::Image { .. }) => wgt::BindingType::Texture {
                sample_type: wgt::TextureSampleType::Float { filterable: true },
                view_dimension: wgt::TextureViewDimension::D2,
                multisampled: false,
            },
            _ => continue,
        };
        let g = binding.group as usize;
        if groups.len() <= g {
            groups.resize(g + 1, Vec::new());
        }
        groups[g].push(wgt::BindGroupLayoutEntry {
            binding: binding.binding,
            visibility: wgt::ShaderStages::all(),
            ty,
            count: None,
        });
    }
    groups
}

/// A module validated as wgpu validates what it is given.
pub fn validated(module: &naga::Module) -> Result<naga::valid::ModuleInfo, String> {
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(module)
    .map_err(|e| format!("{:?}", e.into_inner()))
}

/// A stage of a pipeline, written in HLSL: the source and the name of its
/// entry point. `stage` and `entry` name it in `module`; a vertex stage is
/// written with its pipeline's fragment stage (`fragment`: its module and
/// entry point), as wgpu writes it.
pub fn write(
    module: &naga::Module,
    stage: naga::ShaderStage,
    entry: &str,
    fragment: Option<(&naga::Module, &str)>,
    options: &hlsl::Options,
) -> Result<(String, String), String> {
    let info = validated(module)?;
    let fragment = match fragment {
        Some((m, e)) => Some(
            hlsl::FragmentEntryPoint::new(m, e)
                .ok_or_else(|| format!("no fragment entry point {e}"))?,
        ),
        None => None,
    };
    let pipeline = hlsl::PipelineOptions { entry_point: Some((stage, entry.to_string())) };
    let mut source = String::new();
    let mut writer = hlsl::Writer::new(&mut source, options, &pipeline);
    let mut reflection =
        writer.write(module, &info, fragment.as_ref()).map_err(|e| format!("HLSL: {e:?}"))?;
    let name = reflection
        .entry_point_names
        .pop()
        .ok_or("no entry point written")?
        .map_err(|e| e.to_string())?;
    Ok((source, name))
}

/// The profile FXC compiles a stage with.
pub fn profile(stage: naga::ShaderStage) -> &'static str {
    match stage {
        naga::ShaderStage::Vertex => "vs_5_1",
        naga::ShaderStage::Fragment => "ps_5_1",
        _ => "cs_5_1",
    }
}

/// Microsoft's `d3dcompiler_47.dll`, where `MOONGLOW_D3DCOMPILER` names it
/// (Wine's builtin one is vkd3d's compiler, not FXC).
pub fn d3dcompiler() -> Option<PathBuf> {
    std::env::var_os("MOONGLOW_D3DCOMPILER").map(PathBuf::from).filter(|p| p.is_file())
}

/// What FXC made of a shader: its file, whether it compiled, how long it
/// took (milliseconds) and what it said.
#[derive(Debug, Clone)]
pub struct Compiled {
    pub file: PathBuf,
    pub ok: bool,
    pub ms: u64,
    pub message: String,
}

/// Compiles each of `shaders` (a file, its entry point and profile) with
/// FXC through `tools/dx12/fxc.sh`, in `dir` (where its manifest is
/// written), with `dll`.
pub fn fxc(
    dll: &Path,
    dir: &Path,
    shaders: &[(PathBuf, String, String)],
) -> Result<Vec<Compiled>, String> {
    let windows = |p: &Path| format!("Z:{}", p.display()).replace('/', "\\");
    let manifest: String =
        shaders.iter().map(|(f, e, p)| format!("{}\t{e}\t{p}\n", windows(f))).collect();
    let list = dir.join("manifest.txt");
    std::fs::write(&list, manifest).map_err(|e| e.to_string())?;
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/dx12/fxc.sh");
    let out = std::process::Command::new(script)
        .arg(dll)
        .arg(&list)
        .env("MOONGLOW_FXC_DIR", dir.join("bin"))
        .output()
        .map_err(|e| format!("tools/dx12/fxc.sh: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let by_name: std::collections::HashMap<String, &PathBuf> =
        shaders.iter().map(|(f, _, _)| (windows(f), f)).collect();
    let mut done = Vec::new();
    for line in text.lines() {
        let (ok, rest) = if let Some(r) = line.strip_prefix("OK ") {
            (true, r)
        } else if let Some(r) = line.strip_prefix("FAIL ") {
            (false, r)
        } else {
            continue;
        };
        let (head, message) = rest.split_once(": ").unwrap_or((rest, ""));
        let (name, ms) = head.rsplit_once(' ').unwrap_or((head, "0"));
        let Some(file) = by_name.get(name) else { continue };
        done.push(Compiled {
            file: (*file).clone(),
            ok,
            ms: ms.parse().unwrap_or(0),
            message: message.trim().to_string(),
        });
    }
    if done.len() != shaders.len() {
        return Err(format!(
            "FXC gave {} answers for {} shaders: {}{}",
            done.len(),
            shaders.len(),
            String::from_utf8_lossy(&out.stderr),
            text
        ));
    }
    Ok(done)
}
