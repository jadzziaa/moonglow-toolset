//! Where textures come from: the resource manager, with the game's order.
//! A mesh's texture name may name an MTR (which wins and may point to other
//! textures), else a DDS or TGA (DDS first within each layer class), else a
//! PLT (coloured with the default palette rows).

use mg_core::{ResRef, ResType};
use mg_image::Texture;
use mg_image::mtr::Mtr;
use mg_image::plt::Plt;
use mg_image::txi::Txi;
use mg_resman::{LayerClass, ResKey, ResMan};

/// A texture with its settings.
#[derive(Debug, Clone)]
pub struct LoadedTexture {
    pub texture: Texture,
    pub txi: Txi,
}

/// A source of textures.
pub trait Assets {
    /// A texture by the name a model uses (lower case).
    fn texture(&self, name: &str) -> Option<LoadedTexture>;

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

impl Assets for ResMan {
    fn texture(&self, name: &str) -> Option<LoadedTexture> {
        let (name, colors) = split_colors(name);
        let resref = ResRef::cut(name)?;
        let mtr = self.get(&ResKey::new(resref, ResType::MTR)).ok().map(|d| Mtr::parse(&d));
        let image = match mtr.as_ref().and_then(|m| m.textures[0].clone()) {
            Some(t) if !t.eq_ignore_ascii_case(name) => ResRef::cut(&t)?,
            _ => resref,
        };
        let txi =
            self.get(&ResKey::new(image, ResType::TXI)).map(|d| Txi::parse(&d)).unwrap_or_default();
        let plt = || {
            let data = self.get(&ResKey::new(image, ResType::PLT)).ok()?;
            plt_colored(self, &Plt::read(&data).ok()?, colors.unwrap_or([0; 10]))
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

/// A PLT coloured with a colour per layer.
fn plt_colored(rm: &ResMan, plt: &Plt, colors: [u8; 10]) -> Option<Texture> {
    let palette = |name: &str| {
        let (t, data) = rm.texture(ResRef::from_str(name).ok()?)?;
        mg_image::read(t, &data).ok().map(|t| t.to_rgba())
    };
    Some(plt.colorize_by(palette, colors).into_texture(true))
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
}
