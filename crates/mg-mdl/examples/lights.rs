//! Prints a model's lights: `cargo run -p mg-mdl --example lights -- a.mdl`.
fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).unwrap();
        let m = mg_mdl::Model::read(&data).unwrap();
        for n in &m.nodes {
            if let mg_mdl::NodeKind::Light(l) = &n.kind {
                let c: Vec<String> =
                    n.controllers.iter().map(|c| format!("{}={:?}", c.name, c.row(0))).collect();
                println!("{} {} pos {:?} {:?} {}", m.name, n.name, n.position, l, c.join(" "));
            }
        }
    }
}
