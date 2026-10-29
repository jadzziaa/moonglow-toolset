//! The files the game loads to draw something: its models, their
//! walkmeshes, textures and materials, wherever in the load order they
//! are. For handing a placeable (or a door, an item, a creature) on to
//! whoever mends its model, without digging through the haks for them.

use std::collections::BTreeSet;

use mg_core::{ResRef, ResType};
use mg_gff::Struct;
use mg_image::mtr::Mtr;
use mg_image::txi::Txi;
use mg_mdl::{Model, NodeKind};
use mg_resman::{LayerClass, ResKey, ResMan, priority};
use mg_rules::{GameData, ObjectKind};

use crate::Preview;

/// One file of [`files`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentFile {
    pub key: ResKey,
    /// The layer it comes from (`hak:name`, `override`…).
    pub origin: String,
    /// Whether it is the game's own (its KEY archives and patches), not a
    /// hak's, the module's or a folder's.
    pub stock: bool,
}

/// The files the game loads to draw things, gathered over many models and
/// textures, each once (what [`files`] gathers for one preview): for
/// walking a whole module's models without reading one twice.
#[derive(Debug)]
pub struct Gatherer<'a> {
    rm: &'a ResMan,
    out: Vec<ContentFile>,
    seen: BTreeSet<ResKey>,
    /// Names given and found nowhere, each once, in the order met.
    missing: Vec<String>,
}

impl<'a> Gatherer<'a> {
    pub fn new(rm: &'a ResMan) -> Gatherer<'a> {
        Gatherer { rm, out: Vec::new(), seen: BTreeSet::new(), missing: Vec::new() }
    }

    /// What is gathered so far, in the order it was found.
    pub fn files(&self) -> &[ContentFile] {
        &self.out
    }

    /// The models and textures named and found nowhere in the load order
    /// (a texture: no MTR, image, PLT or TXI of its name), as
    /// `name.mdl` and `name` (a texture's type is the one the game would
    /// find), in the order met: what the game draws without, or not at
    /// all.
    pub fn missing(&self) -> &[String] {
        &self.missing
    }

    fn miss(&mut self, name: String) {
        if !self.missing.contains(&name) {
            self.missing.push(name);
        }
    }

    pub fn into_files(self) -> Vec<ContentFile> {
        self.out
    }

    /// A preview's models and textures, as [`files`] gathers them.
    pub fn preview(&mut self, preview: &Preview) {
        for part in std::iter::once(&preview.base).chain(&preview.parts) {
            self.model(&part.model, 0);
            // (Textures put in place of the model's own, and what it
            // mirrors.)
            for t in part.textures.values().chain(&part.env_map) {
                self.texture(mg_image::plt::split_colors(t).0, 0);
            }
        }
    }

    /// What the game draws an area with (`are`, its ARE fields; `git`, its
    /// GIT's, for what is placed in it): its tileset (`.set`), each tile's
    /// model with its walkmesh and minimap picture, the tileset's
    /// environment map and grass, the skybox's models and the loading
    /// screen; and every creature, door, placeable and item placed in it,
    /// as [`Gatherer::preview`] gathers a blueprint's (the items a
    /// creature wears read from the load order). `set` is the area's
    /// tileset where the caller has it read already (else it is read).
    pub fn area(
        &mut self,
        game: &GameData,
        are: &Struct,
        set: Option<&mg_set::Tileset>,
        git: Option<&Struct>,
    ) {
        let read = |g: &Gatherer<'_>, name: ResRef| {
            let data = g.data(name, ResType::SET)?;
            mg_set::Tileset::parse(&data, game.codepage()).ok()
        };
        let name = are.resref("Tileset").filter(|n| self.add(*n, ResType::SET));
        let owned = name.filter(|_| set.is_none()).and_then(|n| read(self, n));
        if let Some(set) = set.or(owned.as_ref()) {
            for t in [&set.general.env_map, &set.grass.texture].into_iter().flatten() {
                self.texture(t, 0);
            }
            for tile in are.list("Tile_List").unwrap_or_default() {
                let id = usize::try_from(tile.integer("Tile_ID").unwrap_or(-1)).ok();
                let Some(t) = id.and_then(|i| set.tiles.get(i)) else { continue };
                self.model(&t.model, 0);
                if let Some(map) = &t.image_map_2d {
                    self.texture(map, 0);
                }
            }
        }
        let cell = |table: &str, row: i64, column: &str| {
            let t = game.table(table).ok()?;
            let value = t.get(usize::try_from(row).ok()?, column)?;
            Some(value.to_string())
        };
        let sky = are.integer("SkyBox").unwrap_or(0);
        for c in ["DAWN", "DAY", "DUSK", "NIGHT"] {
            if let Some(m) = cell("skyboxes", sky, c).filter(|_| sky > 0) {
                self.model(&m, 0);
            }
        }
        let screen = are.integer("LoadScreenID").unwrap_or(0);
        if let Some(image) = cell("loadscreens", screen, "BMPResRef").filter(|_| screen > 0) {
            self.texture(&image, 0);
        }
        let Some(git) = git else { return };
        let items = |r: ResRef| -> Option<mg_gff::Gff> {
            mg_gff::Gff::read(&game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?).ok()
        };
        // (The game draws no waypoint, store or sound: those are Aurora's.)
        use ObjectKind as K;
        for kind in [K::Creature, K::Placeable, K::Door, K::Item] {
            for s in git.list(kind.list()).unwrap_or_default() {
                if let Some(Ok(p)) = crate::of_object(game, kind, s, &items, crate::Shown::Placed) {
                    self.preview(&p);
                }
            }
        }
    }

