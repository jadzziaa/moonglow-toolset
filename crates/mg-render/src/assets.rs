//! Where textures come from: the resource manager, with the game's order.
//! A mesh's texture name may name an MTR (which wins and may point to other
//! textures), else a DDS or TGA (DDS first within each layer class), else a
//! PLT (coloured with the default palette rows).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use mg_core::{ResRef, ResType};
use mg_image::mtr::Mtr;
use mg_image::plt::Plt;
use mg_image::txi::Txi;
use mg_image::{Rgba, Texture};
use mg_resman::{LayerClass, ResKey, ResMan};

/// A texture with its settings.
#[derive(Debug, Clone)]
pub struct LoadedTexture {
    pub texture: Texture,
    pub txi: Txi,
}

/// The palettes PLT textures are colored with (`pal_skin01` …), as
/// pixels: kept for the textures of one load order (the renderer keeps
/// one while its textures stay, [`Assets::texture_kept`]), so that each is
/// read and decoded once, not ten times a PLT.
#[derive(Debug, Default)]
pub struct Palettes(Mutex<HashMap<&'static str, Option<Arc<Rgba>>>>);

impl Palettes {
    /// The palette `name`, as kept; else as `read` reads it (none: kept as
    /// none), kept from now on.
    fn get(
        &self,
        name: &'static str,
        read: impl FnOnce(&'static str) -> Option<Rgba>,
    ) -> Option<Arc<Rgba>> {
        if let Some(kept) = self.0.lock().ok().and_then(|k| k.get(name).cloned()) {
            return kept;
        }
        // (Read with the lock let go: two threads may read one palette
        // once each, and keep the same.)
        let palette = read(name).map(Arc::new);
        if let Ok(mut kept) = self.0.lock() {
            kept.insert(name, palette.clone());
        }
        palette
    }
}

/// A source of textures.
pub trait Assets {
    /// A texture by the name a model uses (lower case).
    fn texture(&self, name: &str) -> Option<LoadedTexture>;

    /// [`Assets::texture`], one of many read from this source while it
    /// stays as it is: what they share (the PLTs' palettes) is kept in
    /// `kept`. By default nothing is.
    fn texture_kept(&self, name: &str, kept: &Palettes) -> Option<LoadedTexture> {
        let _ = kept;
        self.texture(name)
    }

    /// Whether the texture `name` (plain: no colors in it) is drawn in an
    /// instance's PLT colors: a PLT. One that is not (a part whose texture
    /// is a plain DDS or TGA: custom heads commonly are) is the same
    /// pixels whatever the colors, and is kept once under its plain name.
    /// By default every texture is taken to be.
    fn takes_colors(&self, name: &str) -> bool {
        let _ = name;
        true
    }

    /// An MTR by name (lower case).
    fn material(&self, name: &str) -> Option<Mtr> {
        let _ = name;
        None
    }

    /// A TXI by name (lower case), also where no image has the name (a
    /// cube map's faces are `name0`…`name5`).
    fn txi(&self, name: &str) -> Option<Txi> {
        let _ = name;
        None
    }

    /// A shader's source by name (lower case): only read for what it
    /// switches off, never run.
    fn shader(&self, name: &str) -> Option<String> {
        let _ = name;
        None
    }

    /// The game's own copy of a shader (its install's, whatever the load
    /// order puts over it: `override/`, `development/`, a hak): what the
    /// lit check ([`crate::probe::probe_lit`]) draws with, as what the
    /// stock programs draw is known. By default the load order's.
    fn game_shader(&self, name: &str) -> Option<String> {
        self.shader(name)
    }

    /// Where the load order's copy of a shader comes from when it isn't
    /// the game's own (the layer: `override`, `development`, `hak:…`);
    /// `None` for the game's own, or where it isn't known.
    fn shader_origin(&self, name: &str) -> Option<String> {
        let _ = name;
        None
    }
}

// (PLT colours in a texture name: mg-image's, as blueprint previews read
// them without a renderer.)
pub use mg_image::plt::{colored_name, split_colors};

/// The MTR a texture name names, if any, and the image it is read from
/// (the MTR's first texture where it names another, else the name's own).
fn image_of(rm: &ResMan, name: &str) -> Option<(Option<Mtr>, ResRef)> {
    let resref = ResRef::cut(name)?;
    let mtr = rm.get(&ResKey::new(resref, ResType::MTR)).ok().map(|d| Mtr::parse(&d));
    let image = match mtr.as_ref().and_then(|m| m.textures[0].clone()) {
        Some(t) if !t.eq_ignore_ascii_case(name) => ResRef::cut(&t)?,
        _ => resref,
    };
    Some((mtr, image))
}

impl Assets for ResMan {
    fn texture(&self, name: &str) -> Option<LoadedTexture> {
        self.texture_kept(name, &Palettes::default())
    }

    fn texture_kept(&self, name: &str, kept: &Palettes) -> Option<LoadedTexture> {
        let (name, colors) = split_colors(name);
        let (mtr, image) = image_of(self, name)?;
        let txi =
            self.get(&ResKey::new(image, ResType::TXI)).map(|d| Txi::parse(&d)).unwrap_or_default();
        let plt = || {
            let data = self.get(&ResKey::new(image, ResType::PLT)).ok()?;
            let read = |palette: &'static str| read_palette(self, palette);
            Some(plt_colored(&Plt::read(&data).ok()?, colors.unwrap_or([0; 10]), kept, read))
        };
        let plain = || {
            let (t, data) = self.texture(image)?;
            mg_image::read(t, &data).ok()
        };
        // Asked for in colours (a creature's or an item's part), the PLT,
        // as the game colours it: some parts have a plain grey TGA of the
        // same name too (a dwarf's head, pmd0_head001). Otherwise, and
        // through an MTR, the DDS or TGA first.
        let texture = if colors.is_some() && mtr.is_none() {
            plt().or_else(plain)?
        } else {
            plain().or_else(plt)?
        };
        Some(LoadedTexture { texture, txi })
    }

    fn takes_colors(&self, name: &str) -> bool {
        // (As `texture_kept` reads it in colors: the PLT, but through an
        // MTR only where there is no DDS or TGA.)
        let Some((mtr, image)) = image_of(self, name) else { return false };
        self.contains(&ResKey::new(image, ResType::PLT))
            && (mtr.is_none() || self.texture_type(image).is_none())
    }

    fn material(&self, name: &str) -> Option<Mtr> {
        let resref = ResRef::cut(name)?;
        self.get(&ResKey::new(resref, ResType::MTR)).ok().map(|d| Mtr::parse(&d))
    }

    fn txi(&self, name: &str) -> Option<Txi> {
        let resref = ResRef::cut(name)?;
        self.get(&ResKey::new(resref, ResType::TXI)).ok().map(|d| Txi::parse(&d))
    }

    fn shader(&self, name: &str) -> Option<String> {
        let resref = ResRef::cut(name)?;
        let data = self.get(&ResKey::new(resref, ResType::SHD)).ok()?;
        Some(String::from_utf8_lossy(&data).into_owned())
    }

    fn game_shader(&self, name: &str) -> Option<String> {
        let key = ResKey::new(ResRef::cut(name)?, ResType::SHD);
        let layer = self
            .copies(&key)
            .into_iter()
            .find(|&i| self.layers().get(i).is_some_and(|l| l.class == LayerClass::Key))?;
        let data = self.get_in(layer, &key).ok()?;
        Some(String::from_utf8_lossy(&data).into_owned())
    }

    fn shader_origin(&self, name: &str) -> Option<String> {
        let key = ResKey::new(ResRef::cut(name)?, ResType::SHD);
        let layer = self.layers().get(self.find(&key)?)?;
        (layer.class != LayerClass::Key).then(|| layer.label.clone())
    }
}

/// A palette's image as pixels, read from the load order.
fn read_palette(rm: &ResMan, name: &str) -> Option<Rgba> {
    let (t, data) = rm.texture(ResRef::from_str(name).ok()?)?;
    mg_image::read(t, &data).ok().map(|t| t.to_rgba())
}

/// A PLT coloured with a colour per layer: the palettes of the layers it
/// uses, as `kept` has them, else as `read` reads them (kept from then
/// on).
fn plt_colored(
    plt: &Plt,
    colors: [u8; 10],
    kept: &Palettes,
    mut read: impl FnMut(&'static str) -> Option<Rgba>,
) -> Texture {
    let used = plt.layers();
    let palettes: [Option<Arc<Rgba>>; 10] = std::array::from_fn(|layer| {
        used[layer].then(|| kept.get(mg_image::plt::PALETTES[layer], &mut read)).flatten()
    });
    let refs: [Option<&Rgba>; 10] = std::array::from_fn(|i| palettes[i].as_deref());
    plt.colorize(&refs, colors).into_texture(true)
}

/// No textures (tests, untextured previews).
#[derive(Debug, Default)]
pub struct NoAssets;

impl Assets for NoAssets {
    fn texture(&self, _name: &str) -> Option<LoadedTexture> {
        None
    }
}

#[cfg(test)]
mod tests {
    use mg_resman::{MemContainer, priority};

    use super::*;

    /// A shader the load order replaces (`override/` over the keys): the
    /// load order's copy, the game's own, and where the replacement is;
    /// one the game alone has, and one only `override/` has.
    #[test]
    fn a_shader_s_game_copy_and_where_the_load_order_s_comes_from() {
        let key = |n: &str| ResKey::new(ResRef::from_str(n).unwrap(), ResType::SHD);
        let mut game = MemContainer::new();
        game.insert(key("inc_lighting"), b"game's".to_vec());
        game.insert(key("fslit"), b"game's fslit".to_vec());
        let mut over = MemContainer::new();
        over.insert(key("inc_lighting"), b"builder's".to_vec());
        over.insert(key("fhss_skintile"), b"builder's own".to_vec());
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "key:nwn_base", LayerClass::Key, game);
        rm.add(priority::OVERRIDE, "override", LayerClass::Directory, over);
        assert_eq!(rm.shader("inc_lighting").as_deref(), Some("builder's"));
        assert_eq!(rm.game_shader("inc_lighting").as_deref(), Some("game's"));
        assert_eq!(rm.shader_origin("inc_lighting").as_deref(), Some("override"));
        assert_eq!(rm.game_shader("fslit").as_deref(), Some("game's fslit"));
        assert_eq!(rm.shader_origin("fslit"), None);
        assert_eq!(rm.game_shader("fhss_skintile"), None);
        assert_eq!(rm.shader_origin("fhss_skintile").as_deref(), Some("override"));
        assert_eq!(rm.shader_origin("nowhere"), None);
    }

    /// A PLT of `layers`' pixels, one a layer, gray 0.
    fn plt_of(layers: &[u8]) -> Vec<u8> {
        let mut f = b"PLT V1  ".to_vec();
        for v in [10u32, 0, layers.len() as u32, 1] {
            f.extend_from_slice(&v.to_le_bytes());
        }
        f.extend(layers.iter().flat_map(|&l| [0, l]));
        f
    }

    /// A palette of two rows: `top` (color 0) and black.
    fn palette_of(top: [u8; 4]) -> Rgba {
        let mut pal = Rgba::new(4, 2);
        for x in 0..4 {
            pal.data[(4 + x) * 4..(4 + x) * 4 + 4].copy_from_slice(&top);
        }
        pal
    }

    /// The palettes are read once for the PLTs of one load order, and only
    /// those of the layers a PLT uses: a second PLT of the same layers
    /// asks for none (each PLT read and decoded all ten, six files, again:
    /// fifty NPCs of twenty parts came to ten thousand reads).
    #[test]
    fn a_second_plt_asks_for_no_palette() {
        let kept = Palettes::default();
        let mut asked: Vec<&'static str> = Vec::new();
        let mut read = |name: &'static str| {
            asked.push(name);
            match name {
                "pal_skin01" => Some(palette_of([200, 150, 100, 255])),
                "pal_armor01" => Some(palette_of([10, 20, 30, 255])),
                _ => None,
            }
        };
        let colored =
            |layers: &[u8], read: &mut dyn FnMut(&'static str) -> Option<Rgba>| -> Vec<u8> {
                let plt = Plt::read(&plt_of(layers)).unwrap();
                plt_colored(&plt, [0; 10], &kept, read).to_rgba().data
            };
        // Skin and metal 1.
        let first = colored(&[0, 2], &mut read);
        assert_eq!(first, [200, 150, 100, 255, 10, 20, 30, 255]);
        // Skin, and metal 2 (metal 1's palette): no palette read.
        let second = colored(&[0, 3], &mut read);
        assert_eq!(second, first);
        // Hair (a palette not found, gray, asked for once) with them.
        let third = colored(&[1, 0], &mut read);
        assert_eq!(third, [0, 0, 0, 255, 200, 150, 100, 255]);
        let _ = colored(&[1, 2, 3, 0], &mut read);
        assert_eq!(asked, ["pal_skin01", "pal_armor01", "pal_hair01"]);
    }

    /// A texture is drawn in an instance's colors where it is a PLT: not a
    /// plain DDS or TGA (the same pixels whatever the colors), nor one
    /// read through an MTR that has a TGA; and the load order's `texture`
    /// colors with palettes it reads once a call.
    #[test]
    fn only_a_plt_takes_an_instance_s_colors() {
        let key = |n: &str, t| ResKey::new(ResRef::from_str(n).unwrap(), t);
        let tga = |rgba: Rgba| mg_image::write_tga(&rgba);
        let mut game = MemContainer::new();
        game.insert(key("pmh0_head001", ResType::PLT), plt_of(&[0, 0]));
        game.insert(key("pal_skin01", ResType::TGA), tga(palette_of([200, 150, 100, 255])));
        game.insert(key("custom_head", ResType::TGA), tga(palette_of([1, 2, 3, 255])));
        // (A part with a PLT and a gray TGA of its name: the PLT, colored.)
        game.insert(key("pmd0_head001", ResType::PLT), plt_of(&[0]));
        game.insert(key("pmd0_head001", ResType::TGA), tga(palette_of([9, 9, 9, 255])));
        // (An MTR over a PLT that has a TGA too: the TGA.)
        game.insert(key("with_mtr", ResType::MTR), b"renderhint NormalAndSpecMapped\n".to_vec());
        game.insert(key("with_mtr", ResType::PLT), plt_of(&[0]));
        game.insert(key("with_mtr", ResType::TGA), tga(palette_of([9, 9, 9, 255])));
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "key:nwn_base", LayerClass::Key, game);
        assert!(rm.takes_colors("pmh0_head001") && rm.takes_colors("pmd0_head001"));
        assert!(!rm.takes_colors("custom_head") && !rm.takes_colors("with_mtr"));
        assert!(!rm.takes_colors("nowhere"));
        let colored = Assets::texture(&rm, &colored_name("pmh0_head001", [0; 10])).unwrap();
        assert_eq!(colored.texture.to_rgba().data, [200, 150, 100, 255, 200, 150, 100, 255]);
        // (What does not take them is the same texture with and without.)
        let plain = Assets::texture(&rm, "custom_head").unwrap().texture;
        let in_colors =
            Assets::texture(&rm, &colored_name("custom_head", [7; 10])).unwrap().texture;
        assert_eq!(plain.to_rgba(), in_colors.to_rgba());
    }
}
