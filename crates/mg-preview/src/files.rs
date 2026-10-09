//! The files the game loads to draw something: its models, their
//! walkmeshes, textures and materials, wherever in the load order they
//! are. For handing a placeable (or a door, an item, a creature) on to
//! whoever mends its model, without digging through the haks for them.

use std::collections::BTreeSet;

use mg_core::{ResRef, ResType};
use mg_image::mtr::Mtr;
use mg_image::txi::Txi;
use mg_mdl::{Model, NodeKind};
use mg_resman::{LayerClass, ResKey, ResMan, priority};
use mg_rules::GameData;

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

/// The resource a model names: cut to a resource's 16 characters, as the
/// game reads it.
fn named(name: &str) -> Option<ResRef> {
    let cut = name.char_indices().nth(16).map_or(name, |(i, _)| &name[..i]);
    ResRef::from_str(cut).ok().filter(|r| !r.is_empty())
}

struct Gathered<'a> {
    rm: &'a ResMan,
    out: Vec<ContentFile>,
    seen: BTreeSet<ResKey>,
}

impl Gathered<'_> {
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
        let Some(resref) = named(name) else { return };
        if name.eq_ignore_ascii_case("null") || depth > 4 {
            return;
        }
        if self.add(resref, ResType::MTR)
            && let Some(data) = self.data(resref, ResType::MTR)
        {
            let mtr = Mtr::parse(&data);
            for t in mtr.textures.iter().flatten() {
                if !t.eq_ignore_ascii_case(name) {
                    self.texture(t, depth + 1);
                }
            }
            for shader in [&mtr.shader_vs, &mtr.shader_fs, &mtr.shader_gs].into_iter().flatten() {
                if let Some(s) = named(shader) {
                    self.add(s, ResType::SHD);
                }
            }
        }
        if let Some((t, _)) = self.rm.texture(resref) {
            self.add(resref, t);
        }
        self.add(resref, ResType::PLT);
        if self.add(resref, ResType::TXI)
            && let Some(data) = self.data(resref, ResType::TXI)
        {
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
    /// `.dwk`), its textures, the models its emitters throw, and the
    /// model its animations come from.
    fn model(&mut self, name: &str, depth: u8) {
        let Some(resref) = named(name) else { return };
        if depth > 8 || self.seen.contains(&ResKey::new(resref, ResType::MDL)) {
            return;
        }
        if !self.add(resref, ResType::MDL) {
            return;
        }
        for t in [ResType::PWK, ResType::DWK] {
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
/// and what it names after it: those the load order has (a texture named
/// and found nowhere is not listed).
pub fn files(game: &GameData, preview: &Preview) -> Vec<ContentFile> {
    let mut g = Gathered { rm: &game.resman, out: Vec::new(), seen: BTreeSet::new() };
    for part in std::iter::once(&preview.base).chain(&preview.parts) {
        g.model(&part.model, 0);
        // (Textures put in place of the model's own, and what it mirrors.)
        for t in part.textures.values().chain(&part.env_map) {
            g.texture(mg_render::assets::split_colors(t).0, 0);
        }
    }
    g.out
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
        // No model, nothing.
        assert!(files(&game, &Preview::model("nothing")).is_empty());
    }
}
