//! Prints each mesh's render flags: `cargo run -p mg-mdl --example meshflags -- a.mdl`.
fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).unwrap();
        let m = mg_mdl::Model::read(&data).unwrap();
        for n in &m.nodes {
            if let Some(mesh) = n.mesh() {
                println!(
                    "{} {:<24} {:<10} render {} shadow {} tilefade {} hint {} faces {} tex {:?}",
                    m.name,
                    n.name,
                    n.kind.type_name(),
                    mesh.render,
                    mesh.shadow,
                    mesh.tilefade,
                    mesh.transparency_hint,
                    mesh.faces.len(),
                    mesh.textures[0]
                );
            }
        }
    }
}
