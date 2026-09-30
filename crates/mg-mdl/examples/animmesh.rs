//! Prints each animation's vertex-animated nodes: `cargo run -p mg-mdl --example animmesh -- a.mdl`.
fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).unwrap();
        let m = mg_mdl::Model::read(&data).unwrap();
        for a in &m.animations {
            for n in &a.nodes {
                let Some(s) = &n.anim_mesh else { continue };
                let first = |sets: &Vec<Vec<[f32; 2]>>| -> Vec<[f32; 2]> {
                    sets.iter().filter_map(|v| v.first().copied()).collect()
                };
                println!(
                    "{} {} length {} {}: period {} vertex sets {} x {} uv sets {} x {} uv0 {:?}",
                    m.name,
                    a.name,
                    a.length,
                    n.name,
                    s.sample_period,
                    s.vertex_sets.len(),
                    s.vertex_sets.first().map_or(0, Vec::len),
                    s.uv_sets.len(),
                    s.uv_sets.first().map_or(0, Vec::len),
                    first(&s.uv_sets),
                );
            }
        }
    }
}