    /// A model by name and what it names.
    pub fn add_model(&mut self, name: &str) {
        self.model(name, 0);
    }

    /// A texture by the name something gives it, and what it names.
    pub fn add_texture(&mut self, name: &str) {
        self.texture(name, 0);
    }

    /// One resource the load order has (a tile's walkmesh, say); whether
    /// it has it.
    pub fn add_file(&mut self, resref: ResRef, restype: ResType) -> bool {
        self.add(resref, restype)
    }

    /// Adds a resource the load order has; whether it has it.
    fn add(&mut self, resref: ResRef, restype: ResType) -> bool {
        let key = ResKey::new(resref, restype);
        let Some(i) = self.rm.find(&key) else { return false };
        if self.seen.insert(key) {
            let layer = &self.rm.layers()[i];
            let stock = layer.class == LayerClass::Key || layer.priority <= priority::PATCH;
            self.out.push(ContentFile { key, origin: layer.label.clone(), stock });
        }
        true
    }

    fn data(&self, resref: ResRef, restype: ResType) -> Option<Vec<u8>> {
        self.rm.get(&ResKey::new(resref, restype)).ok().map(|d| d.into_owned())
    }

    /// A texture by the name a model, a material or a TXI gives it: its
    /// MTR and what that names, its image (as the game picks it, and a PLT
    /// of the name), its TXI and what that names.
    fn texture(&mut self, name: &str, depth: u8) {
        let Some(resref) = ResRef::cut(name) else { return };
        if name.eq_ignore_ascii_case("null") || depth > 4 {
            return;
        }
        let material = self.add(resref, ResType::MTR);
        if material && let Some(data) = self.data(resref, ResType::MTR) {
            let mtr = Mtr::parse(&data);
            for t in mtr.textures.iter().flatten() {
                if !t.eq_ignore_ascii_case(name) {
                    self.texture(t, depth + 1);
                }
            }
            for shader in [&mtr.shader_vs, &mtr.shader_fs, &mtr.shader_gs].into_iter().flatten() {
                if let Some(s) = ResRef::cut(shader) {
                    self.add(s, ResType::SHD);
                }
            }
        }
        let image = self.rm.texture_type(resref).is_some_and(|t| self.add(resref, t));
        let plt = self.add(resref, ResType::PLT);
        let txi = self.add(resref, ResType::TXI);
        if !(material || image || plt || txi) {
            self.miss(resref.to_string());
        }
        if txi && let Some(data) = self.data(resref, ResType::TXI) {
            for e in Txi::parse(&data).entries {
                if e.key.to_ascii_lowercase().ends_with("texture") {
                    for v in &e.values {
                        self.texture(v, depth + 1);
                    }
                }
            }
        }
    }

