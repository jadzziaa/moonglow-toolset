//! Prints an area's corner lattice (terrain and height of each corner,
//! north at the top) and the crossers on cell edges, from the area's tiles:
//! `cargo run -p mg-corpus-tests --example print_lattice MODULE AREA`.
//! A corner prints as its terrain's first two letters and its height
//! (`Gr0`), an edge's crosser as its first letter between them; cells whose
//! tile disagrees with a neighbour are listed after.
use mg_core::{ResRef, ResType};
use mg_gff::Gff;
use mg_module::Module;
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_set::Tileset;
use mg_tiles::{Lattice, Placement, TileIndex};

fn main() {
    let mut args = std::env::args().skip(1);
    let module = args.next().expect("module");
    let area = args.next().expect("area resref");
    let root = mg_testkit::nwn_root().expect("game");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let m = Module::open(std::path::Path::new(&module)).unwrap();
    let key = ResKey::new(ResRef::from_str(&area).unwrap(), ResType::ARE);
    let are: Gff = m.gff(&key).unwrap().expect("no such area");
    let tileset = are.root.resref("Tileset").unwrap();
    let data = game.resman.get(&ResKey::new(tileset, ResType::SET)).unwrap();
    let set = Tileset::parse(&data, mg_core::Codepage::WINDOWS_1252).unwrap();
    let index = TileIndex::new(&set);
    let (w, h) =
        (are.root.integer("Width").unwrap() as u32, are.root.integer("Height").unwrap() as u32);
    let tiles: Vec<Placement> = are
        .root
        .list("Tile_List")
        .unwrap()
        .iter()
        .map(|t| Placement {
            tile: t.integer("Tile_ID").unwrap_or(0) as u32,
            orientation: t.integer("Tile_Orientation").unwrap_or(0) as u8,
            height: t.integer("Tile_Height").unwrap_or(0) as i32,
        })
        .collect();
    let (lattice, bad) = Lattice::from_tiles(&index, w, h, &tiles).expect("unknown tile");
    print!("{}", render(&index, &lattice));
    if !bad.is_empty() {
        println!("mismatched cells: {bad:?}");
    }
    for y in (0..h).rev() {
        let row: Vec<String> = (0..w)
            .map(|x| {
                let p = tiles[(y * w + x) as usize];
                format!("{:>3}/{}/{}", p.tile, p.orientation, p.height)
            })
            .collect();
        println!("{}", row.join(" "));
    }
}

/// Two-letter codes for the terrains: the first letter and the first later
/// one that no earlier terrain's code has.
fn codes(index: &TileIndex) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in index.terrains() {
        let name = index.terrain_name(t);
        let chars: Vec<char> = name.chars().collect();
        let first = chars.first().copied().unwrap_or('?');
        let code = chars[1..]
            .iter()
            .map(|c| format!("{first}{c}"))
            .find(|c| !out.contains(c))
            .unwrap_or_else(|| format!("{first}{}", t.0));
        out.push(code);
    }
    out
}

fn render(index: &TileIndex, l: &Lattice) -> String {
    let codes = codes(index);
    let corner = |x, y| {
        let c = l.corner(x, y);
        format!("{:<2}{}", codes[usize::from(c.terrain.0)], c.height)
    };
    let mut out = String::new();
    for y in (0..=l.height()).rev() {
        // Corners and the edges along x between them.
        for x in 0..=l.width() {
            out += &corner(x, y);
            if x < l.width() {
                let cell = l.cell(x, y.min(l.height() - 1));
                let edge = if y == l.height() {
                    cell.edges[mg_tiles::NORTH]
                } else {
                    cell.edges[mg_tiles::SOUTH]
                };
                out += &format!(
                    " {} ",
                    edge.map_or('-', |c| index.crosser_name(c).chars().next().unwrap())
                );
            }
        }
        out += "\n";
        if y > 0 {
            // The edges along y of the cells below.
            for x in 0..=l.width() {
                let cell = l.cell(x.min(l.width() - 1), y - 1);
                let edge = if x == l.width() {
                    cell.edges[mg_tiles::EAST]
                } else {
                    cell.edges[mg_tiles::WEST]
                };
                out += &format!(
                    " {}    ",
                    edge.map_or('|', |c| index.crosser_name(c).chars().next().unwrap())
                );
            }
            out += "\n";
        }
    }
    out
}
