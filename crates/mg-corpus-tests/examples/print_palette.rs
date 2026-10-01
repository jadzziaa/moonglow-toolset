//! Prints a tileset's palette as the area editor shows it:
//! `cargo run -p mg-corpus-tests --example print_palette TILESET...`.
use mg_area::terrain::{PaletteItem, TilesetPalette};
use mg_core::ResRef;
use mg_resman::GameInstall;
use mg_rules::GameData;
use mg_tiles::TileIndex;

fn print(items: &[PaletteItem], depth: usize) {
    for item in items {
        match item {
            PaletteItem::Brush { label, brush } => {
                println!("{}{label}  {brush:?}", "  ".repeat(depth))
            }
            PaletteItem::Folder { label, items } => {
                println!("{}{label}/", "  ".repeat(depth));
                print(items, depth + 1);
            }
        }
    }
}

fn main() {
    let root = mg_testkit::nwn_root().expect("game");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    for name in std::env::args().skip(1) {
        let resref = ResRef::from_str(&name).unwrap();
        let set = mg_area::tileset(&game, resref).unwrap();
        let index = TileIndex::new(&set);
        println!("== {name}");
        print(&TilesetPalette::read(&game, resref, &set, &index).branches, 0);
    }
}