    /// A model: its file, its walkmeshes (a placeable's `.pwk`, a door's
    /// `.dwk`, a tile's `.wok`), its textures, the models its emitters
    /// throw, and the model its animations come from.
    fn model(&mut self, name: &str, depth: u8) {
        let Some(resref) = ResRef::cut(name) else { return };
        if depth > 8 || self.seen.contains(&ResKey::new(resref, ResType::MDL)) {
            return;
        }
        if !self.add(resref, ResType::MDL) {
            self.miss(ResKey::new(resref, ResType::MDL).to_string());
            return;
        }
        for t in [ResType::PWK, ResType::DWK, ResType::WOK] {
            self.add(resref, t);
        }
        let Some(model) = self.data(resref, ResType::MDL).and_then(|d| Model::read(&d).ok()) else {
            return;
        };
        for n in &model.nodes {
            match &n.kind {
                NodeKind::Mesh(m) => {
                    for t in m.textures.iter().flatten().chain(&m.material) {
                        self.texture(t, 0);
                    }
                }
                NodeKind::Emitter(e) => {
                    if let Some(t) = &e.texture {
                        self.texture(t, 0);
                    }
                    if let Some(chunk) = &e.chunk {
                        self.model(chunk, depth + 1);
                    }
                }
                NodeKind::Light(l) => {
                    for t in &l.flare_textures {
                        self.texture(t, 0);
                    }
                }
                _ => {}
            }
        }
        if let Some(sup) = model.supermodel.as_deref().filter(|s| !s.eq_ignore_ascii_case("null")) {
            self.model(sup, depth + 1);
        }
    }
}

/// The files the game loads to draw `preview`, each once, a model first
/// and what it names after it: those the load order has (what is named
/// and found nowhere is [`Gatherer::missing`]'s).
pub fn files(game: &GameData, preview: &Preview) -> Vec<ContentFile> {
    let mut g = Gatherer::new(&game.resman);
    g.preview(preview);
    g.out
}

/// The row of a table an object's look is chosen by (a placeable's of
/// placeables.2da, a door's of genericdoors.2da or doortypes.2da, a
/// creature's of appearance.2da, an item's of baseitems.2da), as a 2DA of
/// that one row: whoever mends a model needs the row that names it, and a
/// hak's table has thousands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookRow {
    /// The table's name (`placeables`).
    pub table: &'static str,
    pub row: usize,
    /// Where the game reads the table from (`hak:name`, `override`…).
    pub origin: String,
    /// The table's columns and that row, numbered as it is there: a 2DA
    /// to read or take the line from, not to put in a hak in place of the
    /// table.
    pub text: String,
}

impl LookRow {
    /// The file it is written as: `placeables.2da-row-12.txt`, which no
    /// hak takes for the table.
    pub fn file_name(&self) -> String {
        format!("{}.2da-row-{}.txt", self.table, self.row)
    }
}

/// The row `object` (a blueprint's fields, or a placed object's) of
/// `kind` has its look from, as the game reads the table; `None` for one
/// of no table's, or a row the table lacks.
pub fn look_row(game: &GameData, kind: ObjectKind, object: &Struct) -> Option<LookRow> {
    let at = mg_rules::appearance::AppearanceRow::of(kind, object)?;
    let row = usize::try_from(at.row).ok()?;
    let table = game.table(at.table).ok()?;
    if row >= table.len() {
        return None;
    }
    let key = ResKey::new(ResRef::from_str(at.table).ok()?, ResType::TWODA);
    let origin = game.resman.origin(&key).unwrap_or("the game").to_string();
    let cell = |c: Option<&str>| match c {
        None => "****".to_string(),
        Some(v) if v.is_empty() || v.contains([' ', '\t']) => format!("\"{v}\""),
        Some(v) => v.to_string(),
    };
    let columns = table.columns();
    let mut text = String::from("2DA V2.0\n\n");
    // (The columns' line begins with a space, as the row with its number.)
    for c in columns {
        text.push(' ');
        text += c;
    }
    text += &format!("\n{row}");
    for c in 0..columns.len() {
        text.push(' ');
        text += &cell(table.cell(row, c));
    }
    text.push('\n');
    Some(LookRow { table: at.table, row, origin, text })
}

