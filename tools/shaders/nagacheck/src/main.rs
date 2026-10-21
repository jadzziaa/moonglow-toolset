// The naga half of the shader spike (docs/shaders-proposal.md): parse each SPIR-V file
// (after spirv-opt --split-combined-image-sampler), renumber every resource binding,
// validate and write WGSL. UBO=1 prints the uniform block, DUMP=1 the globals.
// Outside the workspace: cargo run --release --manifest-path tools/shaders/nagacheck/Cargo.toml -- out/*.spv
use naga::valid::{Capabilities, ValidationFlags, Validator};
fn main() {
    let (mut ok, mut bad) = (0, 0);
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).unwrap();
        let opts = naga::front::spv::Options { adjust_coordinate_space: true, strict_capabilities: false, block_ctx_dump_prefix: None };
        let r = naga::front::spv::parse_u8_slice(&bytes, &opts).map_err(|e| format!("parse: {e}")).and_then(|mut m| {
            let mut n = 0u32;
            for (_, g) in m.global_variables.iter_mut() { if let Some(b) = g.binding.as_mut() { b.group = 0; b.binding = n; n += 1; } }
            if std::env::var("UBO").is_ok() { for (_, g) in m.global_variables.iter() { if g.space == naga::AddressSpace::Uniform { if let naga::TypeInner::Struct { members, span } = &m.types[g.ty].inner { eprintln!("  block span {} bytes, {} members: {}", span, members.len(), members.iter().map(|x| x.name.clone().unwrap_or_default()).collect::<Vec<_>>().join(" ")); } } } }
            if std::env::var("DUMP").is_ok() { for (_, g) in m.global_variables.iter() { eprintln!("  {:?} {:?} {:?}", g.name, g.space, g.binding); } }
            Validator::new(ValidationFlags::all(), Capabilities::default()).validate(&m).map(|i| (m, i)).map_err(|e| format!("validate: {:?}", e.as_inner()))
        }).and_then(|(m, i)| {
            naga::back::wgsl::write_string(&m, &i, naga::back::wgsl::WriterFlags::empty()).map_err(|e| format!("wgsl: {e}"))
        });
        let name = std::path::Path::new(&path).file_name().unwrap().to_string_lossy();
        match r { Ok(_) => { ok += 1; println!("OK   {name}") } Err(e) => { bad += 1; println!("FAIL {name}: {}", e.chars().take(600).collect::<String>()) } }
    }
    println!("\n{ok} accepted, {bad} rejected");
}
