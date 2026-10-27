//! Where textures come from: the resource manager, with the game's order.
//! A mesh's texture name may name an MTR (which wins and may point to other
//! textures), else a DDS or TGA (DDS first within each layer class), else a
//! PLT (coloured with the default palette rows).

use mg_core::{ResRef, ResType};
use mg_image::mtr::Mtr;
use mg_image::plt::{PALETTES, Plt};
use mg_image::txi::Txi;
use mg_image::{Rgba, Texture};
use mg_resman::{ResKey, ResMan};

/// A texture with its settings.
#[derive(Debug, Clone)]
pub struct LoadedTexture {
    pub texture: Texture,
    pub txi: Txi,
    /// The MTR the name resolved through, if any.
    pub mtr: Option<Mtr>,
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
        Some(LoadedTexture { texture, txi, mtr })
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
}

/// A PLT coloured with a colour per layer.
fn plt_colored(rm: &ResMan, plt: &Plt, colors: [u8; 10]) -> Option<Texture> {
    let palettes: Vec<Option<Rgba>> = PALETTES
        .iter()
        .map(|p| {
            let (t, data) = rm.texture(ResRef::from_str(p).ok()?)?;
            mg_image::read(t, &data).ok().map(|t| t.to_rgba())
        })
        .collect();
    let refs: [Option<&Rgba>; 10] = std::array::from_fn(|i| palettes[i].as_ref());
    Some(plt.colorize(&refs, colors).into_texture(true))
}

/// No textures (tests, untextured previews).
#[derive(Debug, Default)]
pub struct NoAssets;

impl Assets for NoAssets {
    fn texture(&self, _name: &str) -> Option<LoadedTexture> {
        None
    }
}
