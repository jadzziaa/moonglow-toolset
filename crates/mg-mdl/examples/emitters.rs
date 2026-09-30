//! Prints the emitters of models (settings and rest controllers):
//! `cargo run -p mg-mdl --example emitters -- a.mdl b.mdl`.
fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).unwrap();
        let m = mg_mdl::Model::read(&data).unwrap();
        for n in &m.nodes {
            if let mg_mdl::NodeKind::Emitter(e) = &n.kind {
                println!("{} {}: {:?}", m.name, n.name, e);
                let c: Vec<String> =
                    n.controllers.iter().map(|c| format!("{}={:?}", c.name, c.row(0))).collect();
                println!("    {}", c.join(" "));
            }
        }
    }
}
