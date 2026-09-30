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
}

impl Assets for ResMan {
    fn texture(&self, name: &str) -> Option<LoadedTexture> {
        let resref = ResRef::from_str(name).ok()?;
        let mtr = self.get(&ResKey::new(resref, ResType::MTR)).ok().map(|d| Mtr::parse(&d));
        let image = match mtr.as_ref().and_then(|m| m.textures[0].clone()) {
            Some(t) if !t.eq_ignore_ascii_case(name) => ResRef::from_str(&t).ok()?,
            _ => resref,
        };
        let txi =
            self.get(&ResKey::new(image, ResType::TXI)).map(|d| Txi::parse(&d)).unwrap_or_default();
        let texture = match self.texture(image) {
            Some((t, data)) => mg_image::read(t, &data).ok()?,
            None => {
                let data = self.get(&ResKey::new(image, ResType::PLT)).ok()?;
                plt_default(self, &Plt::read(&data).ok()?)?
            }
        };
        Some(LoadedTexture { texture, txi, mtr })
    }

    fn material(&self, name: &str) -> Option<Mtr> {
        let resref = ResRef::from_str(name).ok()?;
        self.get(&ResKey::new(resref, ResType::MTR)).ok().map(|d| Mtr::parse(&d))
    }

    fn txi(&self, name: &str) -> Option<Txi> {
        let resref = ResRef::from_str(name).ok()?;
        self.get(&ResKey::new(resref, ResType::TXI)).ok().map(|d| Txi::parse(&d))
    }
}

/// A PLT coloured with colour 0 of every layer.
fn plt_default(rm: &ResMan, plt: &Plt) -> Option<Texture> {
    let palettes: Vec<Option<Rgba>> = PALETTES
        .iter()
        .map(|p| {
            let (t, data) = rm.texture(ResRef::from_str(p).ok()?)?;
            mg_image::read(t, &data).ok().map(|t| t.to_rgba())
        })
        .collect();
    let refs: [Option<&Rgba>; 10] = std::array::from_fn(|i| palettes[i].as_ref());
    Some(plt.colorize(&refs, [0; 10]).into_texture(true))
}

/// No textures (tests, untextured previews).
#[derive(Debug, Default)]
pub struct NoAssets;

impl Assets for NoAssets {
    fn texture(&self, _name: &str) -> Option<LoadedTexture> {
        None
    }
}
