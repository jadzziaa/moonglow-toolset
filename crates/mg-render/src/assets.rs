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

/// A texture name with PLT colours (layer order), for [`Assets::texture`]:
/// `name#c0,c1,…,c9`.
pub fn colored_name(name: &str, colors: [u8; 10]) -> String {
    let c: Vec<String> = colors.iter().map(u8::to_string).collect();
    format!("{name}#{}", c.join(","))
}

/// A texture name and the PLT colours it carries, if any.
pub fn split_colors(name: &str) -> (&str, Option<[u8; 10]>) {
    let Some((base, list)) = name.split_once('#') else { return (name, None) };
    let values: Vec<u8> = list.split(',').filter_map(|v| v.parse().ok()).collect();
    (base, <[u8; 10]>::try_from(values).ok())
}

/// The resource a model names: a name longer than a resource's 16
/// characters is cut to them, as the game reads it (a model's `bitmap
/// c_drgdeep_hed_new` finds `c_drgdeep_hed_ne`).
fn named(name: &str) -> Option<ResRef> {
    let cut = name.char_indices().nth(16).map_or(name, |(i, _)| &name[..i]);
    ResRef::from_str(cut).ok()
}

impl Assets for ResMan {
    fn texture(&self, name: &str) -> Option<LoadedTexture> {
        let (name, colors) = split_colors(name);
        let resref = named(name)?;
        let mtr = self.get(&ResKey::new(resref, ResType::MTR)).ok().map(|d| Mtr::parse(&d));
        let image = match mtr.as_ref().and_then(|m| m.textures[0].clone()) {
            Some(t) if !t.eq_ignore_ascii_case(name) => named(&t)?,
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
        let resref = named(name)?;
        self.get(&ResKey::new(resref, ResType::MTR)).ok().map(|d| Mtr::parse(&d))
    }

    fn txi(&self, name: &str) -> Option<Txi> {
        let resref = named(name)?;
        self.get(&ResKey::new(resref, ResType::TXI)).ok().map(|d| Txi::parse(&d))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_name_is_cut_to_a_resource_s_sixteen_characters() {
        let name = |n: &str| named(n).map(|r| r.to_string());
        assert_eq!(name("c_drgdeep_hed_new").as_deref(), Some("c_drgdeep_hed_ne"));
        assert_eq!(name("c_drgdeep_hed").as_deref(), Some("c_drgdeep_hed"));
        assert_eq!(name("sixteen_chars_ok").as_deref(), Some("sixteen_chars_ok"));
    }
}