#[cfg(test)]
mod tests {
    use mg_resman::MemContainer;

    use super::*;

    const MODEL: &str = "newmodel mg_fog\nsetsupermodel mg_fog NULL\nclassification Character\n\
        setanimationscale 1\nbeginmodelgeom mg_fog\nnode dummy mg_fog\n  parent NULL\nendnode\n\
        node trimesh box\n  parent mg_fog\n  bitmap mg_wood\n  verts 3\n    0 0 0\n    1 0 0\n    \
        0 1 0\n  faces 1\n    0 1 2 1 0 0 0 0\nendnode\n\
        node emitter mist\n  parent mg_fog\n  update Fountain\n  render Normal\n  blend Normal\n  \
        texture fxpa_smoke\n  chunkName mg_chunk\nendnode\n\
        endmodelgeom mg_fog\ndonemodel mg_fog\n";

    /// A hak's placeable: its model, walkmesh and the textures it names
    /// come from the hak, the model its emitter throws too; the game's own
    /// smoke is told apart, and a texture found nowhere is not listed.
    #[test]
    fn a_hak_s_model_brings_its_files() {
        let key = |name: &str| ResKey::from_filename(name).unwrap();
        let mut hak = MemContainer::new();
        hak.insert(key("mg_fog.mdl"), MODEL.as_bytes());
        hak.insert(key("mg_fog.pwk"), &b"pwk"[..]);
        hak.insert(key("mg_wood.mtr"), &b"texture0 mg_wood_d\ntexture1 mg_wood_n\n"[..]);
        hak.insert(key("mg_wood_d.dds"), &b"dds"[..]);
        hak.insert(key("mg_wood_d.txi"), &b"envmaptexture mg_sky\n"[..]);
        hak.insert(key("mg_sky.tga"), &b"tga"[..]);
        hak.insert(key("mg_chunk.mdl"), MODEL.replace("mg_fog", "mg_chunk").as_bytes());
        hak.insert(key("unrelated.tga"), &b"tga"[..]);
        let mut stock = MemContainer::new();
        stock.insert(key("fxpa_smoke.tga"), &b"tga"[..]);
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "hak:mg", LayerClass::Erf, hak);
        rm.add(priority::KEY, "nwn_base.key", LayerClass::Key, stock);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));

        let found = files(&game, &Preview::model("mg_fog"));
        let names: Vec<String> = found.iter().map(|f| f.key.to_string()).collect();
        assert_eq!(
            names,
            [
                "mg_fog.mdl",
                "mg_fog.pwk",
                "mg_wood.mtr",
                "mg_wood_d.dds",
                "mg_wood_d.txi",
                "mg_sky.tga",
                "fxpa_smoke.tga",
                "mg_chunk.mdl",
            ]
        );
        let stock: Vec<&ContentFile> = found.iter().filter(|f| f.stock).collect();
        assert_eq!(stock.len(), 1);
        assert_eq!(
            (stock[0].key, stock[0].origin.as_str()),
            (key("fxpa_smoke.tga"), "nwn_base.key")
        );
        assert!(found.iter().filter(|f| !f.stock).all(|f| f.origin == "hak:mg"));
        // What is named and found nowhere is said: the material's normal
        // map, once though the chunk's model names it again.
        let mut g = Gatherer::new(&game.resman);
        g.preview(&Preview::model("mg_fog"));
        assert_eq!(g.missing(), ["mg_wood_n"]);
        // No model, nothing; and the model is found nowhere.
        assert!(files(&game, &Preview::model("nothing")).is_empty());
        let mut g = Gatherer::new(&game.resman);
        g.preview(&Preview::model("nothing"));
        assert_eq!(g.missing(), ["nothing.mdl"]);
    }

    /// An area's files: its tileset, its tiles' models with their
    /// walkmeshes and the textures they name, once however many tiles
    /// are of one model; a tile model found nowhere is said.
    #[test]
    fn an_area_brings_its_tileset_and_tiles() {
        let key = |name: &str| ResKey::from_filename(name).unwrap();
        let tile = |name: &str| MODEL.replace("mg_fog", name).replace("mg_chunk", "null");
        let mut hak = MemContainer::new();
        let set = "[GENERAL]\nName=TST01\nEnvMap=mg_sky\n[TILES]\nCount=2\n[TILE0]\n\
                   Model=tst01_a01\nTopLeft=Grass\n[TILE1]\nModel=tst01_gone\nTopLeft=Grass\n";
        hak.insert(key("tst01.set"), set.as_bytes());
        hak.insert(key("tst01_a01.mdl"), tile("tst01_a01").as_bytes());
        hak.insert(key("tst01_a01.wok"), &b"wok"[..]);
        hak.insert(key("mg_wood.tga"), &b"tga"[..]);
        hak.insert(key("mg_sky.tga"), &b"tga"[..]);
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "hak:tiles", LayerClass::Erf, hak);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut are = Struct::default();
        are.set("Tileset", mg_gff::Value::resref(ResRef::from_str("tst01").unwrap()));
        let tiles = [0, 0, 1].map(|id| {
            let mut t = Struct::default();
            t.set("Tile_ID", mg_gff::Value::Int(id));
            t
        });
        are.set("Tile_List", mg_gff::Value::List(tiles.to_vec()));
        let mut g = Gatherer::new(&game.resman);
        g.area(&game, &are, None, None);
        let names: Vec<String> = g.files().iter().map(|f| f.key.to_string()).collect();
        assert_eq!(
            names,
            ["tst01.set", "mg_sky.tga", "tst01_a01.mdl", "tst01_a01.wok", "mg_wood.tga"]
        );
        assert!(g.missing().contains(&"tst01_gone.mdl".to_string()), "{:?}", g.missing());
    }

    /// A placeable's row of placeables.2da, as a 2DA of that row numbered
    /// as it is in the table (from the hak that has the table), its empty
    /// cells as the table writes them; none for a row the table lacks.
    #[test]
    fn a_placeable_s_row_is_written_as_a_2da_of_it() {
        let key = |name: &str| ResKey::from_filename(name).unwrap();
        let mut hak = MemContainer::new();
        let table = "2DA V2.0\n\n   Label   ModelName  LightColor\n0  Chest   plc_c01    ****\n\
                     1  \"My Fog\"  mg_fog  3\n";
        hak.insert(key("placeables.2da"), table.as_bytes());
        let mut rm = ResMan::new();
        rm.add(priority::HAK_USER, "hak:mg", LayerClass::Erf, hak);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut utp = Struct::default();
        utp.set("Appearance", mg_gff::Value::Dword(1));
        let row = look_row(&game, ObjectKind::Placeable, &utp).unwrap();
        assert_eq!((row.table, row.row, row.origin.as_str()), ("placeables", 1, "hak:mg"));
        assert_eq!(row.file_name(), "placeables.2da-row-1.txt");
        assert_eq!(row.text, "2DA V2.0\n\n Label ModelName LightColor\n1 \"My Fog\" mg_fog 3\n");
        let back = mg_2da::TwoDa::parse(row.text.as_bytes(), mg_core::Codepage::WINDOWS_1252);
        assert_eq!(back.unwrap().get(0, "ModelName"), Some("mg_fog"));
        utp.set("Appearance", mg_gff::Value::Dword(0));
        let chest = look_row(&game, ObjectKind::Placeable, &utp).unwrap();
        assert!(chest.text.ends_with("\n0 Chest plc_c01 ****\n"), "{}", chest.text);
        utp.set("Appearance", mg_gff::Value::Dword(7));
        assert_eq!(look_row(&game, ObjectKind::Placeable, &utp), None);
    }
}
